//! Server API communication module for echelon-server.
//!
//! Handles uploading replays, checking status, and downloading finished videos.

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::OnceLock;
use std::time::Duration;
use serde_json::json;

/// Cached server URL, initialized once on first access.
static SERVER_URL: OnceLock<String> = OnceLock::new();

/// Represents the status of a replay processing job from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ReplayStatus {
    #[serde(rename = "queued")]
    Queued {
        position: u32,
        #[serde(default)]
        estimate_minutes: u32,
    },
    #[serde(rename = "processing")]
    Processing { estimate_minutes: u32 },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "not_found")]
    NotFound { message: String },
}

/// Creates an HTTP client with appropriate timeouts.
/// Connection timeout: 30s, Total request timeout: 90s
/// This prevents hanging indefinitely on slow or unresponsive connections.
fn create_http_client() -> HttpClient {
    HttpClient::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(90))
        .build()
        .expect("Failed to build HTTP client")
}

/// Creates a replay job with default configuration (no customization for Discord bot).
/// Returns the unique ID assigned to the replay.
pub async fn create_replay(server_url: &str) -> Result<String, String> {
    let client = create_http_client();
    let create_url = format!("{}/create", server_url);

    let mut request = client.post(&create_url).header("Content-Type", "application/json");

    // Add bot secret token if configured (to bypass rate limiting)
    if let Ok(bot_secret) = env::var("BOT_SECRET") {
        if !bot_secret.is_empty() {
            request = request.header("X-Bot-Secret", bot_secret);
        }
    }

    // Use default configuration
    let body = json!({
        "top_down_view": false,
        "swap_players": false,
        "game_speed": 1.0
    });

    let response = request
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Server returned {}", response.status()));
    }

    let id = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?
        .trim()
        .to_string();

    Ok(id)
}

/// Uploads a replay file to the echelon server for a given task ID.
pub async fn upload_replay(server_url: &str, task_id: &str, data: &[u8]) -> Result<(), String> {
    let client = create_http_client();
    let upload_url = format!("{}/upload?task_id={}", server_url, task_id);

    let mut request = client
        .post(&upload_url)
        .header("Content-Type", "application/octet-stream");

    // Add bot secret token if configured (to bypass rate limiting)
    if let Ok(bot_secret) = env::var("BOT_SECRET") {
        if !bot_secret.is_empty() {
            request = request.header("X-Bot-Secret", bot_secret);
        }
    }

    let response = request
        .body(data.to_vec())
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Server returned {}", response.status()));
    }

    Ok(())
}

/// Fetches the current status of a replay from the server.
pub async fn get_replay_status(server_url: &str, id: &str) -> Result<ReplayStatus, String> {
    let client = create_http_client();
    let status_url = format!("{}/status/{}", server_url, id);

    let response = client
        .get(&status_url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Server returned {}", response.status()));
    }

    let status = response
        .json::<ReplayStatus>()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    Ok(status)
}

/// Downloads a finished replay video from the server.
pub async fn download_video(server_url: &str, id: &str) -> Result<Vec<u8>, String> {
    let client = create_http_client();
    let download_url = format!("{}/download/{}", server_url, id);

    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Server returned {}", response.status()));
    }

    let data = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to download video: {e}"))?
        .to_vec();

    Ok(data)
}

/// Gets the echelon server URL, cached after first read.
/// Panics if `ECHELON_SERVER_URL` is not set. Call [`validate_server_url`] at startup
/// to fail fast instead of on the first user command.
pub fn get_server_url() -> &'static str {
    SERVER_URL.get_or_init(|| {
        env::var("ECHELON_SERVER_URL").expect("ECHELON_SERVER_URL must be set")
    })
}

/// Validates that `ECHELON_SERVER_URL` is set. Call at startup to fail fast.
pub fn validate_server_url() {
    let _ = get_server_url();
}
