use serenity::all::{
    ChannelId, CommandInteraction, Interaction, MessageId, OnlineStatus, Ready, UserId,
};
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::prelude::*;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

mod api;
use api::{ReplayStatus, download_video, get_replay_status, get_server_url, upload_file};

type Http = Arc<serenity::http::Http>;

// Configuration constants
const POLL_INTERVAL_PROCESSING_SECS: u64 = 3; // Poll every 3s during processing
const POLL_INTERVAL_DEFAULT_SECS: u64 = 10; // Poll every 10s for other states
const STALE_STATUS_THRESHOLD_SECS: u64 = 60;

/// Discord's file size limit in bytes (10MB for non-Nitro users)
const DISCORD_FILE_LIMIT_BYTES: usize = 10 * 1024 * 1024;

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
    fn extract_file_attachment(
        command: &CommandInteraction,
    ) -> Option<serenity::model::prelude::Attachment> {
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

        if !attachment.filename.ends_with(".yrpX") {
            Self::respond_with_error(ctx, command, "❌ Please upload a `.yrpX` replay file").await;
            return;
        }

        match attachment.download().await {
            Ok(data) => {
                let server_url = get_server_url();
                let upload_url = format!("{}/upload", server_url);

                match upload_file(&upload_url, &data).await {
                    Ok(id) => {
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

                        match command.get_response(&ctx.http).await {
                            Ok(status_msg) => {
                                let channel_id = command.channel_id;
                                let http = ctx.http.clone();
                                let requester_id = command.user.id;

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
                                Self::respond_with_error(
                                    ctx,
                                    command,
                                    &format!("[`{id}`] ⚠️ Replay was queued but we couldn't start monitoring. Check back later or try again."),
                                ).await;
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

/// Formats a replay status into a user-friendly message with optional animation frame.
fn format_status(status: &ReplayStatus, animation_frame: Option<usize>) -> String {
    match status {
        ReplayStatus::Queued { position } => {
            format!("⏳ Queued at position {position}")
        }
        ReplayStatus::Processing => {
            let spinner = ['⠋', '⠙', '⠴', '⠦'][animation_frame.unwrap_or(0) % 4];
            format!("{spinner} Currently processing...")
        }
        ReplayStatus::Done => "✅ Replay is ready!".to_string(),
        ReplayStatus::Error { message } => format!("❌ Error: {message}"),
        ReplayStatus::NotFound { message } => format!("❓ Not found: {message}"),
    }
}

async fn monitor_replay(
    server_url: String,
    id: String,
    channel_id: ChannelId,
    status_msg_id: MessageId,
    requester_id: UserId,
    http: Http,
) {
    let mut last_status: Option<ReplayStatus> = None;
    let mut last_update = Instant::now();
    let mut update_count: usize = 0;

    loop {
        let poll_interval_secs = if let Some(ref status) = last_status {
            match status {
                ReplayStatus::Processing => POLL_INTERVAL_PROCESSING_SECS,
                _ => POLL_INTERVAL_DEFAULT_SECS,
            }
        } else {
            POLL_INTERVAL_PROCESSING_SECS
        };

        tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;

        match get_replay_status(&server_url, &id).await {
            Ok(status) => {
                let status_changed = last_status
                    .as_ref()
                    .is_none_or(|last| format!("{:?}", last) != format!("{:?}", status));

                let should_update = status_changed
                    || last_update.elapsed() >= Duration::from_secs(STALE_STATUS_THRESHOLD_SECS);

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

                if let ReplayStatus::Error { message } = &status {
                    // Update status message with the error before breaking
                    let error_message = format!(
                        "[`{id}`] ❌ Processing failed: {message}"
                    );
                    if let Err(e) = channel_id
                        .edit_message(
                            &http,
                            status_msg_id,
                            serenity::builder::EditMessage::new().content(&error_message),
                        )
                        .await
                    {
                        error!("Failed to update status message with error: {e}");
                        // Try sending a new message as fallback
                        let _ = channel_id.say(&http, &error_message).await;
                    }
                    break;
                }

                last_status = Some(status);
            }
            Err(e) => {
                warn!("Failed to get replay status: {e}");
                if last_update.elapsed() >= Duration::from_secs(STALE_STATUS_THRESHOLD_SECS) {
                    let error_message = if e.contains("404") || e.contains("Not found") {
                        format!(
                            "[`{id}`] ❓ Replay not found on server. It may have expired or been deleted."
                        )
                    } else if e.contains("Request failed") {
                        format!(
                            "[`{id}`] ⚠️ Lost connection to processing server. Please try again later."
                        )
                    } else {
                        format!("[`{id}`] ⚠️ Unable to get status updates: {e}")
                    };
                    // Try to edit the status message first
                    if let Err(edit_err) = channel_id
                        .edit_message(
                            &http,
                            status_msg_id,
                            serenity::builder::EditMessage::new().content(&error_message),
                        )
                        .await
                    {
                        error!("Failed to edit status message: {edit_err}");
                        // Fallback to sending a new message
                        if let Err(e) = channel_id.say(&http, &error_message).await {
                            error!("Failed to send error update: {e}");
                        }
                    }
                    break;
                }
            }
        }
    }
}

async fn send_video_message(
    channel_id: &ChannelId,
    http: &Http,
    server_url: &str,
    id: &str,
    requester_id: UserId,
    status_msg_id: MessageId,
) {
    if let Err(e) = channel_id.delete_message(http, status_msg_id).await {
        warn!("Failed to delete status message: {e}");
    }

    match download_video(server_url, id).await {
        Ok(video_data) => {
            let video_size = video_data.len();
            let video_size_mb = video_size as f64 / (1024.0 * 1024.0);

            // Check if video exceeds Discord's file size limit
            if video_size > DISCORD_FILE_LIMIT_BYTES {
                info!(
                    "Video for replay {id} is too large for Discord ({:.2} MB)",
                    video_size_mb
                );
                let msg = format!(
                    "{} [`{id}`] ✅ Replay processed successfully! \n\n\
                    However, the video is too large for Discord ({:.1} MB, limit is 10 MB).\n\n\
                    📥 **Download your recording here (available for 1 hour):** <{}/download/{}>",
                    requester_id.mention(),
                    video_size_mb,
                    server_url,
                    id
                );
                if let Err(e) = channel_id.say(http, &msg).await {
                    error!("Failed to send too-large notification: {e}");
                }
                return;
            }

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
                Ok(_) => info!("Sent video for replay {id} ({:.2} MB)", video_size_mb),
                Err(e) => {
                    error!("Failed to send video message: {e}");
                    // Check if the error is due to file size (Discord might reject it)
                    let error_str = e.to_string();
                    let msg = if error_str.contains("40005") || error_str.contains("too large") {
                        format!(
                            "{} [`{id}`] ✅ Replay processed successfully! \n\n\
                            However, the video is too large for Discord ({:.1} MB, limit is 10 MB).\n\n\
                            📥 **Download your recording here (available for 1 hour):** <{}/download/{}>",
                            requester_id.mention(),
                            video_size_mb,
                            server_url,
                            id
                        )
                    } else {
                        format!(
                            "{} [`{id}`] ✅ Replay processed successfully! \n\n\
                            However, we failed to send the video through Discord.\n\n\
                            📥 **Download your recording here (available for 1 hour):** <{}/download/{}>",
                            requester_id.mention(),
                            server_url,
                            id
                        )
                    };
                    if let Err(send_err) = channel_id.say(http, &msg).await {
                        error!("Failed to send fallback message: {send_err}");
                    }
                }
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
