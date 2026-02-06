//! Type definitions for the replay upload application.

use serde::Deserialize;

/// Maximum allowed file size in bytes (10MB).
pub const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Valid replay file extension.
pub const REPLAY_EXTENSION: &str = ".yrpX";

/// Base URL for the API server.
/// Defaulting to http://localhost:3000 if unconfigured, to match the local Compose file
pub const API_BASE_URL: &str = match option_env!("API_BASE_URL") {
    Some(url) => url,
    None => "http://localhost:3000",
};

/// Represents the current status of a replay processing job.
#[derive(Clone, PartialEq, Debug, Default)]
pub enum ReplayStatus {
    /// No replay has been uploaded yet.
    #[default]
    Idle,
    /// The replay file is being uploaded to the server.
    Uploading,
    /// The replay is queued for processing at the given position with an ETA.
    Queued { position: usize, eta: f64 },
    /// The replay is currently being processed.
    Processing { duration: f64 },
    /// Processing completed successfully, contains the replay ID for download.
    Completed(String),
    /// An error occurred during upload or processing.
    Error(ReplayError),
}

impl ReplayStatus {
    /// Returns `true` if we should poll for status updates.
    #[inline]
    pub const fn should_poll(&self) -> bool {
        matches!(self, Self::Queued { .. } | Self::Processing { .. })
    }
}

/// Errors that can occur during replay upload and processing.
#[derive(Clone, PartialEq, Debug)]
pub enum ReplayError {
    /// The uploaded file is not a valid replay file.
    InvalidFile,
    /// The server's processing queue is full.
    QueueFull,
    /// The replay was not found on the server.
    NotFound(String),
    /// A network or server error occurred.
    Server(String),
    /// File read or validation error.
    Validation(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFile => {
                write!(f, "Invalid replay file. Please upload a valid .yrpX file.")
            }
            Self::QueueFull => write!(f, "Server queue is full. Please try again later."),
            Self::NotFound(msg) => write!(f, "Replay not found: {msg}"),
            Self::Server(msg) => write!(f, "{msg}"),
            Self::Validation(msg) => write!(f, "{msg}"),
        }
    }
}

/// Response from the status endpoint.
#[derive(Deserialize, Debug)]
pub struct StatusResponse {
    pub status: String,
    #[serde(default)]
    pub position: Option<usize>,
    #[serde(default)]
    pub eta: Option<f64>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
}

impl StatusResponse {
    /// Converts the API response to a [`ReplayStatus`].
    pub fn into_replay_status(self, replay_id: &str) -> ReplayStatus {
        match self.status.as_str() {
            "queued" => ReplayStatus::Queued {
                position: self.position.unwrap_or(0),
                eta: self.eta.unwrap_or(0.0),
            },
            "processing" => ReplayStatus::Processing {
                duration: self.duration.unwrap_or(0.0),
            },
            "done" => ReplayStatus::Completed(replay_id.to_owned()),
            "error" => ReplayStatus::Error(ReplayError::Server(
                self.message.unwrap_or_else(|| "Unknown error".to_owned()),
            )),
            "not_found" => ReplayStatus::Error(ReplayError::NotFound(
                self.message.unwrap_or_else(|| "Job not found".to_owned()),
            )),
            _ => ReplayStatus::Error(ReplayError::Server("Unknown status".to_owned())),
        }
    }
}
