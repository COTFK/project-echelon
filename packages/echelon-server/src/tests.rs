use crate::routes::{download, status, upload};
use crate::types::Replay;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum_test::TestServer;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_governor::governor::GovernorConfigBuilder;
use ulid::Ulid;

/// Valid yrpX file header for testing
fn valid_replay_data() -> Vec<u8> {
    let mut data = b"yrpX".to_vec();
    data.extend_from_slice(&[0u8; 100]); // Pad with zeros
    data
}

/// Invalid file data (not a yrpX file)
fn invalid_replay_data() -> Vec<u8> {
    b"not a replay file".to_vec()
}

/// Health check endpoint for testing
async fn health() -> &'static str {
    "OK"
}

/// Creates a test app without rate limiting for easier testing
fn create_app_without_rate_limit(state: Arc<RwLock<BTreeMap<Ulid, Replay>>>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/upload", post(upload))
        .route("/status/{id}", get(status))
        .route("/download/{id}", get(download))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .with_state(state)
}

#[tokio::test]
async fn test_health_endpoint() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/health").await;
    response.assert_status_ok();
    assert_eq!(response.text(), "OK");
}

#[tokio::test]
async fn test_upload_valid_replay() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/upload")
        .bytes(valid_replay_data().into())
        .await;

    response.assert_status_ok();
    // Response should be a valid ULID
    let body = response.text();
    assert!(!body.is_empty());
    assert!(body.parse::<Ulid>().is_ok());
}

#[tokio::test]
async fn test_upload_invalid_replay() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/upload")
        .bytes(invalid_replay_data().into())
        .await;

    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_status_for_queued_job() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    // Upload a replay
    let upload_response = server
        .post("/upload")
        .bytes(valid_replay_data().into())
        .await;

    upload_response.assert_status_ok();
    let job_id = upload_response.text();

    // Check status - should be queued
    let status_response = server.get(&format!("/status/{job_id}")).await;

    status_response.assert_status_ok();
    let body = status_response.text();
    assert!(body.contains("\"status\":\"queued\""));
    assert!(body.contains("\"position\":1"));
}

#[tokio::test]
async fn test_status_not_found() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let fake_id = Ulid::new();
    let response = server.get(&format!("/status/{fake_id}")).await;

    response.assert_status_not_found();
    let body = response.text();
    assert!(body.contains("\"status\":\"not_found\""));
}

#[tokio::test]
async fn test_download_not_found() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let fake_id = Ulid::new();
    let response = server.get(&format!("/download/{fake_id}")).await;

    response.assert_status_not_found();
}

#[tokio::test]
async fn test_queue_limit() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    // Fill up the queue (MAX_QUEUE_SIZE = 100)
    for i in 0..100 {
        let response = server
            .post("/upload")
            .bytes(valid_replay_data().into())
            .await;
        assert!(
            response.status_code() == StatusCode::OK,
            "Upload {} should succeed",
            i + 1
        );
    }

    // 101st upload should fail with SERVICE_UNAVAILABLE
    let response = server
        .post("/upload")
        .bytes(valid_replay_data().into())
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::SERVICE_UNAVAILABLE,
        "Upload after queue is full should return 503"
    );
}

#[tokio::test]
async fn test_rate_limit_config_is_valid() {
    // Test that the rate limit config can be built successfully
    let config = GovernorConfigBuilder::default()
        .per_second(60)
        .burst_size(5)
        .finish();

    assert!(config.is_some(), "Rate limit config should be valid");
}

#[tokio::test]
async fn test_multiple_uploads_get_unique_ids() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let mut ids = Vec::new();
    for _ in 0..5 {
        let response = server
            .post("/upload")
            .bytes(valid_replay_data().into())
            .await;
        response.assert_status_ok();
        ids.push(response.text());
    }

    // All IDs should be unique
    let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(unique_count, 5, "All upload IDs should be unique");
}

#[tokio::test]
async fn test_queue_positions_are_sequential() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    // Upload 3 replays
    let mut ids = Vec::new();
    for _ in 0..3 {
        let response = server
            .post("/upload")
            .bytes(valid_replay_data().into())
            .await;
        ids.push(response.text());
    }

    // Check that each job has a valid position (1, 2, or 3)
    let mut positions = Vec::new();
    for id in &ids {
        let response = server.get(&format!("/status/{id}")).await;
        let body = response.text();

        // Extract position from JSON
        if let Some(pos_start) = body.find("\"position\":") {
            let pos_str = &body[pos_start + 11..];
            if let Some(pos_end) = pos_str.find('}') {
                if let Ok(pos) = pos_str[..pos_end].parse::<usize>() {
                    positions.push(pos);
                }
            }
        }
    }

    // All positions should be present (1, 2, 3)
    positions.sort();
    assert_eq!(positions, vec![1, 2, 3], "Positions should be 1, 2, 3");
}

// =============================================================================
// Status endpoint tests for different job states
// =============================================================================

#[tokio::test]
async fn test_status_for_processing_job() {
    use crate::types::ReplayStatus;

    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Manually insert a job in Recording (processing) state
    {
        let mut lock = state.write().await;
        let mut job = Replay::new(valid_replay_data().into());
        job.status = ReplayStatus::Recording;
        lock.insert(id, job);
    }

    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get(&format!("/status/{id}")).await;
    response.assert_status_ok();
    let body = response.text();
    assert!(body.contains("\"status\":\"processing\""));
}

