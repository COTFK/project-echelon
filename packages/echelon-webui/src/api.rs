//! API client for communicating with the replay processing server.

use crate::types::{API_BASE_URL, ReplayError, ReplayStatus, StatusResponse};

/// API client for the replay server.
#[derive(Clone)]
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new(API_BASE_URL)
    }
}

impl ApiClient {
    /// Creates a new API client with the given base URL.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Uploads a replay file to the server.
    ///
    /// # Returns
    /// - `Ok(replay_id)` on successful upload
    /// - `Err(ReplayError)` on failure
    pub async fn upload_replay(&self, data: Vec<u8>) -> Result<String, ReplayError> {
        let response = self
            .client
            .post(format!("{}/upload", self.base_url))
            .body(data)
            .send()
            .await
            .map_err(|e| ReplayError::Server(e.to_string()))?;

        match response.status().as_u16() {
            200 => response
                .text()
                .await
                .map_err(|e| ReplayError::Server(e.to_string())),
            400 => Err(ReplayError::InvalidFile),
            503 => Err(ReplayError::QueueFull),
            status => {
                let message = response
                    .text()
                    .await
                    .unwrap_or_else(|_| format!("HTTP {status}"));
                Err(ReplayError::Server(message))
            }
        }
    }

    /// Fetches the current status of a replay job.
    ///
    /// # Returns
    /// - `Ok(ReplayStatus)` with the current status
    /// - `Err(ReplayError)` on network or parse failure
    pub async fn get_status(&self, replay_id: &str) -> Result<ReplayStatus, ReplayError> {
        let response = self
            .client
            .get(format!("{}/status/{}", self.base_url, replay_id))
            .send()
            .await
            .map_err(|e| ReplayError::Server(e.to_string()))?;

        let status_response: StatusResponse = response
            .json()
            .await
            .map_err(|e| ReplayError::Server(e.to_string()))?;

        Ok(status_response.into_replay_status(replay_id))
    }

    /// Returns the download URL for a completed replay video.
    #[must_use]
    pub fn download_url(&self, replay_id: &str) -> String {
        format!("{}/download/{}", self.base_url, replay_id)
    }
}

/// Validates a replay file before upload.
///
/// # Returns
/// - `Ok(())` if the file is valid
/// - `Err(ReplayError)` if validation fails
pub fn validate_replay_file(filename: &str, data: &[u8]) -> Result<(), ReplayError> {
    use crate::types::{MAX_FILE_SIZE, REPLAY_EXTENSION};

    if !filename.ends_with(REPLAY_EXTENSION) {
        return Err(ReplayError::InvalidFile);
    }

    if data.len() > MAX_FILE_SIZE {
        return Err(ReplayError::Validation(format!(
            "File exceeds {}MB limit",
            MAX_FILE_SIZE / (1024 * 1024)
        )));
    }

    Ok(())
}
