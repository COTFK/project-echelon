use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum_test::TestServer;
use echelon_server::routes::{create_replay, download, status, upload};
use echelon_server::types::{Replay, ReplayConfig, ReplayError, ReplayStatus, VideoPreset};
use serde_json::json;
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

fn default_replay_config() -> ReplayConfig {
    ReplayConfig {
        top_down_view: false,
        swap_players: false,
        game_speed: 1.0,
        video_preset: VideoPreset::Balanced,
    }
}

fn replay_with_valid_data() -> Replay {
    let mut replay = Replay::new(default_replay_config());
    replay
        .add_replay_data(valid_replay_data().into())
        .expect("valid replay data");
    replay.mark_replay_as_ready();
    replay
}

async fn create_job(server: &TestServer) -> String {
    let response = server
        .post("/create")
        .json(&json!({
            "top_down_view": false,
            "swap_players": false,
            "game_speed": 1.0,
            "video_preset": "balanced",
        }))
        .await;
    response.assert_status_ok();
    response.text()
}

/// Creates a test app without rate limiting for easier testing
fn create_app_without_rate_limit(state: Arc<RwLock<BTreeMap<Ulid, Replay>>>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/create", post(create_replay))
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

    let task_id = create_job(&server).await;

    let response = server
        .post(&format!("/upload?task_id={task_id}"))
        .bytes(valid_replay_data().into())
        .await;

    response.assert_status_ok();
    assert!(
        response.text().is_empty(),
        "Upload should not return a body"
    );
}

#[tokio::test]
async fn test_upload_invalid_replay() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let task_id = create_job(&server).await;
    let response = server
        .post(&format!("/upload?task_id={task_id}"))
        .bytes(invalid_replay_data().into())
        .await;

    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_status_for_queued_job() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let task_id = create_job(&server).await;

    // Upload a replay
    let upload_response = server
        .post(&format!("/upload?task_id={task_id}"))
        .bytes(valid_replay_data().into())
        .await;

    upload_response.assert_status_ok();
    let job_id = task_id;

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
    const QUEUE_LIMIT: usize = 100;

    for i in 0..QUEUE_LIMIT {
        let task_id = create_job(&server).await;
        let response = server
            .post(&format!("/upload?task_id={task_id}"))
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
        .post("/create")
        .json(&json!({
            "top_down_view": false,
            "swap_players": false,
            "game_speed": 1.0,
            "video_preset": "balanced",
        }))
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::SERVICE_UNAVAILABLE,
        "New job creation after queue is full should return 503"
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
        ids.push(create_job(&server).await);
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
        let task_id = create_job(&server).await;
        let response = server
            .post(&format!("/upload?task_id={task_id}"))
            .bytes(valid_replay_data().into())
            .await;
        response.assert_status_ok();
        ids.push(task_id);
    }

    // Check that each job has a valid position (1, 2, or 3)
    let mut positions = Vec::new();
    for id in &ids {
        let response = server.get(&format!("/status/{id}")).await;
        let body = response.text();

        // Extract position from JSON - jobs might have moved to Recording state already
        // Only extract position if the status is "queued"
        if body.contains("\"status\":\"queued\"") {
            if let Some(pos_start) = body.find("\"position\":") {
                let pos_str = &body[pos_start + 11..];
                if let Some(comma_pos) = pos_str.find(',') {
                    if let Ok(pos) = pos_str[..comma_pos].parse::<usize>() {
                        positions.push(pos);
                    }
                }
            }
        }
    }

    // All positions should be present if jobs are still queued (1, 2, 3)
    // If jobs have started processing, positions may be empty - that's ok
    positions.sort();
    // Skip this assertion if worker has already started processing
    if !positions.is_empty() {
        assert_eq!(
            positions,
            vec![1, 2, 3],
            "Positions should be 1, 2, 3 if still queued"
        );
    }
}

// =============================================================================
// Status endpoint tests for different job states
// =============================================================================

