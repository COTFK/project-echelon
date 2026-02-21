//! Types relating to replays, videos and their status.

use crate::estimation::estimate_duration;
use crate::estimation::load_replay_packets;
use axum::body::Bytes;
use serde::{Deserialize, Deserializer};

/// The processing status of a video.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum ReplayStatus {
    /// The task is created but not ready to be processed yet.
    Created,
    /// The video is done and ready to be downloaded.
    Done,
    /// An error was encountered during processing.
    Error,
    /// The video is currently being recorded.
    Recording,
    /// The video is queued and waiting to be processed.
    Queued,
}

/// Video encoding presets clients can request for each replay.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VideoPreset {
    /// Prioritize smaller files (higher CRF, faster encoding).
    FileSize,
    /// Balanced quality vs performance (current default).
    Balanced,
    /// Higher quality encoding (lower CRF, slower preset).
    Quality,
}

impl Default for VideoPreset {
    fn default() -> Self {
        VideoPreset::Balanced
    }
}

fn deserialize_game_speed<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    const MIN_GAME_SPEED: f64 = 0.5;
    const MAX_GAME_SPEED: f64 = 10.0;

    let value = f64::deserialize(deserializer)?;

    if value < MIN_GAME_SPEED || value > MAX_GAME_SPEED {
        return Err(serde::de::Error::custom(format!(
            "game_speed must be between {}x and {}x (got {}x)",
            MIN_GAME_SPEED, MAX_GAME_SPEED, value
        )));
    }

    Ok(value)
}

/// Replay configuration.
#[derive(Deserialize, Clone, PartialEq, PartialOrd, Debug)]
pub struct ReplayConfig {
    /// Whether to use top-down view.
    pub top_down_view: bool,
    /// Whether to swap players for recording (swap viewer/player sides).
    pub swap_players: bool,
    /// Optional game speed multiplier for offline replays (0.1x to 10.0x).
    /// Values >1.0 speed up gameplay; values <1.0 slow it down. Defaults to 1.0.
    #[serde(deserialize_with = "deserialize_game_speed")]
    pub game_speed: f64,
    /// Requested video-quality preset for ffmpeg encoding.
    #[serde(default)]
    pub video_preset: VideoPreset,
}

#[derive(Debug)]
pub enum ReplayError {
    /// Failed the magic number check.
    MagicError,
    /// Failed loading replay packets.
    PacketError,
}

/// A tracked *.yrpX replay file.
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Replay {
    pub config: ReplayConfig,
    /// The replay file contents.
    pub data: Option<Bytes>,
    /// The video data, if any.
    pub video: Option<Bytes>,
    /// Estimated video duration
    pub estimated_duration: Option<f64>,
    /// The processing status - queued, recording, etc.
    pub status: ReplayStatus,
    /// Error message if the job failed.
    pub error_message: Option<String>,
}

impl Replay {
    /// Creates an empty [`Replay`] with no data, in Created status.
    pub fn new(config: ReplayConfig) -> Self {
        Self {
            config: config,
            data: None,
            video: None,
            estimated_duration: None,
            status: ReplayStatus::Created,
            error_message: None,
        }
    }

    /// Adds replay file data to the [`Replay`].
    /// Returns an error if the replay file is malformed or cannot be parsed.
    pub fn add_replay_data(&mut self, data: Bytes) -> Result<(), ReplayError> {
        // Check if the replay file is legitimate.
        if !(data.get(..4).is_some_and(|x| x == b"yrpX")) {
            return Err(ReplayError::MagicError);
        }

        let packets = load_replay_packets(&data).map_err(|_| ReplayError::PacketError)?;

        self.data = Some(data);
        self.estimated_duration = Some(estimate_duration(&packets));

        Ok(())
    }

    /// Marks a replay as ready for processing.
    pub fn mark_replay_as_ready(&mut self) -> () {
        self.status = ReplayStatus::Queued;
    }
}
