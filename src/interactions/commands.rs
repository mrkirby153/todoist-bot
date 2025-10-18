use std::sync::Arc;

use crate::AppState;
use twilight_model::http::interaction::InteractionResponseData;
use twilight_model::http::interaction::InteractionResponseType;
use twilight_model::{
    application::interaction::Interaction, channel::message::MessageFlags,
    http::interaction::InteractionResponse,
};

pub async fn add_reminder(
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("Reminder added!".to_string()),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}
