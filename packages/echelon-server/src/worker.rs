//! The [`worker()`] and [`cleanup()`] functions - background tasks for processing and maintenance.

use crate::commands::capture_audio_pipe;
use crate::commands::create_named_pipe;
use crate::commands::get_video_duration_secs;
use crate::commands::launch_edopro;
use crate::commands::mux_audio_into_video;
use crate::commands::record_display;
use crate::types::Replay;
use crate::types::ReplayStatus;
use nix::sys::signal::{Signal, kill};
use nix::unistd;
use std::collections::BTreeMap;
use std::path::Path;
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

/// Maximum time allowed for processing a single job (1 hour).
const JOB_TIMEOUT_SECS: u64 = 3600;

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
    let start_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
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

    // Get replay details
    let mut lock = state.write().await;
    let job = lock
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Job was removed from queue unexpectedly"))?;
    let replay_data = job
        .data
        .clone()
        .ok_or_else(|| anyhow::anyhow!("No replay data found in job? This should not happen."))?;
    let replay_config = job.config.clone();
    drop(lock);

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

    // Refresh config
    let edopro_path = std::env::var("EDOPRO_PATH")?;
    let config_dir = Path::new(&edopro_path).join("config");
    let backup_path = config_dir.join("system.conf.bak");
    let config_path = config_dir.join("system.conf");
    let _ = tokio::fs::copy(backup_path, config_path.clone()).await;

    // Update config with settings
    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read config file: {e}"))?;

    // Set top-down view
    let updated_content = if replay_config.top_down_view {
        config_content.replace("topdown_view = 0", "topdown_view = 1")
    } else {
        config_content.replace("topdown_view = 1", "topdown_view = 0")
    };

    tokio::fs::write(&config_path, updated_content)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to write config file: {e}"))?;
    tracing::debug!("[{}] Updated config: topdown_view = 1", id);

    // Prepare and launch EDOPro
    tracing::info!("[{}] Launching EDOPro in replay mode...", id);
    let replay_path_str = replay_file_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid replay file path"))?;
    let output_file_name = format!("{id}.mp4");
    let output_file = tmp_dir.path().join(output_file_name);
    let output_path_str = output_file
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid output file path"))?;
    // Intermediate video-only file (no audio), muxed into output_file afterwards
    let video_only_file = tmp_dir.path().join(format!("{id}_video.mp4"));
    let video_only_str = video_only_file
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid video-only file path"))?;
    // Raw PCM audio captured from the audio pipe
    let raw_audio_file = tmp_dir.path().join(format!("{id}.pcm"));
    let raw_audio_str = raw_audio_file
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid raw audio file path"))?;
    let frame_pipe_path = tmp_dir.path().join("frames.pipe");
    let frame_pipe_str = frame_pipe_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid frame pipe path"))?;
    let audio_pipe_path = tmp_dir.path().join("audio.pipe");
    let audio_pipe_str = audio_pipe_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid audio pipe path"))?;

    create_named_pipe(frame_pipe_str)
        .map_err(|e| anyhow::anyhow!("Failed to create frame pipe: {e}"))?;
    create_named_pipe(audio_pipe_str)
        .map_err(|e| anyhow::anyhow!("Failed to create audio pipe: {e}"))?;

    // Start recording from the pipes before launching EDOPro to avoid blocking on FIFO open
    tracing::info!("[{}] Starting frame and audio recording...", id);
    let mut ffmpeg_child =
        record_display(video_only_str, frame_pipe_str, replay_config.video_preset)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start video recording: {e}"))?;

    // Capture audio pipe to a raw PCM file concurrently with video recording.
    // Keeping this separate from ffmpeg avoids FIFO ordering deadlocks.
    let audio_handle = tokio::spawn(capture_audio_pipe(
        audio_pipe_str.to_owned(),
        raw_audio_str.to_owned(),
        id.to_string(),
    ));

    let edopro_log_path = tmp_dir.path().join("edopro.stderr.log");
    let edopro_log_str = edopro_log_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid EDOPro log file path"))?;
    let mut edopro_process = launch_edopro(
        replay_path_str,
        frame_pipe_str,
        audio_pipe_str,
        edopro_log_str,
        replay_config.swap_players,
        replay_config.game_speed,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to launch EDOPro: {e}"))?;

    // Wait for EDOPro to exit
    let edopro_exit_status = edopro_process
        .wait()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to wait for EDOPro process: {e}"))?;

    // Wait for ffmpeg to finish after EDOPro closes the frame pipe
    tracing::info!(
        "[{}] Replay finished. Waiting for ffmpeg to finalize...",
        id
    );
    // Give ffmpeg enough time to properly finalize the MP4 file (critical for -movflags faststart)
    const FFMPEG_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);
    let ffmpeg_wait = tokio::time::timeout(FFMPEG_SHUTDOWN_TIMEOUT, ffmpeg_child.wait()).await;
    match ffmpeg_wait {
        Ok(Ok(status)) if !status.success() => {
            tracing::warn!("[{}] ffmpeg exited with status: {:?}", id, status.code());
        }
        Ok(Err(e)) => {
            return Err(anyhow::anyhow!("Failed to wait for ffmpeg: {e}"));
        }
        Err(_) => {
            tracing::error!(
                "[{}] ffmpeg did not finish within {}s",
                id,
                FFMPEG_SHUTDOWN_TIMEOUT.as_secs()
            );
            if let Some(pid) = ffmpeg_child.id() {
                _ = kill(unistd::Pid::from_raw(pid.cast_signed()), Signal::SIGTERM);
            }
            return Err(anyhow::anyhow!("ffmpeg timeout - video may be corrupted"));
        }
        _ => {}
    }

    // Check if EDOPro exited with an error (e.g., invalid replay file)
    if !edopro_exit_status.success() {
        let stderr = tokio::fs::read_to_string(&edopro_log_path)
            .await
            .unwrap_or_default();
        let code = edopro_exit_status
            .code()
            .map(|c| format!(" (exit code {c})"))
            .unwrap_or_default();
        if stderr.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "EDOPro exited with an error{code}. Logs were empty. Headless framebuffer mode may be unsupported in this build/environment."
            ));
        }
        return Err(anyhow::anyhow!(
            "EDOPro exited with an error{code}. Logs:\n{}",
            stderr
        ));
    }

    // Ensure audio capture finished before muxing
    audio_handle
        .await
        .map_err(|e| anyhow::anyhow!("Audio capture task panicked: {e}"))?
        .map_err(|e| anyhow::anyhow!("Audio capture failed: {e}"))?;

    // Mux video + audio into the final output (video stream is copied, no re-encode)
    tracing::info!("[{}] Muxing audio into video...", id);
    mux_audio_into_video(video_only_str, raw_audio_str, output_path_str)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to mux audio into video: {e}"))?;

    // Load output file into memory and clear replay data to free memory
    let video_data = tokio::fs::read(&output_file)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read trimmed video file: {e}"))?;

    let mut lock = state.write().await;
    let job = lock
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Job was removed from queue unexpectedly"))?;
    job.video = Some(video_data.into());
    job.data = None; // Clear replay data - no longer needed
    job.status = ReplayStatus::Done;

    let done_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let elapsed_secs = done_ms.saturating_sub(start_ms) as f64 / 1000.0;
    let video_secs = get_video_duration_secs(output_path_str)
        .await
        .unwrap_or(job.estimated_duration.unwrap_or(0.0));
    let overhead_secs = elapsed_secs - video_secs;
    let ratio = if video_secs > 0.0 {
        elapsed_secs / video_secs
    } else {
        0.0
    };
    tracing::info!(
        "[{}] Processing time {:.2}s vs video {:.2}s: overhead {:.2}s, ratio {:.2}x",
        id,
        elapsed_secs,
        video_secs,
        overhead_secs,
        ratio
    );

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
