//! The application endpoints.
//!
//! [`upload()`] queues a replay to be processed.
//! [`status()`] checks the processing status of a given replay.
//! [`download()`] grabs the finished video from the app state and sends it to the caller.

use crate::types::Replay;
use crate::types::ReplayStatus;
use axum::Json;
use axum::body::Bytes;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use ulid::Ulid;

/// Maximum number of jobs allowed in the queue.
const MAX_QUEUE_SIZE: usize = 100;

fn estimate_minutes_from_seconds(seconds: f64) -> u32 {
    if seconds <= 0.0 {
        0
    } else {
        (seconds / 60.0).ceil().max(1.0) as u32
    }
}

/// Response type for the status endpoint.
#[derive(Serialize)]
#[serde(tag = "status")]
enum StatusResponse {
    /// Job is queued and waiting to be processed.
    #[serde(rename = "queued")]
    Queued {
        position: usize,
        estimate_minutes: u32,
    },
    /// Job is currently being processed.
    #[serde(rename = "processing")]
    Processing { estimate_minutes: u32 },
    /// Job is done and ready for download.
    #[serde(rename = "done")]
    Done,
    /// Job encountered an error.
    #[serde(rename = "error")]
    Error { message: String },
    /// Job was not found.
    #[serde(rename = "not_found")]
    NotFound { message: String },
}

/// Checks for a given replay ID and returns the video data for it (if any).
pub async fn download(
    State(jobs): State<Arc<RwLock<BTreeMap<Ulid, Replay>>>>,
    Path(id): Path<Ulid>,
) -> impl IntoResponse {
    tracing::debug!("[{}] Download requested.", id);
    let lock = jobs.write().await;

    // Check if job exists, is done, and has video data before removing
    let has_video = lock
        .get(&id)
        .is_some_and(|job| job.status == ReplayStatus::Done && job.video.is_some());

    if has_video {
        // Safe to remove - we verified video exists
        let replay = lock.get(&id).unwrap();
        let video_data = replay.video.clone().unwrap();
        let video_size = video_data.len();

        tracing::info!(
            "[{}] Download successful. Video size: {} bytes ({:.2} MB).",
            id,
            video_size,
            video_size as f64 / (1024.0 * 1024.0)
        );

        let disposition = format!("attachment; filename=\"{id}.mp4\"");
        (
            StatusCode::OK,
            [
                ("Content-Type", "video/mp4"),
                ("Content-Disposition", disposition.as_str()),
            ],
            video_data,
        )
            .into_response()
    } else {
        tracing::warn!("[{}] Download failed - video not found or not ready.", id);
        (
            StatusCode::NOT_FOUND,
            [("Content-Type", "text/plain"), ("Content-Disposition", "")],
            Bytes::from("Video not found."),
        )
            .into_response()
    }
}

/// Checks the status of a given replay ID and returns it.
pub async fn status(
    State(jobs): State<Arc<RwLock<BTreeMap<Ulid, Replay>>>>,
    Path(id): Path<Ulid>,
) -> impl IntoResponse {
    tracing::debug!("[{}] Status check requested.", id);
    let lock = jobs.read().await;
    let job = lock.get(&id);
    let status_option = job.map(|j| j.status.clone());
    let error_message = job.and_then(|j| j.error_message.clone());

    match status_option {
        Some(ReplayStatus::Queued) => {
            // Collect queued jobs to calculate position and ETA
            let queued_jobs: Vec<_> = lock
                .iter()
                .filter(|&(_, job)| job.status == ReplayStatus::Queued)
                .collect();
            let recording_estimate = lock
                .values()
                .find(|job| job.status == ReplayStatus::Recording)
                .map(|job| estimate_minutes_from_seconds(job.estimated_duration))
                .unwrap_or(0);

            if let Some(position) = queued_jobs.iter().position(|(job_id, _)| **job_id == id) {
                // Sum estimated durations of all queued replays up to and including this one
                let estimate_minutes: u32 = recording_estimate
                    + queued_jobs
                        .iter()
                        .take(position + 1)
                        .map(|(_, job)| estimate_minutes_from_seconds(job.estimated_duration))
                        .sum::<u32>();

                (
                    StatusCode::OK,
                    Json(StatusResponse::Queued {
                        position: position + 1,
                        estimate_minutes,
                    }),
                )
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(StatusResponse::NotFound {
                        message: String::from(
                            "No replay with the requested ID was found in the queue.",
                        ),
                    }),
                )
            }
        }
        Some(ReplayStatus::Recording) => {
            let estimate_minutes = estimate_minutes_from_seconds(job.unwrap().estimated_duration);

            (
                StatusCode::OK,
                Json(StatusResponse::Processing { estimate_minutes }),
            )
        }
        Some(ReplayStatus::Done) => (StatusCode::OK, Json(StatusResponse::Done)),
        Some(ReplayStatus::Error) => (
            StatusCode::OK,
            Json(StatusResponse::Error {
                message: error_message.unwrap_or_else(|| String::from("An error has occurred.")),
            }),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(StatusResponse::NotFound {
                message: String::from("No replay with the requested ID was found."),
            }),
        ),
    }
}

/// Receives a replay file and adds it to the processing queue.
pub async fn upload(
    State(jobs): State<Arc<RwLock<BTreeMap<Ulid, Replay>>>>,
    body: Bytes,
) -> impl IntoResponse {
    let id = Ulid::new();
    let file_size = body.len();

    tracing::info!(
        "[{}] Received replay file. Size: {} bytes ({:.2} KB).",
        id,
        file_size,
        file_size as f64 / 1024.0
    );

    let replay = Replay::new(body);
    let mut lock = jobs.write().await;

    // Check if we have space in the queue
    let current_queue_size = lock.len();
    if current_queue_size >= MAX_QUEUE_SIZE {
        tracing::warn!(
            "[{}] Queue full ({}/{} jobs). Refusing upload.",
            id,
            current_queue_size,
            MAX_QUEUE_SIZE
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            String::from("Queue is full. Please try again later."),
        );
    }

    if !replay.is_replay_file() {
        tracing::error!("[{}] Invalid replay file format.", id);
        return (
            StatusCode::BAD_REQUEST,
            String::from("File is not a *.yrpX file."),
        );
    }

    tracing::info!(
        "[{}] File is valid - adding to queue. Queue size: {}/{}.",
        id,
        current_queue_size + 1,
        MAX_QUEUE_SIZE
    );
    lock.insert(id, replay);

    (StatusCode::OK, id.to_string())
}
