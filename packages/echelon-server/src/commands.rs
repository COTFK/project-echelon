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

/// PulseAudio UNIX socket path used inside the container.
/// Shared between the server process (which starts PulseAudio) and EDOPro (which connects as a client).
pub const PULSE_SERVER: &str = "unix:/tmp/pulseaudio.socket";

/// Screen recording offset, to avoid recording the EDOPro sidebar.
pub const SIDEBAR_OFFSET: u32 = 456;

/// Create a named FIFO pipe at the given path, replacing any existing file.
pub fn create_named_pipe(pipe_path: &str) -> anyhow::Result<()> {
    let path = Path::new(pipe_path);
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to remove existing pipe: {e}"))?;
    }
    mkfifo(path, Mode::from_bits_truncate(0o600))
        .map_err(|e| anyhow::anyhow!("Failed to create named pipe: {e}"))?;
    Ok(())
}

/// Launch EDOPro with the given replay file.
pub async fn launch_edopro(
    replay_file_path: &str,
    frame_pipe_path: &str,
    audio_pipe_path: &str,
    stderr_log_path: &str,
    swap_players: bool,
    game_speed: f64,
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

    let mut command = TokioCommand::new(Path::new(&edopro_path).join("EDOPro"));
    command.args(["-i-want-to-be-admin", "-replay", replay_file_path, "-q"]);
    command.env("DISPLAY", DISPLAY_ID);
    command.env("PULSE_SERVER", PULSE_SERVER);
    // Propagate the swap flag into the EDOPro process as an environment variable
    if swap_players {
        command.env("EDOPRO_REPLAY_SWAP", "1");
    }

    // Propagate the requested gameplay speed into the EDOPro process so
    // offline rendering can scale simulated time while keeping rendering at 60fps.
    // Only set if it's a positive finite value.
    if game_speed.is_finite() && game_speed > 0.0 {
        command.env("EDOPRO_GAME_SPEED", game_speed.to_string());
    }

    let child = command
        .env("EDOPRO_FRAME_PIPE", frame_pipe_path)
        .env("EDOPRO_AUDIO_PIPE", audio_pipe_path)
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

/// Start recording the display using ffmpeg (video only).
/// Audio is captured concurrently via [`capture_audio_pipe`] and muxed in afterwards.
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
            "-vf",
            "select='gte(n,10)',setpts=N/FRAME_RATE/TB",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-pix_fmt",
            "yuv420p",
            "-y",
            output_file,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    tracing::debug!("ffmpeg recording started with PID: {:?}", child.id());
    Ok(child)
}

/// Capture raw PCM audio from the audio pipe into a file.
/// Runs concurrently with EDOPro/ffmpeg; await the returned handle before muxing.
pub async fn capture_audio_pipe(
    audio_pipe_path: String,
    raw_audio_path: String,
    job_id: String,
) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;

    tracing::debug!(
        "[{}] Audio capture task started, opening '{}'",
        job_id,
        audio_pipe_path
    );

    let mut pipe = tokio::fs::OpenOptions::new()
        .read(true)
        .open(&audio_pipe_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open audio pipe for reading: {e}"))?;

    tracing::debug!("[{}] Audio pipe opened — EDOPro connected. Capturing...", job_id);

    let mut out = tokio::fs::File::create(&raw_audio_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create raw audio file: {e}"))?;

    let total_bytes = tokio::io::copy(&mut pipe, &mut out)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to capture audio data: {e}"))?;

    out.flush()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to flush audio file: {e}"))?;

    // s16le stereo 44100 Hz => 4 bytes per sample
    let approx_secs = total_bytes as f64 / (44100.0 * 4.0);
    tracing::debug!(
        "[{}] Audio capture done. {} bytes (~{:.2}s of audio).",
        job_id, total_bytes, approx_secs
    );

    Ok(())
}

/// Mux an already-encoded video file with a raw PCM audio file into a final MP4.
/// Uses `-c:v copy` so there is no video re-encode — only AAC audio encoding.
pub async fn mux_audio_into_video(
    video_path: &str,
    raw_audio_path: &str,
    output_path: &str,
) -> anyhow::Result<()> {
    tracing::debug!(
        "Muxing '{}' + '{}' -> '{}'",
        video_path,
        raw_audio_path,
        output_path
    );

    let output = TokioCommand::new("ffmpeg")
        .args([
            // Video: copy existing encoded stream, no re-encode
            "-i",
            video_path,
            // Audio: raw PCM s16le stereo 44100 Hz
            "-f",
            "s16le",
            "-ar",
            "44100",
            "-ac",
            "2",
            "-i",
            raw_audio_path,
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
            "-y",
            output_path,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run ffmpeg mux: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "ffmpeg mux failed (exit {:?}):\n{}",
            output.status.code(),
            stderr
        ));
    }
    tracing::debug!("Mux complete: '{}'", output_path);
    Ok(())
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

/// Starts a PulseAudio daemon with a null sink, providing a virtual audio device for EDOPro.
/// Uses a UNIX socket at [`PULSE_SERVER`] so the path is explicit and shared with EDOPro.
pub async fn start_pulseaudio() -> anyhow::Result<Child> {
    tracing::info!("Starting PulseAudio with null sink on {}", PULSE_SERVER);

    let pa = TokioCommand::new("pulseaudio")
        .args([
            "--exit-idle-time=-1",
            "--daemonize=no",
            "--log-level=error",
            // Load a null sink so the mixer has a device to write to even without hardware
            "-L",
            "module-null-sink sink_name=null sink_properties=device.description=Null",
            // Expose on a well-known socket so EDOPro can find it regardless of UID
            "--load=module-native-protocol-unix auth-anonymous=1 socket=/tmp/pulseaudio.socket",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start PulseAudio: {e}"))?;

    // Give PulseAudio time to create the socket before EDOPro tries to connect
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    tracing::info!("PulseAudio started with PID {:?}", pa.id());
    Ok(pa)
}
