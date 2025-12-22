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
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

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
