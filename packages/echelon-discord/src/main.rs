use serenity::all::OnlineStatus;
use serenity::all::Ready;
use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::env;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

mod api;
use api::{ReplayStatus, download_video, get_replay_status, get_server_url, upload_file};

// Configuration constants
const POLL_INTERVAL_SECS: u64 = 5;
const STALE_STATUS_THRESHOLD_SECS: u64 = 60;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore bot's own messages
        if msg.author.bot {
            return;
        }

        // Check if message is a DM or bot mention
        let is_dm = msg.guild_id.is_none();
        let bot_id = ctx.cache.current_user().id;
        let is_mention = msg.mentions.iter().any(|u| u.id == bot_id)
            || msg.content.contains(&format!("<@{}>", bot_id))
            || msg.content.contains(&format!("<@!{}>", bot_id));

        if !is_dm && !is_mention {
            return;
        }

        // Check for .yrpX file attachments or mentions in content
        let has_yrpx = msg
            .attachments
            .iter()
            .any(|a| a.filename.ends_with(".yrpX"))
            || msg.content.contains(".yrpX");

        if !has_yrpx {
            if let Err(e) = msg
                .reply(&ctx, "👋 Hi! Send me a .yrpX file to get started.")
                .await
            {
                error!("Failed to send help message: {e}");
            }
            return;
        }

        // Process all .yrpX file attachments
        for attachment in &msg.attachments {
            if !attachment.filename.ends_with(".yrpX") {
                continue;
            }

            self.process_replay(&ctx, &msg, attachment).await;
        }
    }

    async fn ready(&self, ctx: Context, _: Ready) {
        info!("✅ Bot is online!");

        let activity = serenity::all::ActivityData::custom("Tag/message me with a replay!");
        ctx.set_presence(Some(activity), OnlineStatus::Online);
    }
}

impl Handler {
    /// Processes a single replay file attachment.
    async fn process_replay(
        &self,
        ctx: &Context,
        msg: &Message,
        attachment: &serenity::model::prelude::Attachment,
    ) {
        debug!("Processing file: {}", attachment.filename);

        // Download the file from Discord
        match attachment.download().await {
            Ok(data) => {
                debug!("Downloaded {} bytes", data.len());
                self.upload_and_monitor(ctx, msg, data).await;
            }
            Err(e) => {
                error!("Failed to download attachment: {e}");
                let error_msg = if e.to_string().contains("timeout") {
                    "❌ Download timed out. The file might be too large or the connection is slow. Please try again."
                } else {
                    "❌ Failed to download the file from Discord. Please check the file size and try again."
                };
                let _ = msg.reply(ctx, error_msg).await;
            }
        }
    }

    /// Uploads a replay file to the echelon server and spawns a monitoring task.
    async fn upload_and_monitor(&self, ctx: &Context, msg: &Message, data: Vec<u8>) {
        let server_url = get_server_url();
        let upload_url = format!("{}/upload", server_url);

        match upload_file(&upload_url, &data).await {
            Ok(id) => {
                // Acknowledge the upload
                let _ = msg
                    .reply(ctx, format!("[`{}`] 📋 Replay queued!", id))
                    .await;

                // Spawn background task to monitor replay status
                let channel_id = msg.channel_id;
                let http = ctx.http.clone();
                tokio::spawn(monitor_replay(server_url, id, channel_id, http));
            }
            Err(e) => {
                error!("Failed to upload replay: {e}");
                let error_msg = if e.contains("500") {
                    "❌ Server error occurred. The processing service is experiencing issues. Please try again in a few moments."
                } else if e.contains("Request failed") {
                    "❌ Failed to reach the processing server. Please check your internet connection and try again."
                } else {
                    &format!("❌ Upload failed: {e}")
                };
                let _ = msg.reply(ctx, error_msg).await;
            }
        }
    }
}

/// Formats a replay status into a user-friendly message.
fn format_status(status: &ReplayStatus) -> String {
    match status {
        ReplayStatus::Queued { position } => {
            format!("⏳ Queued at position {position}")
        }
        ReplayStatus::Processing => "🔄 Currently processing...".to_string(),
        ReplayStatus::Done => "✅ Replay is ready!".to_string(),
        ReplayStatus::Error { message } => format!("❌ Error: {message}"),
        ReplayStatus::NotFound { message } => format!("❓ Not found: {message}"),
    }
}

/// Monitors the status of a replay and sends updates to the Discord channel.
///
/// Polls the server every POLL_INTERVAL_SECS and sends updates when:
/// - The status changes (e.g., queued -> processing)
/// - STALE_STATUS_THRESHOLD_SECS have passed without a change (shows bot is alive)
async fn monitor_replay(
    server_url: String,
    id: String,
    channel_id: serenity::all::ChannelId,
    http: std::sync::Arc<serenity::http::Http>,
) {
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));
    let mut last_status: Option<ReplayStatus> = None;
    let mut last_update = Instant::now();

    loop {
        ticker.tick().await;

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
                    send_video_message(&channel_id, &http, &server_url, &id).await;
                    break;
                }

                // Send status update for other statuses
                if should_update {
                    let message = format!("[`{id}`] {}", format_status(&status));
                    if let Err(e) = channel_id.say(&http, &message).await {
                        error!("Failed to send status update: {e}");
                    }
                    last_update = Instant::now();
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

/// Sends the completed replay video to the Discord channel.
async fn send_video_message(
    channel_id: &serenity::all::ChannelId,
    http: &std::sync::Arc<serenity::http::Http>,
    server_url: &str,
    id: &str,
) {
    match download_video(server_url, id).await {
        Ok(video_data) => {
            let filename = format!("{id}.mp4");
            match channel_id
                .send_message(
                    http,
                    serenity::builder::CreateMessage::new()
                        .content(format!("[`{id}`] ✅ Replay is ready!"))
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
    let intents = GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    if let Err(why) = client.start().await {
        error!("Client error: {why}");
    }
}
