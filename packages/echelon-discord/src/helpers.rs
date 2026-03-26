//! Helper functions for error handling, validation, and Discord messaging.

use serenity::builder::EditMessage;
use serenity::http::Http;
use serenity::model::id::{ChannelId, MessageId};
use tracing::error;

/// Translates API errors into user-friendly messages based on error details and action type.
pub fn translate_api_error(error: &str, action: &str) -> String {
    let lower_error = error.to_ascii_lowercase();

    if lower_error.contains("request failed") {
        return "❌ Failed to reach the processing server. Please try again; if this error persists, reach out to us for help.".to_string();
    }

    if lower_error.contains("429") || lower_error.contains("too many requests") {
        return "⏳ Too many requests right now. Please wait a moment and try again.".to_string();
    }

    if lower_error.contains("413") || lower_error.contains("payload too large") {
        return "❌ Replay file is too large. Please upload a smaller file (max 10 MB).".to_string();
    }

    if lower_error.contains("queue is full") || lower_error.contains("503") {
        return "🚧 The processing queue is currently full. Please try again in a few minutes.".to_string();
    }

    if lower_error.contains("file is not a *.yrpx file") {
        return "❌ Invalid replay format. Please upload a valid `.yrpX` replay file.".to_string();
    }

    if lower_error.contains("invalid replay file") || lower_error.contains("corrupted") {
        return "❌ Replay file appears to be corrupted or unreadable. Please export/upload a valid replay and try again.".to_string();
    }

    if lower_error.contains("task id not found") {
        return "❌ Upload session expired or was not found. Please run the command again and re-upload your replay.".to_string();
    }

    if lower_error.contains("task is already finished") {
        return "❌ This upload session is already closed. Please run the command again to create a new session.".to_string();
    }

    if lower_error.contains("video not found") {
        return "❌ Video is not ready or has expired. Please re-upload the replay and try again.".to_string();
    }

    if lower_error.contains("500") {
        return "❌ Server error occurred. The processing service is experiencing issues. Please try again in a few moments.".to_string();
    }

    match action {
        "upload" => "❌ Upload failed due to an unexpected server response. Please try again.".to_string(),
        "create" => {
            "❌ Failed to create replay job due to an unexpected server response. Please try again."
                .to_string()
        }
        "download" => {
            "❌ Failed to download the completed video due to an unexpected server response."
                .to_string()
        }
        _ => "❌ Operation failed due to an unexpected server response. Please try again."
            .to_string(),
    }
}

/// Validates that a filename is a valid replay file (.yrpX extension).
pub fn validate_replay_file(filename: &str) -> Result<(), &'static str> {
    if filename.ends_with(".yrpX") {
        Ok(())
    } else {
        Err("❌ Please upload a `.yrpX` replay file")
    }
}

/// Attempts to update a Discord message, falling back to sending a new message if editing fails.
pub async fn update_status_message(
    channel_id: ChannelId,
    http: &Http,
    msg_id: MessageId,
    content: &str,
) {
    if let Err(e) = channel_id
        .edit_message(http, msg_id, EditMessage::new().content(content))
        .await
    {
        error!("Failed to edit status message: {e}");
        if let Err(send_err) = channel_id.say(http, content).await {
            error!("Failed to send fallback message: {send_err}");
        }
    }
}
