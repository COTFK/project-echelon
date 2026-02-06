//! Functions to spawn outside processes - EDOPro, Xvfb and ffmpeg.

use std::process::Stdio;
use tokio::process::Child;
use tokio::process::Command as TokioCommand;

/// The screen width, passed to EDOPro, Xvfb and ffmpeg.
pub const SCREEN_WIDTH: u32 = 1556;

/// The screen height, passed to EDOPro, Xvfb and ffmpeg.
pub const SCREEN_HEIGHT: u32 = 1000;

/// Screen recording offset, to avoid recording the EDOPro sidebar.
pub const SIDEBAR_OFFSET: u32 = 456;

/// Display ID for the Xvfb instance.
pub const DISPLAY_ID: &str = ":99";

/// Launch EDOPro with the given replay file.
pub async fn launch_edopro(replay_file_path: &str) -> anyhow::Result<Child> {
    let edopro_path = std::env::var("EDOPRO_PATH")?;

    tracing::debug!(
        "Launching EDOPro from '{}' with replay '{}' on display {}",
        edopro_path,
        replay_file_path,
        DISPLAY_ID
    );

    let child = TokioCommand::new(edopro_path)
        .args(["-i-want-to-be-admin", "-replay", replay_file_path, "-q"])
        .env("DISPLAY", DISPLAY_ID)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    tracing::debug!("EDOPro process started with PID: {:?}", child.id());
    Ok(child)
}

