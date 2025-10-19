use std::sync::Arc;

use crate::AppState;
use todoist_derive::Command;
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

#[derive(Command)]

pub struct TestCommand {
    #[option(description = "The first option")]
    option_one: String,
    #[option(description = "The second option")]
    option_two: Option<i32>,
}

pub async fn test_command(
    _command: TestCommand,
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(format!(
                "Received option_one: {}, option_two: {:?}",
                _command.option_one, _command.option_two
            )),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}
