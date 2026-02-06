//! Server API communication module for echelon-server.
//!
//! Handles uploading replays, checking status, and downloading finished videos.

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::env;

/// Represents the status of a replay processing job from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ReplayStatus {
    #[serde(rename = "queued")]
    Queued {
        position: u32,
        #[serde(default)]
        eta: f64,
    },
    #[serde(rename = "processing")]
    Processing { duration: f64 },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "not_found")]
    NotFound { message: String },
}

/// Uploads a replay file to the echelon server.
/// Returns the unique ID assigned to the replay.
pub async fn upload_file(url: &str, data: &[u8]) -> Result<String, String> {
    let client = HttpClient::new();

    let response = client
        .post(url)
        .header("Content-Type", "application/octet-stream")
        .body(data.to_vec())
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

/// Fetches the current status of a replay from the server.
pub async fn get_replay_status(server_url: &str, id: &str) -> Result<ReplayStatus, String> {
    let client = HttpClient::new();
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
    let client = HttpClient::new();
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

/// Gets the echelon server URL from environment or uses default.
pub fn get_server_url() -> String {
    env::var("ECHELON_SERVER_URL").expect("ECHELON_SERVER_URL must be set")
}