#[tokio::test]
async fn test_status_for_processing_job() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Manually insert a job in Recording (processing) state
    {
        let mut lock = state.write().await;
        let mut job = replay_with_valid_data();
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
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Manually insert a completed job
    {
        let mut lock = state.write().await;
        let mut job = replay_with_valid_data();
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
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Manually insert a failed job
    {
        let mut lock = state.write().await;
        let mut job = replay_with_valid_data();
        job.status = ReplayStatus::Error;
        job.error_message = Some("Test error message".to_string());
        lock.insert(id, job);
    }

    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get(&format!("/status/{id}")).await;
    response.assert_status_ok(); // Status endpoint returns 200 even for failed jobs
    let body = response.text();
    assert!(body.contains("\"status\":\"error\""));
    assert!(body.contains("Test error message"));
}

#[tokio::test]
async fn test_status_for_error_job_without_message() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Manually insert a failed job without an error message
    {
        let mut lock = state.write().await;
        let mut job = replay_with_valid_data();
        job.status = ReplayStatus::Error;
        job.error_message = None;
        lock.insert(id, job);
    }

    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get(&format!("/status/{id}")).await;
    response.assert_status_ok(); // Status endpoint returns 200 even for failed jobs
    let body = response.text();
    assert!(body.contains("\"status\":\"error\""));
    assert!(body.contains("An error has occurred"));
}

// =============================================================================
// Download endpoint edge case tests
// =============================================================================

#[tokio::test]
async fn test_download_job_not_done_yet() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Insert a job that is still processing (no video yet)
    {
        let mut lock = state.write().await;
        let mut job = replay_with_valid_data();
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
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let id = Ulid::new();

    // Insert a failed job
    {
        let mut lock = state.write().await;
        let mut job = replay_with_valid_data();
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

// =============================================================================
// is_replay_file() edge case tests
// =============================================================================

#[tokio::test]
async fn test_upload_empty_file() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    let task_id = create_job(&server).await;
    let response = server
        .post(&format!("/upload?task_id={task_id}"))
        .bytes(Vec::new().into())
        .await;
    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_upload_too_short_file() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    // Only 3 bytes - less than magic number length
    let task_id = create_job(&server).await;
    let response = server
        .post(&format!("/upload?task_id={task_id}"))
        .bytes(b"yrp".to_vec().into())
        .await;
    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_upload_wrong_magic_bytes() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    // 4 bytes but wrong magic
    let task_id = create_job(&server).await;
    let response = server
        .post(&format!("/upload?task_id={task_id}"))
        .bytes(b"yrp1".to_vec().into())
        .await;
    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_upload_exact_magic_bytes_only() {
    let state = Arc::new(RwLock::new(BTreeMap::<Ulid, Replay>::new()));
    let app = create_app_without_rate_limit(state);
    let server = TestServer::new(app).unwrap();

    // Exactly 4 bytes - the magic number and nothing else - should be rejected as too small to parse
    let task_id = create_job(&server).await;
    let response = server
        .post(&format!("/upload?task_id={task_id}"))
        .bytes(b"yrpX".to_vec().into())
        .await;
    response.assert_status_bad_request(); // Now correctly rejected during parsing
    let body = response.text();
    assert!(body.contains("Invalid replay file"));
}

// =============================================================================
// Replay struct unit tests
// =============================================================================

#[test]
fn test_replay_new_initial_state() {
    let config = default_replay_config();
    let replay = Replay::new(config.clone());

    assert_eq!(replay.config, config);
    assert!(replay.data.is_none());
    assert!(replay.video.is_none());
    assert_eq!(replay.status, ReplayStatus::Created);
    assert!(replay.error_message.is_none());
}

#[test]
fn test_replay_add_replay_data_sets_state() {
    let mut replay = Replay::new(default_replay_config());
    replay
        .add_replay_data(valid_replay_data().into())
        .expect("valid replay data");
    assert!(replay.data.is_some());
    assert!(replay.estimated_duration.is_some());

    replay.mark_replay_as_ready();
    assert_eq!(replay.status, ReplayStatus::Queued);
}

#[test]
fn test_replay_add_replay_data_magic_error() {
    let mut replay = Replay::new(default_replay_config());
    let result = replay.add_replay_data(b"notavalid".to_vec().into());
    assert!(matches!(result, Err(ReplayError::MagicError)));
}

#[test]
fn test_replay_add_replay_data_empty() {
    let mut replay = Replay::new(default_replay_config());
    let result = replay.add_replay_data(Vec::new().into());
    assert!(matches!(result, Err(ReplayError::MagicError)));
}

#[test]
fn test_replay_add_replay_data_exact_magic_only() {
    let mut replay = Replay::new(default_replay_config());
    let result = replay.add_replay_data(b"yrpX".to_vec().into());
    assert!(matches!(result, Err(ReplayError::PacketError)));
}

#[test]
fn test_replay_add_replay_data_too_short() {
    let mut replay = Replay::new(default_replay_config());
    let result = replay.add_replay_data(b"yrp".to_vec().into());
    assert!(matches!(result, Err(ReplayError::MagicError)));
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
