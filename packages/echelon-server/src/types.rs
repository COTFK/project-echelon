//! Types relating to replays, videos and their status.

use crate::estimation::estimate_duration;
use crate::estimation::load_replay_packets;
use axum::body::Bytes;
use serde::Deserialize;

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

/// Replay configuration.
#[derive(Deserialize, Clone, PartialEq, PartialOrd, Debug)]
pub struct ReplayConfig {
    /// Whether to use top-down view.
    #[serde(default)]
    pub top_down_view: bool,
    /// Whether to swap players for recording (swap viewer/player sides).
    #[serde(default)]
    pub swap_players: bool,
}

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
