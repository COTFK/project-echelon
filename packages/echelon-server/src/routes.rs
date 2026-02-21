//! The application endpoints.
//!
//! [`upload()`] queues a replay to be processed.
//! [`status()`] checks the processing status of a given replay.
//! [`download()`] grabs the finished video from the app state and sends it to the caller.

use crate::types::Replay;
use crate::types::ReplayError;
use crate::types::ReplayStatus;
use axum::Json;
use axum::body::Bytes;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::HeaderMap;
use axum::response::IntoResponse;
use serde::Deserialize;
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
    /// Job is created and awaiting replay upload.
    #[serde(rename = "created")]
    Created,
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

/// Query parameters for the download endpoint.
#[derive(Deserialize)]
pub struct DownloadQuery {
    /// If set to "1", forces download with Content-Disposition: attachment
    #[serde(default)]
    pub download: String,
}

/// Checks for a given replay ID and returns the video data for it (if any).
pub async fn download(
    State(jobs): State<Arc<RwLock<BTreeMap<Ulid, Replay>>>>,
    Path(id): Path<Ulid>,
    Query(params): Query<DownloadQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    tracing::debug!("[{}] Download requested.", id);
    let lock = jobs.read().await;

    // Safely extract video data if job exists, is done, and has video
    if let Some(replay) = lock.get(&id)
        && replay.status == ReplayStatus::Done
        && let Some(ref video_data) = replay.video
    {
        let video_size = video_data.len();

        tracing::info!(
            "[{}] Download successful. Video size: {} bytes ({:.2} MB).",
            id,
            video_size,
            video_size as f64 / (1024.0 * 1024.0)
        );

        // Handle HTTP Range requests for video seeking
        if let Some(range_value) = headers.get("range")
            && let Ok(range_str) = range_value.to_str()
            && let Ok(parsed_ranges) = http_range_header::parse_range_header(range_str)
        {
            match parsed_ranges.validate(video_size as u64) {
                Ok(ranges) => {
                    if let Some(range) = ranges.first() {
                        let start = *range.start() as usize;
                        let end = *range.end() as usize;
                        let range_data = video_data.slice(start..=end);

                        return (
                            StatusCode::PARTIAL_CONTENT,
                            [
                                ("Content-Type", "video/mp4".to_string()),
                                (
                                    "Content-Range",
                                    format!("bytes {}-{}/{}", start, end, video_size),
                                ),
                                ("Content-Length", (end - start + 1).to_string()),
                                ("Accept-Ranges", "bytes".to_string()),
                            ],
                            range_data,
                        )
                            .into_response();
                    }
                }
                Err(_) => {
                    return (
                        StatusCode::RANGE_NOT_SATISFIABLE,
                        [("Content-Range", format!("bytes */{}", video_size))],
                        Bytes::new(),
                    )
                        .into_response();
                }
            }
        }

        // No range request - send full video
        let video_data = video_data.clone();
        let disposition = if params.download == "1" {
            format!("attachment; filename=\"{id}.mp4\"")
        } else {
            format!("inline; filename=\"{id}.mp4\"")
        };
        return (
            StatusCode::OK,
            [
                ("Content-Type", "video/mp4".to_string()),
                ("Content-Disposition", disposition),
                ("Content-Length", video_size.to_string()),
                ("Accept-Ranges", "bytes".to_string()),
            ],
            video_data,
        )
            .into_response();
    }

    // Job not found, not done, or no video data available
    tracing::warn!("[{}] Download failed - video not found or not ready.", id);
    (
        StatusCode::NOT_FOUND,
        [("Content-Type", "text/plain"), ("Content-Disposition", "")],
        Bytes::from("Video not found."),
    )
        .into_response()
}

