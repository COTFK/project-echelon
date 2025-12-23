use serenity::all::OnlineStatus;
use serenity::all::Ready;
use serenity::all::{CommandInteraction, Interaction};
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::prelude::*;
use std::env;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

mod api;
use api::{ReplayStatus, download_video, get_replay_status, get_server_url, upload_file};

// Configuration constants
const POLL_INTERVAL_PROCESSING_SECS: u64 = 3; // Poll every 5s during processing
const POLL_INTERVAL_DEFAULT_SECS: u64 = 10; // Poll every 10s for other states
const STALE_STATUS_THRESHOLD_SECS: u64 = 60;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            if command.data.name == "echelon" {
                self.handle_echelon_command(&ctx, &command).await;
            }
        }
    }

    async fn ready(&self, ctx: Context, _: Ready) {
        info!("✅ Bot is online!");

        // Register slash command
        match serenity::all::Command::create_global_command(&ctx.http, {
            serenity::builder::CreateCommand::new("echelon")
                .description("Convert your EDOPro replay to video")
                .add_option(
                    serenity::builder::CreateCommandOption::new(
                        serenity::all::CommandOptionType::SubCommand,
                        "convert",
                        "Upload a replay file to convert",
                    )
                    .add_sub_option(
                        serenity::builder::CreateCommandOption::new(
                            serenity::all::CommandOptionType::Attachment,
                            "file",
                            "Your .yrpX replay file (max 10MB)",
                        )
                        .required(true),
                    ),
                )
        })
        .await
        {
            Ok(_) => info!("✅ Slash command '/echelon' registered"),
            Err(e) => error!("Failed to register slash command: {e}"),
        }

        let activity = serenity::all::ActivityData::custom("Use /echelon to record a replay!");
        ctx.set_presence(Some(activity), OnlineStatus::Online);
    }
}

