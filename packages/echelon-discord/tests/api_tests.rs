use echelon_discord::api::{download_video, get_replay_status, upload_file, ReplayStatus};

#[test]
fn test_replay_status_parsing_queued() {
    let json = r#"{"status":"queued","position":5}"#;
    let status: ReplayStatus = serde_json::from_str(json).unwrap();
    
    match status {
        ReplayStatus::Queued { position } => assert_eq!(position, 5),
        _ => panic!("Expected Queued status"),
    }
}

#[test]
fn test_replay_status_parsing_processing() {
    let json = r#"{"status":"processing"}"#;
    let status: ReplayStatus = serde_json::from_str(json).unwrap();
    
    match status {
        ReplayStatus::Processing => {},
        _ => panic!("Expected Processing status"),
    }
}

#[test]
fn test_replay_status_parsing_done() {
    let json = r#"{"status":"done"}"#;
    let status: ReplayStatus = serde_json::from_str(json).unwrap();
    
    match status {
        ReplayStatus::Done => {},
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
async fn test_upload_file_success() {
    let mut server = mockito::Server::new_async().await;
    
    let mock = server
        .mock("POST", "/upload")
        .with_status(200)
        .with_body("replay-id-123")
        .expect(1)
        .create();

    let test_data = b"fake replay data";
    let result = upload_file(&format!("{}/upload", server.url()), test_data).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "replay-id-123");
    mock.assert();
}

#[tokio::test]
async fn test_upload_file_server_error() {
    let mut server = mockito::Server::new_async().await;
    
    let _mock = server
        .mock("POST", "/upload")
        .with_status(500)
        .expect(1)
        .create();

    let test_data = b"fake replay data";
    let result = upload_file(&format!("{}/upload", server.url()), test_data).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("500"));
}

#[tokio::test]
async fn test_get_replay_status_success() {
    let mut server = mockito::Server::new_async().await;
    
    let _mock = server
        .mock("GET", "/status/test-id")
        .with_status(200)
        .with_body(r#"{"status":"processing"}"#)
        .expect(1)
        .create();

    let result = get_replay_status(&server.url(), "test-id").await;

    assert!(result.is_ok());
    match result.unwrap() {
        ReplayStatus::Processing => {},
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