/// Checks the status of a given replay ID and returns it.
pub async fn status(
    State(jobs): State<Arc<RwLock<BTreeMap<Ulid, Replay>>>>,
    Path(id): Path<Ulid>,
) -> impl IntoResponse {
    tracing::debug!("[{}] Status check requested.", id);
    let lock = jobs.read().await;

    if let Some(job) = lock.get(&id) {
        match job.status {
            ReplayStatus::Created => {
                (StatusCode::OK, Json(StatusResponse::Created))
            }
            ReplayStatus::Queued => {
                // Collect queued jobs to calculate position and ETA
                let queued_jobs: Vec<_> = lock
                    .iter()
                    .filter(|&(_, job)| job.status == ReplayStatus::Queued)
                    .collect();
                let recording_estimate = lock
                    .values()
                    .find(|job| job.status == ReplayStatus::Recording)
                    .map(|job| estimate_minutes_from_seconds(job.estimated_duration.unwrap_or(0.0)))
                    .unwrap_or(0);

                if let Some(position) = queued_jobs.iter().position(|(job_id, _)| **job_id == id) {
                    // Sum estimated durations of all queued replays up to and including this one
                    let estimate_minutes: u32 = recording_estimate
                        + queued_jobs
                            .iter()
                            .take(position + 1)
                            .map(|(_, job)| estimate_minutes_from_seconds(job.estimated_duration.unwrap_or(0.0)))
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
            ReplayStatus::Recording => {
                let estimate_minutes = estimate_minutes_from_seconds(job.estimated_duration.unwrap_or(0.0));

                (
                    StatusCode::OK,
                    Json(StatusResponse::Processing { estimate_minutes }),
                )
            }
            ReplayStatus::Done => (StatusCode::OK, Json(StatusResponse::Done)),
            ReplayStatus::Error => (
                StatusCode::OK,
                Json(StatusResponse::Error {
                    message: job
                        .error_message
                        .clone()
                        .unwrap_or_else(|| String::from("An error has occurred.")),
                }),
            ),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(StatusResponse::NotFound {
                message: String::from("No replay with the requested ID was found."),
            }),
        )
    }
}

pub async fn create_replay(
    State(jobs): State<Arc<RwLock<BTreeMap<Ulid, Replay>>>>,
) -> impl IntoResponse {
    let id = Ulid::new();

    let mut lock = jobs.write().await;

    // Check if we have space in the queue (only count active jobs)
    let current_queue_size = lock
        .values()
        .filter(|job| job.status == ReplayStatus::Queued)
        .count();

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

    tracing::info!(
        "[{}] Queue size: {}/{} - adding task to queue.",
        id,
        current_queue_size + 1,
        MAX_QUEUE_SIZE
    );

    let replay = Replay::new();
    lock.insert(id, replay);

    (StatusCode::OK, id.to_string())
}

/// Query parameters for the upload endpoint.
#[derive(Deserialize)]
pub struct UploadQuery {
    #[serde(default)]
    pub task_id: Ulid,
}

/// Receives a replay file and adds it to the processing queue.
pub async fn upload(
    Query(params): Query<UploadQuery>,
    State(jobs): State<Arc<RwLock<BTreeMap<Ulid, Replay>>>>,
    body: Bytes,
) -> impl IntoResponse {
    let file_size = body.len();

    tracing::info!(
        "[{}] Received replay file. Size: {} bytes ({:.2} KB).",
        params.task_id,
        file_size,
        file_size as f64 / 1024.0
    );

    let mut lock = jobs.write().await;
    let entry = lock.get_mut(&params.task_id);

    match entry {
        Some(replay) => {
            if replay.status != ReplayStatus::Created {
                return (
                    StatusCode::BAD_REQUEST,
                    String::from("Task is already finished. Create a new task to upload a replay."),
                );
            }


            let upload_result = replay.add_replay_data(body);

            match upload_result {
                Ok(()) => {
                    replay.mark_replay_as_ready();
                    return (StatusCode::OK, String::new());
                }
                Err(ReplayError::MagicError) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        String::from("File is not a *.yrpX file."),
                    );
                }
                Err(ReplayError::PacketError) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid replay file - make sure it is not corrupted."),
                    );
                }
            }
        }
        None => {
            return (
                StatusCode::NOT_FOUND,
                String::from("Task ID not found - please create a new task before uploading."),
            );
        }
    }
}
