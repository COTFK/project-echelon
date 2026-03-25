use serenity::all::{
    ButtonStyle, ChannelId, CommandInteraction, ComponentInteraction,
    ComponentInteractionDataKind, Interaction, MessageId, OnlineStatus, Ready, UserId,
};
use serenity::async_trait;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::client::{Client, Context, EventHandler};
use serenity::model::prelude::Attachment;
use serenity::prelude::*;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

mod api;
use api::{
    ReplayConfig, ReplayStatus, VideoPreset, create_replay_with_config,
    download_video, get_replay_status, get_server_url, upload_replay, validate_server_url,
};

mod helpers;
use helpers::{translate_api_error, update_status_message, validate_replay_file};

type Http = Arc<serenity::http::Http>;

// Configuration constants
const POLL_INTERVAL_PROCESSING_SECS: u64 = 3; // Poll every 3s during processing
const POLL_INTERVAL_DEFAULT_SECS: u64 = 10; // Poll every 10s for other states
const STALE_STATUS_THRESHOLD_SECS: u64 = 60;
const MAX_MONITORING_DURATION_SECS: u64 = 3660; // Maximum 1 hour of monitoring + 1 minute grace period
const ADVANCED_COMPONENT_PREFIX: &str = "echelon-advanced:";
const ADVANCED_COMPONENT_TTL_SECS: u64 = 900;

struct PendingAdvancedRequest {
    attachment: Attachment,
    user_id: UserId,
    status_msg_id: MessageId,
    created_at: Instant,
    config: ReplayConfig,
}

struct AdvancedRequestStore;