#[tokio::test]
async fn test_status_for_done_job() {
    use crate::types::ReplayStatus;

    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Manually insert a completed job
    {
        let mut lock = state.write().await;
        let mut job = Replay::new(valid_replay_data().into());
        job.status = ReplayStatus::Done;
        job.video = Some(b"fake video data".to_vec().into());
        lock.insert(id, job);
    }

    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get(&format!("/status/{id}")).await;
    response.assert_status_ok();
    let body = response.text();
    assert!(body.contains("\"status\":\"done\""));
}

#[tokio::test]
async fn test_status_for_error_job() {
    use crate::types::ReplayStatus;

    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Manually insert a failed job
    {
        let mut lock = state.write().await;
        let mut job = Replay::new(valid_replay_data().into());
        job.status = ReplayStatus::Error;
        job.error_message = Some("Test error message".to_string());
        lock.insert(id, job);
    }

    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get(&format!("/status/{id}")).await;
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.text();
    assert!(body.contains("\"status\":\"error\""));
    assert!(body.contains("Test error message"));
}

#[tokio::test]
async fn test_status_for_error_job_without_message() {
    use crate::types::ReplayStatus;

    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Manually insert a failed job without an error message
    {
        let mut lock = state.write().await;
        let mut job = Replay::new(valid_replay_data().into());
        job.status = ReplayStatus::Error;
        job.error_message = None;
        lock.insert(id, job);
    }

    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get(&format!("/status/{id}")).await;
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.text();
    assert!(body.contains("\"status\":\"error\""));
    assert!(body.contains("An error has occurred"));
}

// =============================================================================
// Download endpoint edge case tests
// =============================================================================

#[tokio::test]
async fn test_download_job_not_done_yet() {
    use crate::types::ReplayStatus;

    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Insert a job that is still processing (no video yet)
    {
        let mut lock = state.write().await;
        let mut job = Replay::new(valid_replay_data().into());
        job.status = ReplayStatus::Recording;
        lock.insert(id, job);
    }

    let app = create_app_without_rate_limit(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server.get(&format!("/download/{id}")).await;
    response.assert_status_not_found();

    // Verify job was NOT removed from state
    assert!(
        state.read().await.contains_key(&id),
        "Job should still exist"
    );
}

#[tokio::test]
async fn test_download_job_in_error_state() {
    use crate::types::ReplayStatus;

    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Insert a failed job
    {
        let mut lock = state.write().await;
        let mut job = Replay::new(valid_replay_data().into());
        job.status = ReplayStatus::Error;
        lock.insert(id, job);
    }

    let app = create_app_without_rate_limit(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server.get(&format!("/download/{id}")).await;
    response.assert_status_not_found();

    // Verify job was NOT removed
    assert!(
        state.read().await.contains_key(&id),
        "Failed job should still exist"
    );
}

#[tokio::test]
async fn test_download_success_removes_job() {
    use crate::types::ReplayStatus;

    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Insert a completed job with video
    {
        let mut lock = state.write().await;
        let mut job = Replay::new(valid_replay_data().into());
        job.status = ReplayStatus::Done;
        job.video = Some(b"fake video data".to_vec().into());
        lock.insert(id, job);
    }

    let app = create_app_without_rate_limit(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server.get(&format!("/download/{id}")).await;
    response.assert_status_ok();
    assert_eq!(response.as_bytes().as_ref(), b"fake video data");

    // Verify job was removed from state after download
    assert!(
        !state.read().await.contains_key(&id),
        "Job should be removed after download"
    );
}

// =============================================================================
// is_replay_file() edge case tests
// =============================================================================

#[tokio::test]
async fn test_upload_empty_file() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.post("/upload").bytes(Vec::new().into()).await;
    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_upload_too_short_file() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    // Only 3 bytes - less than magic number length
    let response = server.post("/upload").bytes(b"yrp".to_vec().into()).await;
    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_upload_wrong_magic_bytes() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    // 4 bytes but wrong magic
    let response = server.post("/upload").bytes(b"yrp1".to_vec().into()).await;
    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_upload_exact_magic_bytes_only() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    // Exactly 4 bytes - the magic number and nothing else
    let response = server.post("/upload").bytes(b"yrpX".to_vec().into()).await;
    response.assert_status_ok();
}

// =============================================================================
// Replay struct unit tests
// =============================================================================

#[test]
fn test_replay_new_initial_state() {
    use crate::types::ReplayStatus;

    let data = valid_replay_data();
    let replay = Replay::new(data.clone().into());

    assert_eq!(replay.data.as_ref(), data.as_slice());
    assert!(replay.video.is_none());
    assert_eq!(replay.status, ReplayStatus::Queued);
    assert!(replay.error_message.is_none());
}

#[test]
fn test_replay_is_replay_file_valid() {
    let replay = Replay::new(b"yrpXsomedata".to_vec().into());
    assert!(replay.is_replay_file());
}

#[test]
fn test_replay_is_replay_file_invalid() {
    let replay = Replay::new(b"notavalid".to_vec().into());
    assert!(!replay.is_replay_file());
}

#[test]
fn test_replay_is_replay_file_empty() {
    let replay = Replay::new(Vec::new().into());
    assert!(!replay.is_replay_file());
}

#[test]
fn test_replay_is_replay_file_too_short() {
    let replay = Replay::new(b"yrp".to_vec().into());
    assert!(!replay.is_replay_file());
}

// =============================================================================
// Invalid path parameter tests
// =============================================================================

#[tokio::test]
async fn test_status_invalid_ulid() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/status/not-a-valid-ulid").await;
    // Axum returns 400 Bad Request for invalid path parameters
    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_download_invalid_ulid() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/download/invalid-ulid").await;
    response.assert_status_bad_request();
}