/// Start recording the display using ffmpeg.
pub async fn record_display(output_file: &str) -> anyhow::Result<Child> {
    let recording_width = SCREEN_WIDTH - SIDEBAR_OFFSET;
    tracing::debug!(
        "Starting ffmpeg recording: {}x{} from display {} to '{}'",
        recording_width,
        SCREEN_HEIGHT,
        DISPLAY_ID,
        output_file
    );

    let child = TokioCommand::new("ffmpeg")
        .args([
            "-f",
            "x11grab",
            "-draw_mouse",
            "0",
            "-framerate",
            "30",
            "-s",
            &format!("{}x{}", recording_width, SCREEN_HEIGHT),
            "-i",
            &format!("{DISPLAY_ID}.0+{SIDEBAR_OFFSET},0"),
            "-c:v",
            "libx264",
            "-crf",
            "23",
            "-preset",
            "medium",
            "-g",
            "240",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "faststart",
            "-y",
            output_file,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    tracing::debug!("ffmpeg recording started with PID: {:?}", child.id());
    Ok(child)
}

/// Trim black frames from the start and end of a video file.
/// Analyzes the video using ffmpeg's blackdetect filter to find black regions.
pub async fn trim_black_frames(input_file: &str, output_file: &str) -> anyhow::Result<()> {
    tracing::debug!(
        "Trimming black frames from '{}' to '{}'",
        input_file,
        output_file
    );

    // Get total duration
    let duration_output = TokioCommand::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            input_file,
        ])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get video duration: {e}"))?;

    let duration_str = String::from_utf8_lossy(&duration_output.stdout);
    let total_duration: f64 = match duration_str.trim().parse() {
        Ok(d) => d,
        Err(_) => {
            tracing::warn!("Could not parse duration, skipping trim");
            // If we can't get duration, just copy the original video
            tokio::fs::copy(input_file, output_file)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to copy video: {e}"))?;
            return Ok(());
        }
    };

    tracing::debug!("Video duration: {:.2}s", total_duration);

    // Use ffmpeg with blackdetect to find black frames
    let brightness_output = TokioCommand::new("ffmpeg")
        .args([
            "-i",
            input_file,
            "-vf",
            "fps=10,blackdetect=d=0.5:pic_th=0.95:pix_th=0.1",
            "-f",
            "null",
            "-",
        ])
        .output()
        .await;

    let mut actual_trim_start = 0.0;
    let mut actual_trim_end = total_duration;

    if let Ok(output) = brightness_output {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse blackdetect output to find black regions
        let mut black_regions: Vec<(f64, f64)> = Vec::new();

        for line in stderr.lines() {
            if line.contains("black_start:") && line.contains("black_end:") {
                // Extract black_start value
                if let Some(start_idx) = line.find("black_start:")
                    && let Ok(start) = line[start_idx + 12..]
                        .split_whitespace()
                        .next()
                        .unwrap_or("0")
                        .parse::<f64>()
                {
                    // Extract black_end value
                    if let Some(end_idx) = line.find("black_end:")
                        && let Ok(end) = line[end_idx + 10..]
                            .split_whitespace()
                            .next()
                            .unwrap_or("0")
                            .parse::<f64>()
                    {
                        black_regions.push((start, end));
                        tracing::debug!("Black region: {:.2}s - {:.2}s", start, end);
                    }
                }
            }
        }

        // Sort regions by start time
        black_regions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Determine trim boundaries from black regions
        if !black_regions.is_empty() {
            // Trim from start: only skip if the first black region starts at the very beginning
            let (first_start, first_end) = black_regions[0];
            if first_start < 0.5 {
                // First black region is at the start, skip past it with a small buffer (but cap at 5 seconds)
                actual_trim_start = (first_end + 0.5).min(5.0);
                tracing::debug!(
                    "Found leading black region, trimming from {:.2}s",
                    actual_trim_start
                );
            }

            // Trim from end: check for black regions that are near the end of the video
            // If any black region's END is within 1 second of the video end, trim from its START
            for (start, end) in &black_regions {
                if (total_duration - end).abs() < 1.0 {
                    // This black region extends to the very end
                    actual_trim_end = *start;
                    tracing::debug!("Found trailing black region, trimming to {:.2}s", start);
                    break;
                }
            }

            tracing::info!(
                "Detected {} black region(s), trimming from {:.2}s to {:.2}s",
                black_regions.len(),
                actual_trim_start,
                actual_trim_end
            );
        }
    }

    // Validate trim boundaries
    if actual_trim_start >= actual_trim_end {
        tracing::warn!(
            "Trim boundaries invalid (start={:.2}s >= end={:.2}s), keeping original video",
            actual_trim_start,
            actual_trim_end
        );
        tokio::fs::copy(input_file, output_file)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to copy video: {e}"))?;
        return Ok(());
    }

    let trim_duration = actual_trim_end - actual_trim_start;

    tracing::info!(
        "Trimming video: {:.2}s → {:.2}s (duration: {:.2}s, original: {:.2}s)",
        actual_trim_start,
        actual_trim_end,
        trim_duration,
        total_duration
    );

    // Use ffmpeg to trim the video
    let trim_output = TokioCommand::new("ffmpeg")
        .args([
            "-i",
            input_file,
            "-ss",
            &format!("{:.3}", actual_trim_start),
            "-to",
            &format!("{:.3}", actual_trim_end),
            "-c:v",
            "libx264",
            "-crf",
            "23",
            "-preset",
            "medium",
            "-c:a",
            "aac",
            "-movflags",
            "faststart",
            "-y",
            output_file,
        ])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to trim video: {e}"))?;

    if !trim_output.status.success() {
        let stderr = String::from_utf8_lossy(&trim_output.stderr);
        return Err(anyhow::anyhow!("ffmpeg trim failed: {}", stderr));
    }

    tracing::info!("Video trimmed successfully to '{}'", output_file);
    Ok(())
}

/// Starts an `xvfb` instance and returns it as a [`tokio::process::Child`].
pub async fn start_xvfb() -> anyhow::Result<Child> {
    tracing::info!(
        "Starting Xvfb on display {} with resolution {}x{}x24",
        DISPLAY_ID,
        SCREEN_WIDTH,
        SCREEN_HEIGHT
    );

    let xvfb = TokioCommand::new("Xvfb")
        .args([
            DISPLAY_ID,
            "-screen",
            "0",
            &format!("{SCREEN_WIDTH}x{SCREEN_HEIGHT}x24"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    tracing::info!("Xvfb started with PID {:?}", xvfb.id());
    Ok(xvfb)
}