impl Handler {
    /// Extracts the replay file attachment from a slash command.
    fn extract_file_attachment(command: &CommandInteraction) -> Option<serenity::model::prelude::Attachment> {
        command.data.options.iter().find_map(|opt| {
            if opt.name == "convert" {
                // This is a subcommand, need to get the nested options
                if let serenity::all::CommandDataOptionValue::SubCommand(sub_opts) = &opt.value {
                    // Find the file option within the subcommand options
                    sub_opts.iter().find_map(|sub_opt| {
                        if sub_opt.name == "file" {
                            match &sub_opt.value {
                                serenity::all::CommandDataOptionValue::Attachment(id) => {
                                    command.data.resolved.attachments.get(id).cloned()
                                }
                                _ => None,
                            }
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Validates that the attachment is a replay file (.yrpX).
    fn validate_replay_file(filename: &str) -> bool {
        filename.ends_with(".yrpX")
    }

    /// Sends an error response to the command interaction.
    async fn respond_with_error(ctx: &Context, command: &CommandInteraction, message: &str) {
        let _ = command
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new().content(message),
            )
            .await;
    }

    /// Handles the /echelon slash command.
    async fn handle_echelon_command(&self, ctx: &Context, command: &CommandInteraction) {
        // Defer the response immediately
        if let Err(e) = command.defer(&ctx.http).await {
            error!("Failed to defer command response: {e}");
            return;
        }

        // Get the file attachment from the convert subcommand
        let file_attachment = Self::extract_file_attachment(command);

        let Some(attachment) = file_attachment else {
            Self::respond_with_error(ctx, command, "❌ No file attachment provided").await;
            return;
        };

        // Validate file extension
        if !Self::validate_replay_file(&attachment.filename) {
            Self::respond_with_error(ctx, command, "❌ Please upload a `.yrpX` replay file").await;
            return;
        }

        // Download the file
        match attachment.download().await {
            Ok(data) => {
                debug!("Downloaded {} bytes via slash command", data.len());

                // Upload to server and get ID
                let server_url = get_server_url();
                let upload_url = format!("{}/upload", server_url);

                match upload_file(&upload_url, &data).await {
                    Ok(id) => {
                        // Send initial response
                        if let Err(e) = command
                            .edit_response(
                                &ctx.http,
                                serenity::builder::EditInteractionResponse::new()
                                    .content(format!("[`{}`] 📋 Replay queued!", id)),
                            )
                            .await
                        {
                            error!("Failed to edit command response: {e}");
                            return;
                        }

                        // Get the response message to update it with status
                        match command.get_response(&ctx.http).await {
                            Ok(status_msg) => {
                                let channel_id = command.channel_id;
                                let http = ctx.http.clone();
                                let requester_id = command.user.id;

                                // Spawn background task to monitor and update the status message
                                tokio::spawn(monitor_replay(
                                    server_url,
                                    id,
                                    channel_id,
                                    status_msg.id,
                                    requester_id,
                                    http,
                                ));
                            }
                            Err(e) => {
                                error!("Failed to get response message: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to upload replay: {e}");
                        let error_msg = if e.contains("500") {
                            "❌ Server error occurred. The processing service is experiencing issues. Please try again in a few moments."
                        } else if e.contains("Request failed") {
                            "❌ Failed to reach the processing server. Please try again; if this error persists, reach out to us for help."
                        } else {
                            "❌ Upload failed. Please try again."
                        };
                        Self::respond_with_error(ctx, command, error_msg).await;
                    }
                }
            }
            Err(e) => {
                error!("Failed to download attachment: {e}");
                let error_msg = if e.to_string().contains("timeout") {
                    "❌ Download timed out. The file might be too large or the connection is slow. Please try again."
                } else {
                    "❌ Failed to download the file. Please check the file size and try again."
                };
                Self::respond_with_error(ctx, command, error_msg).await;
            }
        }
    }
}

/// Returns a braille spinner character for animation based on frame count.
fn get_braille_spinner(frame: usize) -> char {
    const SPINNER: &[char] = &['⠋', '⠙', '⠴', '⠦'];
    SPINNER[frame % SPINNER.len()]
}

/// Formats a replay status into a user-friendly message with optional animation frame.
fn format_status(status: &ReplayStatus, animation_frame: Option<usize>) -> String {
    match status {
        ReplayStatus::Queued { position } => {
            format!("⏳ Queued at position {position}")
        }
        ReplayStatus::Processing => {
            if let Some(frame) = animation_frame {
                format!("{} Currently processing...", get_braille_spinner(frame))
            } else {
                "⠋ Currently processing...".to_string()
            }
        }
        ReplayStatus::Done => "✅ Replay is ready!".to_string(),
        ReplayStatus::Error { message } => format!("❌ Error: {message}"),
        ReplayStatus::NotFound { message } => format!("❓ Not found: {message}"),
    }
}

/// Monitors the status of a replay and sends updates to the Discord channel.
///
/// Polls the server with adaptive intervals based on queue position:
/// - Position 1 (next to process): every 2 seconds
/// - Position > 1 (waiting): every 10 seconds
/// - Processing: every 5 seconds for animation
/// - Other states: every 10 seconds
///
/// Also edits the status message when:
/// - The status changes (e.g., queued -> processing)
/// - STALE_STATUS_THRESHOLD_SECS have passed without a change (shows bot is alive)
async fn monitor_replay(
    server_url: String,
    id: String,
    channel_id: serenity::all::ChannelId,
    status_msg_id: serenity::all::MessageId,
    requester_id: serenity::all::UserId,
    http: std::sync::Arc<serenity::http::Http>,
) {
    let mut last_status: Option<ReplayStatus> = None;
    let mut last_update = Instant::now();
    let mut update_count: usize = 0;

    loop {
        // Determine polling interval based on current status
        let poll_interval_secs = if let Some(ref status) = last_status {
            match status {
                ReplayStatus::Processing => POLL_INTERVAL_PROCESSING_SECS,
                _ => POLL_INTERVAL_DEFAULT_SECS,
            }
        } else {
            // First poll: check quickly
            POLL_INTERVAL_PROCESSING_SECS
        };

        tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;

        match get_replay_status(&server_url, &id).await {
            Ok(status) => {
                // Detect status change by comparing debug output
                let status_changed = last_status
                    .as_ref()
                    .is_none_or(|last| format!("{:?}", last) != format!("{:?}", status));

                let should_update = status_changed
                    || last_update.elapsed() >= Duration::from_secs(STALE_STATUS_THRESHOLD_SECS);

                // Handle completion: download video and send final message
                if matches!(status, ReplayStatus::Done) {
                    send_video_message(
                        &channel_id,
                        &http,
                        &server_url,
                        &id,
                        requester_id,
                        status_msg_id,
                    )
                    .await;
                    break;
                }

                // Send status update for other statuses (edit the existing message)
                // Always animate spinner during Processing, update on status change or stale
                if matches!(status, ReplayStatus::Processing) || should_update {
                    let message =
                        format!("[`{id}`] {}", format_status(&status, Some(update_count)));
                    if let Err(e) = channel_id
                        .edit_message(
                            &http,
                            status_msg_id,
                            serenity::builder::EditMessage::new().content(message),
                        )
                        .await
                    {
                        error!("Failed to edit status update: {e}");
                    }
                    if should_update {
                        last_update = Instant::now();
                    }
                    update_count += 1;
                }

                // Stop monitoring on error
                if matches!(status, ReplayStatus::Error { .. }) {
                    break;
                }

                last_status = Some(status);
            }
            Err(e) => {
                warn!("Failed to get replay status: {e}");
                // Only notify user if we haven't heard from server in a while
                if last_update.elapsed() >= Duration::from_secs(STALE_STATUS_THRESHOLD_SECS) {
                    let status_message = if e.contains("404") || e.contains("Not found") {
                        format!(
                            "[`{id}`] ❓ Replay not found on server. It may have expired or been deleted."
                        )
                    } else if e.contains("Request failed") {
                        format!(
                            "[`{id}`] ⚠️ Lost connection to processing server. It will resume when service is available."
                        )
                    } else {
                        format!("[`{id}`] ⚠️ Unable to get status updates: {e}")
                    };
                    if let Err(e) = channel_id.say(&http, &status_message).await {
                        error!("Failed to send error update: {e}");
                    }
                    break;
                }
            }
        }
    }
}

/// Sends the completed replay video to the Discord channel and deletes the status message.
async fn send_video_message(
    channel_id: &serenity::all::ChannelId,
    http: &std::sync::Arc<serenity::http::Http>,
    server_url: &str,
    id: &str,
    requester_id: serenity::all::UserId,
    status_msg_id: serenity::all::MessageId,
) {
    // Delete the status message
    if let Err(e) = channel_id.delete_message(http, status_msg_id).await {
        warn!("Failed to delete status message: {e}");
    }

    match download_video(server_url, id).await {
        Ok(video_data) => {
            let filename = format!("{id}.mp4");
            match channel_id
                .send_message(
                    http,
                    serenity::builder::CreateMessage::new()
                        .content(format!(
                            "{} [`{id}`] ✅ Replay is ready!",
                            requester_id.mention()
                        ))
                        .add_file(serenity::builder::CreateAttachment::bytes(
                            video_data, filename,
                        )),
                )
                .await
            {
                Ok(_) => info!("Sent video for replay {id}"),
                Err(e) => error!("Failed to send video message: {e}"),
            }
        }
        Err(e) => {
            error!("Failed to download video: {e}");
            let error_msg = if e.contains("404") {
                format!(
                    "[`{id}`] ❌ Video generation failed or has expired. Please re-upload the replay to try again."
                )
            } else if e.contains("Request failed") {
                format!(
                    "[`{id}`] ⚠️ Unable to download the completed video. The processing server is unreachable."
                )
            } else {
                format!("[`{id}`] ⚠️ Replay processed but video download failed: {e}")
            };
            if let Err(e) = channel_id.say(http, &error_msg).await {
                error!("Failed to send final message: {e}");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load environment variables - used for various config purposes
    _ = dotenvy::dotenv();

    info!("Starting Discord bot...");

    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    let intents = GatewayIntents::empty();

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    // Clone shard manager for signal handling
    let shard_manager = client.shard_manager.clone();

    // Spawn signal handler task
    tokio::spawn(async move {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
                info!("Received SIGTERM, shutting down gracefully...");
                shard_manager.shutdown_all().await;
            }
            Err(e) => error!("Failed to set up signal handler: {e}"),
        }
    });

    if let Err(why) = client.start().await {
        error!("Client error: {why}");
    }

    info!("Bot has shut down.");
}
