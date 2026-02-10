//! Functions to spawn outside processes - EDOPro, Xvfb and ffmpeg.

use nix::sys::stat::Mode;
use nix::unistd::mkfifo;
use std::fs::File;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Child;
use tokio::process::Command as TokioCommand;

/// The screen width, passed to EDOPro, Xvfb and ffmpeg.
pub const SCREEN_WIDTH: u32 = 1556;

/// The screen height, passed to EDOPro, Xvfb and ffmpeg.
pub const SCREEN_HEIGHT: u32 = 1000;

/// Display ID for the Xvfb instance.
pub const DISPLAY_ID: &str = ":99";

/// Screen recording offset, to avoid recording the EDOPro sidebar.
pub const SIDEBAR_OFFSET: u32 = 456;

/// Create a named pipe for raw frame capture.
pub fn create_frame_pipe(pipe_path: &str) -> anyhow::Result<()> {
    let path = Path::new(pipe_path);
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to remove existing frame pipe: {e}"))?;
    }
    mkfifo(path, Mode::from_bits_truncate(0o600))
        .map_err(|e| anyhow::anyhow!("Failed to create frame pipe: {e}"))?;
    Ok(())
}

/// Launch EDOPro with the given replay file.
pub async fn launch_edopro(
    replay_file_path: &str,
    frame_pipe_path: &str,
    stderr_log_path: &str,
) -> anyhow::Result<Child> {
    let edopro_path = std::env::var("EDOPRO_PATH")?;
    let log_file = File::create(stderr_log_path)
        .map_err(|e| anyhow::anyhow!("Failed to create EDOPro log: {e}"))?;
    let log_file_err = log_file
        .try_clone()
        .map_err(|e| anyhow::anyhow!("Failed to clone EDOPro log handle: {e}"))?;

    tracing::debug!(
        "Launching EDOPro from '{}' with replay '{}' on display {}",
        edopro_path,
        replay_file_path,
        DISPLAY_ID
    );

    let mut command = TokioCommand::new(edopro_path);
    command.args(["-i-want-to-be-admin", "-replay", replay_file_path, "-q"]);
    command.env("DISPLAY", DISPLAY_ID);
    let child = command
        .env("EDOPRO_FRAME_PIPE", frame_pipe_path)
        .env("EDOPRO_OFFLINE_RENDER", "1")
        .env("EDOPRO_FRAME_CROP_X", SIDEBAR_OFFSET.to_string())
        .env("EDOPRO_FRAME_CROP_Y", "0")
        .env("EDOPRO_FRAME_CROP_W", (SCREEN_WIDTH - SIDEBAR_OFFSET).to_string())
        .env("EDOPRO_FRAME_CROP_H", SCREEN_HEIGHT.to_string())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()?;

    tracing::debug!("EDOPro process started with PID: {:?}", child.id());
    Ok(child)
}

/// Start recording the display using ffmpeg.
pub async fn record_display(output_file: &str, frame_pipe_path: &str) -> anyhow::Result<Child> {
    let recording_width = SCREEN_WIDTH - SIDEBAR_OFFSET;
    let video_size = format!("{}x{}", recording_width, SCREEN_HEIGHT);
    tracing::debug!(
        "Starting ffmpeg recording: {}x{} from frame pipe '{}' to '{}'",
        recording_width,
        SCREEN_HEIGHT,
        frame_pipe_path,
        output_file
    );

    let child = TokioCommand::new("ffmpeg")
        .args([
            "-f",
            "rawvideo",
            "-pixel_format",
            "bgra",
            "-video_size",
            &video_size,
            "-framerate",
            "60",
            "-i",
            frame_pipe_path,
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
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

/// Returns the duration of a video file in seconds.
pub async fn get_video_duration_secs(input_file: &str) -> anyhow::Result<f64> {
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
    let duration = duration_str
        .trim()
        .parse::<f64>()
        .map_err(|_| anyhow::anyhow!("Failed to parse video duration"))?;
    Ok(duration)
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
