//! Server API communication module for echelon-server.
//!
//! Handles uploading replays, checking status, and downloading finished videos.

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::sync::OnceLock;
use std::time::Duration;

/// Cached server URL, initialized once on first access.
static SERVER_URL: OnceLock<String> = OnceLock::new();
/// Shared HTTP client, initialized once and reused for connection pooling.
static HTTP_CLIENT: OnceLock<HttpClient> = OnceLock::new();

/// Represents the status of a replay processing job from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Video encoding presets that can be requested when uploading a replay.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoPreset {
    FileSize,
    Balanced,
    Quality,
}

impl VideoPreset {
    pub const fn as_str(self) -> &'static str {
        match self {
            VideoPreset::FileSize => "file_size",
            VideoPreset::Balanced => "balanced",
            VideoPreset::Quality => "quality",
        }
    }

    pub fn from_str_name(value: &str) -> Self {
        match value {
            "file_size" => VideoPreset::FileSize,
            "quality" => VideoPreset::Quality,
            _ => VideoPreset::Balanced,
        }
    }
}

/// Replay configuration sent to the server.
#[derive(Debug, Clone, Serialize)]
pub struct ReplayConfig {
    /// Whether to use top-down view.
    pub top_down_view: bool,
    /// Whether to swap players for recording.
    pub swap_players: bool,
    /// Game speed multiplier (0.5x to 10.0x).
    pub game_speed: f64,
    /// Requested video encoding preset.
    pub video_preset: VideoPreset,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            top_down_view: false,
            swap_players: false,
            game_speed: 1.0,
            video_preset: VideoPreset::Balanced,
        }
    }
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

fn get_http_client() -> &'static HttpClient {
    HTTP_CLIENT.get_or_init(create_http_client)
}

/// Adds bot authentication header to a request if BOT_SECRET is set.
fn add_bot_auth(mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Some(bot_secret) = env::var("BOT_SECRET").ok().filter(|s| !s.is_empty()) {
        request = request.header("X-Bot-Secret", bot_secret);
    }
    request
}

/// Creates a replay job with the provided configuration.
/// Returns the unique ID assigned to the replay.
pub async fn create_replay_with_config(
    server_url: &str,
    config: &ReplayConfig,
) -> Result<String, String> {
    let client = get_http_client();
    let create_url = format!("{}/create", server_url);

    let request = client
        .post(&create_url)
        .header("Content-Type", "application/json");
    let request = add_bot_auth(request);

    let body = json!({
        "top_down_view": config.top_down_view,
        "swap_players": config.swap_players,
        "game_speed": config.game_speed,
        "video_preset": config.video_preset.as_str()
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
pub async fn upload_replay(server_url: &str, task_id: &str, data: Vec<u8>) -> Result<(), String> {
    let client = get_http_client();
    let upload_url = format!("{}/upload?task_id={}", server_url, task_id);

    let request = client
        .post(&upload_url)
        .header("Content-Type", "application/octet-stream");
    let request = add_bot_auth(request);

    let response = request
        .body(data)
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
    let client = get_http_client();
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
    let client = get_http_client();
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
    SERVER_URL
        .get_or_init(|| env::var("ECHELON_SERVER_URL").expect("ECHELON_SERVER_URL must be set"))
}

/// Validates that `ECHELON_SERVER_URL` is set. Call at startup to fail fast.
pub fn validate_server_url() {
    let _ = get_server_url();
}
