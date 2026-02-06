//! The [`worker()`] and [`cleanup()`] functions - background tasks for processing and maintenance.

use crate::commands::launch_edopro;
use crate::commands::record_display;
use crate::commands::trim_black_frames;
use crate::types::Replay;
use crate::types::ReplayStatus;
use axum::body::Bytes;
use nix::sys::signal::{SIGINT, kill};
use nix::unistd;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::SystemTime;
use tempfile::tempdir;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};
use ulid::Ulid;

/// Default TTL for completed/errored jobs (1 hour).
const JOB_TTL_SECS: u64 = 3600;

/// Cleanup interval (5 minutes).
const CLEANUP_INTERVAL_SECS: u64 = 300;

/// Maximum time allowed for processing a single job (10 minutes).
const JOB_TIMEOUT_SECS: u64 = 600;

/// Worker function that gets the first replay in the queue,
/// records it and saves it in the output map.
pub async fn worker(state: Arc<RwLock<BTreeMap<Ulid, Replay>>>) -> ! {
    loop {
        // Get first replay in queue
        let next_job = state
            .read()
            .await
            .iter()
            .find(|&(_, job)| job.status == ReplayStatus::Queued)
            .map(|(id, _)| *id);

        if let Some(id) = next_job {
            // Apply timeout to job processing
            let result = tokio::time::timeout(
                Duration::from_secs(JOB_TIMEOUT_SECS),
                process_job(&state, id),
            )
            .await;

            match result {
                Ok(Ok(())) => {
                    tracing::info!("[{}] Job completed successfully.", id);
                }
                Ok(Err(e)) => {
                    tracing::error!("[{}] Job failed: {}", id, e);
                    if let Some(job) = state.write().await.get_mut(&id) {
                        job.status = ReplayStatus::Error;
                        job.error_message = Some(format!("Processing failed: {e}"));
                    }
                }
                Err(_) => {
                    tracing::error!("[{}] Job timed out after {} seconds", id, JOB_TIMEOUT_SECS);
                    if let Some(job) = state.write().await.get_mut(&id) {
                        job.status = ReplayStatus::Error;
                        job.error_message = Some(format!(
                            "Job timed out: replay took longer than {} minutes to process.",
                            JOB_TIMEOUT_SECS / 60
                        ));
                    }
                }
            }
        } else {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Processes a single job. Returns an error if any step fails.
async fn process_job(state: &Arc<RwLock<BTreeMap<Ulid, Replay>>>, id: Ulid) -> anyhow::Result<()> {
    tracing::info!("[{}] Starting job processing...", id);

    // Update status to Recording
    state
        .write()
        .await
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Job was removed from queue unexpectedly"))?
        .status = ReplayStatus::Recording;

    let tmp_dir =
        tempdir().map_err(|e| anyhow::anyhow!("Failed to create temporary directory: {e}"))?;
    tracing::debug!("[{}] Created temp directory: {:?}", id, tmp_dir.path());

    let temp_replay_path = format!("{id}.yrpX");
    let replay_file_path = tmp_dir.path().join(temp_replay_path);
    let replay_data = state
        .write()
        .await
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Job was removed from queue unexpectedly"))?
        .data
        .clone();

    // Write replay_file to a temporary file
    let replay_size = replay_data.len();
    tokio::fs::write(&replay_file_path, &replay_data)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to write replay file to disk: {e}"))?;
    tracing::info!(
        "[{}] Wrote replay file ({} bytes) to {:?}",
        id,
        replay_size,
        replay_file_path
    );

    // Prepare and launch EDOPro
    tracing::info!("[{}] Launching EDOPro in replay mode...", id);
    let replay_path_str = replay_file_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid replay file path"))?;
    let mut edopro_process = launch_edopro(replay_path_str)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to launch EDOPro: {e}"))?;

    // Wait for EDOPro to initialize
    sleep(Duration::from_millis(50)).await;

    // Start recording the display
    tracing::info!("[{}] Starting display recording...", id);
    let output_file_name = format!("{id}.mp4");
    let output_file = tmp_dir.path().join(output_file_name);
    let output_path_str = output_file
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid output file path"))?;
    let mut ffmpeg_child = record_display(output_path_str)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start video recording: {e}"))?;

    // Wait for EDOPro to exit
    let edopro_exit_status = edopro_process
        .wait()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to wait for EDOPro process: {e}"))?;

    // Stop the recording
    tracing::info!("[{}] Replay finished. Stopping display recording...", id);
    let pid = ffmpeg_child
        .id()
        .ok_or_else(|| anyhow::anyhow!("Video recording process exited unexpectedly"))?;
    _ = kill(unistd::Pid::from_raw(pid.cast_signed()), SIGINT);
    _ = ffmpeg_child.wait().await;

    // Check if EDOPro exited with an error (e.g., invalid replay file)
    if !edopro_exit_status.success() {
        let code = edopro_exit_status
            .code()
            .map(|c| format!(" (exit code {c})"))
            .unwrap_or_default();
        return Err(anyhow::anyhow!(
            "EDOPro exited with an error{code}. The replay file may be invalid or corrupted."
        ));
    }

    // Trim black frames from the video
    tracing::info!("[{}] Trimming black frames from video...", id);
    let trimmed_file_name = format!("{id}_trimmed.mp4");
    let trimmed_file = tmp_dir.path().join(trimmed_file_name);
    let trimmed_path_str = trimmed_file
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid trimmed file path"))?;

    trim_black_frames(output_path_str, trimmed_path_str)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to trim black frames: {e}"))?;

    // Load trimmed file into memory and clear replay data to free memory
    let video_data = tokio::fs::read(&trimmed_file)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read trimmed video file: {e}"))?;
    let video_size = video_data.len();
    tracing::info!(
        "[{}] Video trimmed successfully. Size: {} bytes ({:.2} MB).",
        id,
        video_size,
        video_size as f64 / (1024.0 * 1024.0)
    );

    let mut lock = state.write().await;
    let job = lock
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Job was removed from queue unexpectedly"))?;
    job.video = Some(video_data.into());
    job.data = Bytes::new(); // Clear replay data - no longer needed
    job.status = ReplayStatus::Done;

    tracing::debug!("[{}] Job state updated to Done, replay data cleared.", id);
    Ok(())
}

/// Background task that periodically removes stale jobs.
///
/// Jobs are considered stale if they are `Done` or `Error` and older than [`JOB_TTL_SECS`].
/// This prevents memory from growing unbounded if clients never download their videos.
pub async fn cleanup(state: Arc<RwLock<BTreeMap<Ulid, Replay>>>) -> ! {
    tracing::info!(
        "Cleanup task started. Interval: {} seconds, Job TTL: {} seconds.",
        CLEANUP_INTERVAL_SECS,
        JOB_TTL_SECS
    );

    loop {
        sleep(Duration::from_secs(CLEANUP_INTERVAL_SECS)).await;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut lock = state.write().await;
        let before_count = lock.len();

        // Retain only jobs that are either:
        // - Still being processed (Queued or Recording)
        // - Completed/errored but not yet expired
        lock.retain(|id, job| {
            if job.status == ReplayStatus::Queued || job.status == ReplayStatus::Recording {
                return true;
            }

            let job_timestamp_ms = id.timestamp_ms();
            let age_secs = now.saturating_sub(job_timestamp_ms) / 1000;
            age_secs < JOB_TTL_SECS
        });

        let removed_count = before_count.saturating_sub(lock.len());
        if removed_count > 0 {
            tracing::info!(
                "Cleanup: removed {} stale job(s). Remaining jobs: {}.",
                removed_count,
                lock.len()
            );
        } else {
            tracing::debug!(
                "Cleanup: no stale jobs to remove. Current jobs: {}.",
                lock.len()
            );
        }
    }
}
