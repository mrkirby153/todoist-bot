use std::sync::Arc;

use crate::AppState;
use todoist_derive::Choices;
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
#[command(name = "testing", description = "A test command")]
pub struct TestCommand {
    #[option(description = "The first option")]
    option_one: String,
    #[option(description = "This is a select")]
    choice: TestEnum,
    #[option(description = "The second option")]
    option_two: Option<i32>,
}

#[derive(Choices, Debug)]
enum TestEnum {
    #[choice(value = "1")]
    One,
    #[choice(name = "Three", value = "2")]
    Two,
}

pub async fn test_command(
    command: TestCommand,
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(format!(
                "Received option_one: {}, option_two: {:?}, choice: {:?}",
                command.option_one, command.option_two, command.choice
            )),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}
