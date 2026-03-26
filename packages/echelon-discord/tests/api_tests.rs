use echelon_discord::api::{ReplayConfig, ReplayStatus, create_replay_with_config, download_video, get_replay_status, upload_replay};
use echelon_discord::helpers::translate_api_error;
use serde_json::json;

#[test]
fn test_replay_status_parsing_queued() {
    let json = r#"{"status":"queued","position":5}"#;
    let status: ReplayStatus = serde_json::from_str(json).unwrap();

    match status {
        ReplayStatus::Queued {
            position,
            estimate_minutes: _,
        } => {
            assert_eq!(position, 5);
        }
        _ => panic!("Expected Queued status"),
    }
}

#[test]
fn test_replay_status_parsing_processing() {
    let json = r#"{"status":"processing","estimate_minutes":5}"#;
    let status: ReplayStatus = serde_json::from_str(json).unwrap();

    match status {
        ReplayStatus::Processing { .. } => {}
        _ => panic!("Expected Processing status"),
    }
}

#[test]
fn test_replay_status_parsing_done() {
    let json = r#"{"status":"done"}"#;
    let status: ReplayStatus = serde_json::from_str(json).unwrap();

    match status {
        ReplayStatus::Done => {}
        _ => panic!("Expected Done status"),
    }
}

#[test]
fn test_replay_status_parsing_error() {
    let json = r#"{"status":"error","message":"Something went wrong"}"#;
    let status: ReplayStatus = serde_json::from_str(json).unwrap();

    match status {
        ReplayStatus::Error { message } => assert_eq!(message, "Something went wrong"),
        _ => panic!("Expected Error status"),
    }
}

#[test]
fn test_replay_status_parsing_not_found() {
    let json = r#"{"status":"not_found","message":"Replay not found"}"#;
    let status: ReplayStatus = serde_json::from_str(json).unwrap();

    match status {
        ReplayStatus::NotFound { message } => assert_eq!(message, "Replay not found"),
        _ => panic!("Expected NotFound status"),
    }
}

#[tokio::test]
async fn test_create_replay_success() {
    let mut server = mockito::Server::new_async().await;

    let _mock = server
        .mock("POST", "/create")
        .match_body(mockito::Matcher::Json(json!({
            "top_down_view": false,
            "swap_players": false,
            "game_speed": 1.0,
            "video_preset": "balanced"
        })))
        .with_status(200)
        .with_body("replay-id-123")
        .expect(1)
        .create();

    let result = create_replay_with_config(&server.url(), &ReplayConfig::default()).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "replay-id-123");
}

#[tokio::test]
async fn test_create_replay_server_error() {
    let mut server = mockito::Server::new_async().await;

    let _mock = server
        .mock("POST", "/create")
        .match_body(mockito::Matcher::Json(json!({
            "top_down_view": false,
            "swap_players": false,
            "game_speed": 1.0,
            "video_preset": "balanced"
        })))
        .with_status(500)
        .expect(1)
        .create();

    let result = create_replay_with_config(&server.url(), &ReplayConfig::default()).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("500"));
}

#[tokio::test]
async fn test_upload_replay_success() {
    let mut server = mockito::Server::new_async().await;

    let _mock = server
        .mock(
            "POST",
            mockito::Matcher::Regex(r"^/upload\?task_id=.*".to_string()),
        )
        .with_status(200)
        .expect(1)
        .create();

    let test_data = b"fake replay data";
    let result = upload_replay(&server.url(), "test-task-id", test_data.to_vec()).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_upload_replay_server_error() {
    let mut server = mockito::Server::new_async().await;

    let _mock = server
        .mock(
            "POST",
            mockito::Matcher::Regex(r"^/upload\?task_id=.*".to_string()),
        )
        .with_status(500)
        .expect(1)
        .create();

    let test_data = b"fake replay data";
    let result = upload_replay(&server.url(), "test-task-id", test_data.to_vec()).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("500"));
}

#[tokio::test]
async fn test_upload_replay_invalid_file_includes_server_details() {
    let mut server = mockito::Server::new_async().await;

    let _mock = server
        .mock(
            "POST",
            mockito::Matcher::Regex(r"^/upload\?task_id=.*".to_string()),
        )
        .with_status(400)
        .with_body("File is not a *.yrpX file.")
        .expect(1)
        .create();

    let result = upload_replay(&server.url(), "test-task-id", b"not-a-replay".to_vec()).await;

    let err = result.expect_err("upload should fail for invalid file");
    assert!(err.contains("400"));
    assert!(err.contains("File is not a *.yrpX file."));
}

#[test]
fn test_translate_api_error_upload_invalid_format() {
    let msg =
        translate_api_error("Server returned 400 Bad Request: File is not a *.yrpX file.", "upload");
    assert!(msg.contains("Invalid replay format"));
}

#[test]
fn test_translate_api_error_upload_corrupted_replay() {
    let msg = translate_api_error(
        "Server returned 400 Bad Request: Invalid replay file - make sure it is not corrupted.",
        "upload",
    );
    assert!(msg.contains("corrupted or unreadable"));
}

#[test]
fn test_translate_api_error_upload_missing_task() {
    let msg = translate_api_error(
        "Server returned 404 Not Found: Task ID not found - please create a new task before uploading.",
        "upload",
    );
    assert!(msg.contains("session expired or was not found"));
}

#[test]
fn test_translate_api_error_queue_full() {
    let msg = translate_api_error(
        "Server returned 503 Service Unavailable: Queue is full. Please try again later.",
        "create",
    );
    assert!(msg.contains("queue is currently full"));
}

#[tokio::test]
async fn test_get_replay_status_success() {
    let mut server = mockito::Server::new_async().await;

    let _mock = server
        .mock("GET", "/status/test-id")
        .with_status(200)
        .with_body(r#"{"status":"processing","estimate_minutes":5}"#)
        .expect(1)
        .create();

    let result = get_replay_status(&server.url(), "test-id").await;

    assert!(result.is_ok());
    match result.unwrap() {
        ReplayStatus::Processing { .. } => {}
        _ => panic!("Expected Processing status"),
    }
}

#[tokio::test]
async fn test_get_replay_status_not_found() {
    let mut server = mockito::Server::new_async().await;

    let _mock = server
        .mock("GET", "/status/nonexistent")
        .with_status(404)
        .expect(1)
        .create();

    let result = get_replay_status(&server.url(), "nonexistent").await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("404"));
}

#[tokio::test]
async fn test_download_video_success() {
    let mut server = mockito::Server::new_async().await;

    let video_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG magic bytes
    let _mock = server
        .mock("GET", "/download/test-id")
        .with_status(200)
        .with_body(video_data.clone())
        .expect(1)
        .create();

    let result = download_video(&server.url(), "test-id").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), video_data);
}

#[tokio::test]
async fn test_download_video_not_found() {
    let mut server = mockito::Server::new_async().await;

    let _mock = server
        .mock("GET", "/download/nonexistent")
        .with_status(404)
        .expect(1)
        .create();

    let result = download_video(&server.url(), "nonexistent").await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("404"));
}