impl TypeMapKey for AdvancedRequestStore {
    type Value = Arc<RwLock<HashMap<String, PendingAdvancedRequest>>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) if command.data.name == "echelon" => {
                self.handle_echelon_command(&ctx, &command).await;
            }
            Interaction::Component(component) => {
                self.handle_echelon_component(&ctx, &component).await;
            }
            _ => {}
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
                .add_option(
                    serenity::builder::CreateCommandOption::new(
                        serenity::all::CommandOptionType::SubCommand,
                        "advanced",
                        "Upload a replay with advanced settings",
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
    fn advanced_panel_content() -> &'static str {
        "Configure advanced settings, then press **Start**.\n\n\
        **1) Camera view** — standard or top-down.\n\
        **2) Player order** — Keep default order or swap players.\n\
        **3) Replay speed** — Playback/render speed multiplier.\n\
        **4) Video quality** — File size vs quality tradeoff."
    }

    async fn notify_advanced_start_failure(
        ctx: &Context,
        requester_id: UserId,
        channel_id: ChannelId,
    ) {
        let dm_message = format!(
            "❌ I couldn't post your replay status message in <#{}>. Please check channel permissions and try `/echelon advanced` again.",
            channel_id.get()
        );

        if let Err(e) = requester_id
            .dm(
                &ctx.http,
                serenity::builder::CreateMessage::new().content(dm_message),
            )
            .await
        {
            warn!(
                "Failed to DM user {} about advanced start failure: {}",
                requester_id, e
            );
        }
    }

    /// Extracts the replay file attachment from a slash command.
    fn extract_file_attachment(
        command: &CommandInteraction,
        subcommand: &str,
    ) -> Option<Attachment> {
        command.data.options.iter().find_map(|opt| {
            if opt.name == subcommand {
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

    fn parse_component_custom_id(custom_id: &str) -> Option<(String, String)> {
        let rest = custom_id.strip_prefix(ADVANCED_COMPONENT_PREFIX)?;
        let (token, action) = rest.split_once(':')?;
        Some((token.to_string(), action.to_string()))
    }

    fn build_advanced_components(token: &str, config: &ReplayConfig) -> Vec<CreateActionRow> {
        let top_down_menu = CreateSelectMenu::new(
            format!("{ADVANCED_COMPONENT_PREFIX}{token}:top_down_view"),
            CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("Standard view", "false")
                        .default_selection(!config.top_down_view),
                    CreateSelectMenuOption::new("Top-down view", "true")
                        .default_selection(config.top_down_view),
                ],
            },
        )
        .placeholder("1) Camera view")
        .min_values(1)
        .max_values(1);

        let swap_menu = CreateSelectMenu::new(
            format!("{ADVANCED_COMPONENT_PREFIX}{token}:swap_players"),
            CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("Keep original order", "false")
                        .default_selection(!config.swap_players),
                    CreateSelectMenuOption::new("Swap players around", "true")
                        .default_selection(config.swap_players),
                ],
            },
        )
        .placeholder("2) Player order")
        .min_values(1)
        .max_values(1);

        let speed_menu = CreateSelectMenu::new(
            format!("{ADVANCED_COMPONENT_PREFIX}{token}:game_speed"),
            CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("Slowest (0.5x)", "0.5")
                        .default_selection(config.game_speed == 0.5),
                    CreateSelectMenuOption::new("Slow (0.75x)", "0.75")
                        .default_selection(config.game_speed == 0.75),
                    CreateSelectMenuOption::new("Normal (1x)", "1.0")
                        .default_selection(config.game_speed == 1.0),
                    CreateSelectMenuOption::new("Fast (1.5x)", "1.5")
                        .default_selection(config.game_speed == 1.5),
                    CreateSelectMenuOption::new("Faster (2x)", "2.0")
                        .default_selection(config.game_speed == 2.0),
                    CreateSelectMenuOption::new("Very Fast (3x)", "3.0")
                        .default_selection(config.game_speed == 3.0),
                    CreateSelectMenuOption::new("Timelapse (10x)", "10.0")
                        .default_selection(config.game_speed == 10.0),
                ],
            },
        )
        .placeholder("3) Replay speed")
        .min_values(1)
        .max_values(1);

        let preset_menu = CreateSelectMenu::new(
            format!("{ADVANCED_COMPONENT_PREFIX}{token}:video_preset"),
            CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("File-size optimized", "file_size")
                        .default_selection(matches!(config.video_preset, VideoPreset::FileSize)),
                    CreateSelectMenuOption::new("Balanced (default)", "balanced")
                        .default_selection(matches!(
                            config.video_preset,
                            VideoPreset::Balanced
                        )),
                    CreateSelectMenuOption::new("High quality", "quality")
                        .default_selection(matches!(config.video_preset, VideoPreset::Quality)),
                ],
            },
        )
        .placeholder("4) Output quality preset")
        .min_values(1)
        .max_values(1);

        let buttons = vec![
            CreateButton::new(format!("{ADVANCED_COMPONENT_PREFIX}{token}:submit"))
                .label("Start")
                .style(ButtonStyle::Primary),
            CreateButton::new(format!("{ADVANCED_COMPONENT_PREFIX}{token}:cancel"))
                .label("Cancel")
                .style(ButtonStyle::Secondary),
        ];

        vec![
            CreateActionRow::SelectMenu(top_down_menu),
            CreateActionRow::SelectMenu(swap_menu),
            CreateActionRow::SelectMenu(speed_menu),
            CreateActionRow::SelectMenu(preset_menu),
            CreateActionRow::Buttons(buttons),
        ]
    }

    async fn advanced_store(ctx: &Context) -> Arc<RwLock<HashMap<String, PendingAdvancedRequest>>> {
        let data = ctx.data.read().await;
        data.get::<AdvancedRequestStore>()
            .cloned()
            .expect("Advanced request store missing")
    }

    fn prune_pending_requests(map: &mut HashMap<String, PendingAdvancedRequest>) {
        let now = Instant::now();
        map.retain(|_, req| {
            now.duration_since(req.created_at) < Duration::from_secs(ADVANCED_COMPONENT_TTL_SECS)
        });
    }

    /// Sends an immediate message response to a command interaction.
    async fn respond_with_command_message(
        ctx: &Context,
        command: &CommandInteraction,
        message: &str,
    ) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(message),
                ),
            )
            .await;
    }

    /// Sends a message response to a component interaction.
    async fn respond_with_component_message(
        ctx: &Context,
        component: &ComponentInteraction,
        message: &str,
        ephemeral: bool,
    ) {
        let mut response = CreateInteractionResponseMessage::new().content(message);
        if ephemeral {
            response = response.ephemeral(true);
        }

        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(response),
            )
            .await;
    }

    /// Routes /echelon subcommands.
    async fn handle_echelon_command(&self, ctx: &Context, command: &CommandInteraction) {
        let Some(subcommand) = command.data.options.first().map(|opt| opt.name.as_str()) else {
            Self::respond_with_command_message(ctx, command, "❌ Missing subcommand.").await;
            return;
        };

        match subcommand {
            "convert" => self.handle_convert_command(ctx, command).await,
            "advanced" => self.handle_advanced_command(ctx, command).await,
            _ => {
                Self::respond_with_command_message(
                    ctx,
                    command,
                    "❌ Unknown subcommand. Use /echelon convert or /echelon advanced.",
                )
                .await;
            }
        }
    }

    /// Handles the /echelon convert command.
    async fn handle_convert_command(&self, ctx: &Context, command: &CommandInteraction) {
        let file_attachment = Self::extract_file_attachment(command, "convert");

        let Some(attachment) = file_attachment else {
            Self::respond_with_command_message(ctx, command, "❌ No file attachment provided").await;
            return;
        };

        if let Err(msg) = validate_replay_file(&attachment.filename) {
            Self::respond_with_command_message(ctx, command, msg).await;
            return;
        }

        if let Err(e) = command.defer(&ctx.http).await {
            error!("Failed to defer command response: {e}");
            return;
        }

        if let Err(e) = command
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new()
                    .content(format!("⏳ Preparing replay for {}...", command.user.mention())),
            )
            .await
        {
            error!("Failed to edit command response: {e}");
            return;
        }

        let status_msg_id = match command.get_response(&ctx.http).await {
            Ok(msg) => msg.id,
            Err(e) => {
                error!("Failed to get deferred convert response: {e}");
                return;
            }
        };

        tokio::spawn(Self::process_replay_request(
            ReplayConfig::default(),
            attachment,
            command.channel_id,
            status_msg_id,
            command.user.id,
            ctx.http.clone(),
        ));
    }

    /// Handles the /echelon advanced command.
    async fn handle_advanced_command(&self, ctx: &Context, command: &CommandInteraction) {
        let file_attachment = Self::extract_file_attachment(command, "advanced");

        let Some(attachment) = file_attachment else {
            Self::respond_with_command_message(ctx, command, "❌ No file attachment provided")
                .await;
            return;
        };

        if let Err(msg) = validate_replay_file(&attachment.filename) {
            Self::respond_with_command_message(ctx, command, msg).await;
            return;
        }

        let token = command.id.get().to_string();
        let config = ReplayConfig::default();

        if let Err(e) = command.defer(&ctx.http).await {
            error!("Failed to defer advanced command response: {e}");
            return;
        }

        let status_msg = match command.get_response(&ctx.http).await {
            Ok(message) => message,
            Err(e) => {
                error!("Failed to get deferred advanced response: {e}");
                return;
            }
        };

        if let Err(e) = command
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new().content(format!(
                    "🛠️ Waiting for advanced settings from {}...",
                    command.user.mention()
                )),
            )
            .await
        {
            error!("Failed to edit advanced status message: {e}");
            return;
        }

        let store = Self::advanced_store(ctx).await;
        {
            let mut map = store.write().await;
            Self::prune_pending_requests(&mut map);
            map.insert(
                token.clone(),
                PendingAdvancedRequest {
                    attachment,
                    user_id: command.user.id,
                    status_msg_id: status_msg.id,
                    created_at: Instant::now(),
                    config: config.clone(),
                },
            );
        }

        let components = Self::build_advanced_components(&token, &config);
        let followup = CreateInteractionResponseFollowup::new()
                .content(Self::advanced_panel_content())
                .ephemeral(true)
                .components(components);

        if let Err(e) = command.create_followup(&ctx.http, followup).await {
            error!("Failed to open advanced settings: {e}");
            let store = Self::advanced_store(ctx).await;
            let mut map = store.write().await;
            map.remove(&token);
            let _ = command.delete_response(&ctx.http).await;
        }
    }

    /// Handles advanced settings component interactions.
    async fn handle_echelon_component(&self, ctx: &Context, component: &ComponentInteraction) {
        let Some((token, action)) = Self::parse_component_custom_id(&component.data.custom_id)
        else {
            return;
        };
        let selected_value = if matches!(
            action.as_str(),
            "top_down_view" | "swap_players" | "game_speed" | "video_preset"
        ) {
            match &component.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => {
                    values.first().cloned()
                }
                _ => None,
            }
        } else {
            None
        };

        let store = Self::advanced_store(ctx).await;
        let mut map = store.write().await;
        Self::prune_pending_requests(&mut map);

        let Some(pending) = map.get(&token) else {
            drop(map);
            Self::respond_with_component_message(
                ctx,
                component,
                "❌ This advanced request expired. Please run /echelon advanced again.",
                true,
            )
            .await;
            return;
        };

        if pending.user_id != component.user.id {
            drop(map);
            Self::respond_with_component_message(
                ctx,
                component,
                "❌ This menu belongs to a different user.",
                true,
            )
            .await;
            return;
        }

        match action.as_str() {
            "top_down_view" | "swap_players" | "game_speed" | "video_preset" => {
                let Some(value) = selected_value else {
                    drop(map);
                    Self::respond_with_component_message(
                        ctx,
                        component,
                        "❌ Missing selection value.",
                        true,
                    )
                    .await;
                    return;
                };

                if let Some(pending) = map.get_mut(&token) {
                    let open_ms = pending.created_at.elapsed().as_millis();
                    match action.as_str() {
                        "top_down_view" => {
                            pending.config.top_down_view = value == "true";
                        }
                        "swap_players" => {
                            pending.config.swap_players = value == "true";
                        }
                        "game_speed" => {
                            if let Ok(parsed) = value.parse::<f64>() {
                                pending.config.game_speed = parsed;
                            }
                        }
                        "video_preset" => {
                            pending.config.video_preset = VideoPreset::from_str_name(&value);
                        }
                        _ => {}
                    }

                    info!(
                        "[advanced:{}] user={} updated {}={} after {}ms",
                        token, pending.user_id, action, value, open_ms
                    );
                }

                drop(map);
                let _ = component
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await;
            }
            "cancel" => {
                let removed = map.remove(&token);
                drop(map);

                if let Some(pending) = removed {
                    info!(
                        "[advanced:{}] user={} canceled after {}ms",
                        token,
                        pending.user_id,
                        pending.created_at.elapsed().as_millis()
                    );

                        if let Err(e) = component
                            .channel_id
                            .delete_message(&ctx.http, pending.status_msg_id)
                            .await
                        {
                            warn!(
                                "Failed to delete advanced status message after cancel: {}",
                                e
                            );
                        }
                } else {
                    warn!(
                        "[advanced:{}] cancel received but request was already removed",
                        token
                    );
                }

                let _ = component
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await;

                if let Err(e) = component.delete_response(&ctx.http).await {
                    warn!("Failed to delete advanced ephemeral response after cancel: {e}");
                }
            }
            "submit" => {
                let pending = map.remove(&token);
                drop(map);

                let Some(pending) = pending else {
                    warn!(
                        "[advanced:{}] submit received but request was already removed",
                        token
                    );
                    Self::respond_with_component_message(
                        ctx,
                        component,
                        "❌ This advanced request was already submitted or expired. Please run /echelon advanced again.",
                        true,
                    )
                    .await;
                    return;
                };

                info!(
                    "[advanced:{}] user={} submitted after {}ms (top_down_view={}, swap_players={}, game_speed={}, video_preset={})",
                    token,
                    pending.user_id,
                    pending.created_at.elapsed().as_millis(),
                    pending.config.top_down_view,
                    pending.config.swap_players,
                    pending.config.game_speed,
                    pending.config.video_preset.as_str(),
                );

                if let Err(e) = component
                    .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                    .await
                {
                    error!("Failed to respond to advanced submit: {e}");
                    return;
                }

                if let Err(e) = component.delete_response(&ctx.http).await {
                    warn!("Failed to delete advanced ephemeral response after submit: {e}");
                }

                let http = ctx.http.clone();
                let channel_id = component.channel_id;
                let requester_id = component.user.id;
                if let Err(e) = channel_id
                    .edit_message(
                        &ctx.http,
                        pending.status_msg_id,
                        serenity::builder::EditMessage::new().content(format!(
                            "⏳ Uploading replay for {}...",
                            requester_id.mention()
                        )),
                    )
                    .await
                {
                    error!("Failed to update advanced status message: {e}");
                    Self::notify_advanced_start_failure(ctx, requester_id, channel_id).await;
                    return;
                }

                tokio::spawn(Self::process_replay_request(
                    pending.config,
                    pending.attachment,
                    channel_id,
                    pending.status_msg_id,
                    requester_id,
                    http,
                ));
            }
            _ => {
                drop(map);
                Self::respond_with_component_message(
                    ctx,
                    component,
                    "❌ Unknown action.",
                    true,
                )
                .await;
            }
        }
    }

    async fn process_replay_request(
        config: ReplayConfig,
        attachment: Attachment,
        channel_id: ChannelId,
        status_msg_id: MessageId,
        requester_id: UserId,
        http: Http,
    ) {
        let server_url = get_server_url();
        if let Err(msg) = validate_replay_file(&attachment.filename) {
            update_status_message(channel_id, &http, status_msg_id, msg).await;
            return;
        }

        match attachment.download().await {
            Ok(data) => {
                match create_replay_with_config(server_url, &config).await {
                    Ok(task_id) => {
                        match upload_replay(server_url, &task_id, data).await {
                            Ok(()) => {
                                update_status_message(channel_id, &http, status_msg_id, "📋 Replay queued!").await;

                                tokio::spawn(monitor_replay(
                                    server_url,
                                    task_id,
                                    channel_id,
                                    status_msg_id,
                                    requester_id,
                                    http,
                                ));
                            }
                            Err(e) => {
                                error!("Failed to upload replay: {e}");
                                let error_msg = translate_api_error(&e, "upload");
                                update_status_message(channel_id, &http, status_msg_id, error_msg).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to create replay job: {e}");
                        let error_msg = translate_api_error(&e, "create");
                        update_status_message(channel_id, &http, status_msg_id, error_msg).await;
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
                update_status_message(channel_id, &http, status_msg_id, error_msg).await;
            }
        }
    }
}

/// Formats a replay status into a user-friendly message with optional animation frame.
fn format_status(status: &ReplayStatus, animation_frame: Option<usize>) -> String {
    match status {
        ReplayStatus::Queued {
            position,
            estimate_minutes,
        } => {
            format!("⏳ Queued at position {position} (ETA: {estimate_minutes} min.)")
        }
        ReplayStatus::Processing { estimate_minutes } => {
            let spinner = ['⠋', '⠙', '⠴', '⠦'][animation_frame.unwrap_or(0) % 4];

            format!("{spinner} Processing... (ETA: {estimate_minutes} min.)")
        }
        ReplayStatus::Done => "✅ Replay is ready!".to_string(),
        ReplayStatus::Error { message } => format!("❌ {message}"),
        ReplayStatus::NotFound { message } => format!("❓ Not found: {message}"),
    }
}

async fn monitor_replay(
    server_url: &'static str,
    id: String,
    channel_id: ChannelId,
    status_msg_id: MessageId,
    requester_id: UserId,
    http: Http,
) {
    let start_time = Instant::now();
    let mut last_status: Option<ReplayStatus> = None;
    let mut last_update = Instant::now();
    let mut update_count: usize = 0;

    loop {
        // Check if we've exceeded the maximum monitoring duration
        if start_time.elapsed() >= Duration::from_secs(MAX_MONITORING_DURATION_SECS) {
            warn!(
                "[{}] Monitoring task exceeded maximum duration ({} minutes), terminating.",
                id,
                MAX_MONITORING_DURATION_SECS / 60
            );
            let timeout_message = format!(
                "[`{id}`] ⏱️ Monitoring timed out after {} minutes. The job may still be processing. Check the status later or contact us if this persists.",
                MAX_MONITORING_DURATION_SECS / 60
            );
            update_status_message(channel_id, &http, status_msg_id, &timeout_message).await;
            break;
        }

        let poll_interval_secs = if let Some(ref status) = last_status {
            match status {
                ReplayStatus::Processing { .. } => POLL_INTERVAL_PROCESSING_SECS,
                _ => POLL_INTERVAL_DEFAULT_SECS,
            }
        } else {
            POLL_INTERVAL_PROCESSING_SECS
        };

        tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;

        match get_replay_status(server_url, &id).await {
            Ok(status) => {
                let status_changed = last_status
                    .as_ref()
                    .is_none_or(|last| last != &status);

                let should_update = status_changed
                    || last_update.elapsed() >= Duration::from_secs(STALE_STATUS_THRESHOLD_SECS);

                if matches!(status, ReplayStatus::Done) {
                    send_video_message(
                        &channel_id,
                        &http,
                        server_url,
                        &id,
                        requester_id,
                        status_msg_id,
                    )
                    .await;
                    break;
                }

                if matches!(status, ReplayStatus::Processing { .. }) || should_update {
                    let message = format_status(&status, Some(update_count));
                    update_status_message(channel_id, &http, status_msg_id, &message).await;
                    if should_update {
                        last_update = Instant::now();
                    }
                    update_count += 1;
                }

                if let ReplayStatus::Error { message } = &status {
                    let error_message = format!("[`{id}`] ❌ {message}");
                    update_status_message(channel_id, &http, status_msg_id, &error_message).await;
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
                    update_status_message(channel_id, &http, status_msg_id, &error_message).await;
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
    match download_video(server_url, id).await {
        Ok(video_data) => {
            let video_size = video_data.len();
            let video_size_mb = video_size as f64 / (1024.0 * 1024.0);

            let filename = format!("{id}.mp4");
            match channel_id
                .edit_message(
                    http,
                    status_msg_id,
                    serenity::builder::EditMessage::new()
                        .content(format!("✅ {}, your replay is ready!", requester_id.mention()))
                        .new_attachment(serenity::builder::CreateAttachment::bytes(
                            video_data, filename,
                        )),
                )
                .await
            {
                Ok(_) => {
                    info!(
                        "Attached and sent video on status message for replay {id} ({:.2} MB)",
                        video_size_mb
                    )
                }
                Err(e) => {
                    let error_str = e.to_string();
                    info!(
                        "Failed to attach video to status message: {e} - trying download link instead."
                    );

                    let msg = if error_str.contains("40005") || error_str.contains("too large") {
                        format!(
                            "✅ {}, your replay is ready! However, the video is too large for Discord.\n\n\
                            📥 **Download it here (available for 1 hour):**\n{}/download/{}\n\n\
                            ℹ️ The preview below works only during this 1-hour window.",
                            requester_id.mention(),
                            server_url,
                            id
                        )
                    } else {
                        format!(
                            "✅ {}, your replay is ready! However, we failed to send the video through Discord.\n\n\
                            📥 **Download it here (available for 1 hour):**\n{}/download/{}\n\n\
                            ℹ️ The preview below works only during this 1-hour window.",
                            requester_id.mention(),
                            server_url,
                            id
                        )
                    };
                    update_status_message(*channel_id, http, status_msg_id, &msg).await;
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
            update_status_message(*channel_id, http, status_msg_id, &error_msg).await;
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    _ = dotenvy::dotenv();

    info!("Starting Discord bot...");

    // Validate required environment variables at startup (fail fast)
    validate_server_url();

    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
    let intents = GatewayIntents::empty();

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client");

    {
        let mut data = client.data.write().await;
        data.insert::<AdvancedRequestStore>(Arc::new(RwLock::new(HashMap::new())));
    }

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
