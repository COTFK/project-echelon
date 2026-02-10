mod commands;
mod estimation;
mod routes;
mod types;
mod worker;

use crate::routes::download;
use crate::routes::status;
use crate::routes::upload;
use crate::types::Replay;
use crate::worker::cleanup;
use crate::worker::worker;
use axum::Router;
use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::http::{Method, Request, Response, StatusCode};
use axum::middleware::{self, Next};
use axum::routing::get;
use axum::routing::post;
use commands::start_xvfb;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::KeyExtractor;
use tower_http::cors::{Any, CorsLayer};
use ulid::Ulid;

#[tokio::main]
async fn main() {
    // Start logging
    tracing_subscriber::fmt::init();
    tracing::info!("Starting server...");

    // Load environment variables - used for various config purposes
    _ = dotenvy::dotenv();

    // Start xvfb - we use it to avoid dependencies on host hardware having a GPU/display device
    let Ok(xvfb_process) = start_xvfb().await else {
        tracing::error!("Unable to start Xvfb - check that you have it installed on your system.");
        return;
    };

    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));

    // Start background worker
    tracing::info!("Starting background worker task...");
    tokio::spawn(worker(Arc::clone(&state)));

    // Start cleanup task
    tracing::info!("Starting cleanup task...");
    tokio::spawn(cleanup(Arc::clone(&state)));

    // Start server
    let app = create_app(state);

    let listener = match tokio::net::TcpListener::bind("0.0.0.0:3000").await {
        Ok(listener) => {
            tracing::info!("Server listening on http://0.0.0.0:3000");
            listener
        }
        Err(e) => {
            tracing::error!("Unable to create listener on 0.0.0.0:3000: {}", e);
            return;
        }
    };

    tracing::info!("Server ready to accept connections.");

    // Use into_make_service_with_connect_info to allow rate limiter to extract client IP
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(close_gracefully(xvfb_process))
    .await
    .expect("Axum server should never raise an error");
}

/// Gracefully close the Xvfb process on SIGINT or SIGTERM.
async fn close_gracefully(mut xvfb_process: tokio::process::Child) {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to setup SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM handler");

    tokio::select! {
        _ = sigint.recv() => {
            tracing::info!("Received SIGINT, shutting down...");
            if let Err(e) = xvfb_process.kill().await {
                tracing::error!("Failed to kill Xvfb process: {}", e);
            }
        }
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM, shutting down...");
            if let Err(e) = xvfb_process.kill().await {
                tracing::error!("Failed to kill Xvfb process: {}", e);
            }
        }
    }
}

/// Middleware to log rate limit hits (429 responses)
async fn log_rate_limit_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Extract client IP before processing
    let client_ip = extract_client_ip(&req);
    
    let response = next.run(req).await;
    
    // Log if rate limit was hit
    if response.status() == StatusCode::TOO_MANY_REQUESTS {
        tracing::warn!("Rate limit exceeded for IP: {}", client_ip);
    }
    
    response
}

/// Helper function to extract client IP from request (same logic as RealIpKeyExtractor)
fn extract_client_ip(req: &Request<Body>) -> String {
    // Try X-Forwarded-For header first
    if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
        if let Ok(value) = forwarded_for.to_str() {
            let client_ip = value.split(',').next().unwrap_or(value).trim();
            if !client_ip.is_empty() {
                return client_ip.to_string();
            }
        }
    }
    
    // Try X-Real-IP header as fallback
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            let client_ip = value.trim();
            if !client_ip.is_empty() {
                return client_ip.to_string();
            }
        }
    }
    
    // Use peer address from connection info
    if let Some(addr) = req.extensions().get::<SocketAddr>() {
        return addr.ip().to_string();
    }
    
    "unknown".to_string()
}

/// Custom key extractor that reads client IP from X-Forwarded-For header (for reverse proxy)
/// or falls back to peer address if header is not present.
#[derive(Clone, Copy, Debug)]
struct RealIpKeyExtractor;

impl KeyExtractor for RealIpKeyExtractor {
    type Key = String;

    fn extract<T>(&self, req: &axum::http::Request<T>) -> Result<Self::Key, tower_governor::GovernorError> {
        // Try X-Forwarded-For header first (CapRover/nginx standard)
        if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
            if let Ok(value) = forwarded_for.to_str() {
                // X-Forwarded-For can be "client, proxy1, proxy2", we want the first one
                let client_ip = value.split(',').next().unwrap_or(value).trim();
                if !client_ip.is_empty() {
                    return Ok(client_ip.to_string());
                }
            }
        }

        // Try X-Real-IP header as fallback
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(value) = real_ip.to_str() {
                let client_ip = value.trim();
                if !client_ip.is_empty() {
                    return Ok(client_ip.to_string());
                }
            }
        }

        // Ultimate fallback: use peer address from connection info
        if let Some(addr) = req.extensions().get::<SocketAddr>() {
            return Ok(addr.ip().to_string());
        }

        // If all else fails, use a default key (shouldn't happen in practice)
        tracing::warn!("Could not extract client IP for rate limiting, using default");
        Ok("unknown".to_string())
    }
}

/// Creates the application router with all routes and middleware.
fn create_app(state: Arc<RwLock<BTreeMap<Ulid, Replay>>>) -> Router {
    // Configure rate limiting: 5 uploads per 60 seconds per IP
    // burst_size(6) allows 5 immediate requests (GCRA uses burst_size - 1 for initial burst)
    // Uses custom key extractor to read real client IP from X-Forwarded-For header (reverse proxy)
    tracing::debug!("Configuring rate limiter: 5 requests per 60 seconds per IP");
    let rate_limit_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(60)
            .burst_size(6)
            .use_headers()
            .key_extractor(RealIpKeyExtractor)
            .finish()
            .expect("Failed to build rate limiter config"),
    );

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    // Rate-limited upload route (nested router so rate limit only applies here)
    let upload_router = Router::new()
        .route("/upload", post(upload))
        .layer(GovernorLayer::new(rate_limit_config))
        .layer(middleware::from_fn(log_rate_limit_middleware));

    // Non-rate-limited routes
    Router::new()
        .merge(upload_router)
        .route("/health", get(health))
        .route("/status/{id}", get(status))
        .route("/download/{id}", get(download))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10 MB limit
        .layer(cors)
        .with_state(state)
}

/// Health check endpoint for load balancers and container orchestration.
async fn health() -> &'static str {
    "OK"
}
