//! Types relating to replays, videos and their status.

use axum::body::Bytes;

/// The processing status of a video.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum ReplayStatus {
    /// The video is done and ready to be downloaded.
    Done,
    /// An error was encountered during processing.
    Error,
    /// The video is currently being recorded.
    Recording,
    /// The video is queued and waiting to be processed.
    Queued,
}

/// A tracked *.yrpX replay file.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Replay {
    /// The replay file contents.
    pub data: Bytes,
    /// The video data, if any.
    pub video: Option<Bytes>,
    /// The processing status - queued, recording, etc.
    pub status: ReplayStatus,
    /// Error message if the job failed.
    pub error_message: Option<String>,
}

impl Replay {
    /// Creates a [`Replay`] with a randomly generated ULID and the given file data.
    pub const fn new(data: Bytes) -> Self {
        Self {
            data,
            video: None,
            status: ReplayStatus::Queued,
            error_message: None,
        }
    }

    /// Checks if the file is a legitimate *.yrpX file.
    pub fn is_replay_file(&self) -> bool {
        // Read the first 4 bytes of the file (if available)
        // and check for the magic `yrpX` value.
        self.data.get(..4).is_some_and(|x| x == b"yrpX")
    }
}
