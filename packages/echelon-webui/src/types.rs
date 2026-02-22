//! Type definitions for the replay upload application.

use serde::{Deserialize, Serialize};

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

/// Discord bot invite URL
pub const DISCORD_INVITE_URL: &str = match option_env!("DISCORD_INVITE_URL") {
    Some(url) => url,
    None => "#",
};

/// Video encoding presets that can be requested when uploading a replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
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

    pub fn from_str(value: &str) -> Self {
        match value {
            "file_size" => VideoPreset::FileSize,
            "quality" => VideoPreset::Quality,
            _ => VideoPreset::Balanced,
        }
    }
}

impl Default for VideoPreset {
    fn default() -> Self {
        VideoPreset::Balanced
    }
}

/// Replay configuration sent to the server.
#[derive(Clone, Debug, Serialize)]
pub struct ReplayConfig {
    /// Whether to use top-down view.
    pub top_down_view: bool,
    /// Whether to swap players for recording.
    pub swap_players: bool,
    /// Game speed multiplier (0.1x to 10.0x).
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

#[cfg(test)]
mod tests {
    use super::{ReplayConfig, VideoPreset};

    #[test]
    fn replay_config_default_uses_balanced_preset() {
        assert_eq!(ReplayConfig::default().video_preset, VideoPreset::Balanced);
    }

    #[test]
    fn video_preset_serialization_strings_are_stable() {
        assert_eq!(VideoPreset::FileSize.as_str(), "file_size");
        assert_eq!(VideoPreset::Balanced.as_str(), "balanced");
        assert_eq!(VideoPreset::Quality.as_str(), "quality");
    }

    #[test]
    fn video_preset_from_str_handles_unknown_and_known_values() {
        assert_eq!(VideoPreset::from_str("file_size"), VideoPreset::FileSize);
        assert_eq!(VideoPreset::from_str("quality"), VideoPreset::Quality);
        assert_eq!(VideoPreset::from_str("banana"), VideoPreset::Balanced);
    }
}

/// Represents the current status of a replay processing job.
#[derive(Clone, PartialEq, Debug, Default)]
pub enum ReplayStatus {
    /// No replay has been uploaded yet.
    #[default]
    Idle,
    /// The replay file is being uploaded to the server.
    Uploading,
    /// The replay is queued for processing at the given position with an ETA.
    Queued {
        position: usize,
        estimate_minutes: u32,
    },
    /// The replay is currently being processed.
    Processing { estimate_minutes: u32 },
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
    pub estimate_minutes: Option<u32>,
    #[serde(default)]
    pub message: Option<String>,
}

impl StatusResponse {
    /// Converts the API response to a [`ReplayStatus`].
    pub fn into_replay_status(self, replay_id: &str) -> ReplayStatus {
        match self.status.as_str() {
            "queued" => ReplayStatus::Queued {
                position: self.position.unwrap_or(0),
                estimate_minutes: self.estimate_minutes.unwrap_or(0),
            },
            "processing" => ReplayStatus::Processing {
                estimate_minutes: self.estimate_minutes.unwrap_or(0),
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
