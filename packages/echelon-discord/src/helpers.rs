//! Helper functions for error handling, validation, and Discord messaging.

use serenity::builder::EditMessage;
use serenity::http::Http;
use serenity::model::id::{ChannelId, MessageId};
use tracing::error;

/// Translates API errors into user-friendly messages based on error details and action type.
pub fn translate_api_error(error: &str, action: &str) -> &'static str {
    if error.contains("500") {
        "❌ Server error occurred. The processing service is experiencing issues. Please try again in a few moments."
    } else if error.contains("Request failed") {
        "❌ Failed to reach the processing server. Please try again; if this error persists, reach out to us for help."
    } else {
        match action {
            "upload" => "❌ Upload failed. Please try again.",
            "create" => "❌ Failed to create replay job. Please try again.",
            "download" => "❌ Failed to download the file. Please check the file size and try again.",
            _ => "❌ Operation failed. Please try again.",
        }
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
