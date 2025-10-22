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
#[command(
    name = "no-arguments",
    description = "A test command with no arguments"
)]
pub struct TestCommandNoArguments;

pub async fn test_command_no_arguments(
    _command: TestCommandNoArguments,
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("Test command with no arguments executed!".to_string()),
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

#[derive(Command)]
#[command(name = "subcommand test", description = "A test command group")]
pub struct SubCommand1;

pub async fn subcommand1_handler(
    _command: SubCommand1,
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("Subcommand 1 executed!".to_string()),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}

#[derive(Command)]
#[command(name = "subcommand test2", description = "Another test command group")]
pub struct SubCommand2 {
    #[option(description = "An option for subcommand 2")]
    value: String,
}

pub async fn subcommand2_handler(
    command: SubCommand2,
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(format!(
                "Subcommand 2 executed with value: {}",
                command.value
            )),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}

#[derive(Command)]
#[command(
    name = "subcommand-group one test",
    description = "A test subcommand group"
)]
pub struct SubCommandGroup1;

pub async fn subcommand_group1_handler(
    _command: SubCommandGroup1,
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("Subcommand Group 1 executed!".to_string()),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}

#[derive(Command)]
#[command(
    name = "subcommand-group two test",
    description = "Another test subcommand group"
)]
pub struct SubCommandGroup2;

pub async fn subcommand_group2_handler(
    _command: SubCommandGroup2,
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("Subcommand Group 2 executed!".to_string()),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}

#[derive(Command)]
#[command(
    name = "subcommand-group two test2",
    description = "Yet another test subcommand group"
)]
pub struct SubCommandGroup3 {
    #[option(description = "An option for subcommand group 3")]
    value: String,
}

pub async fn subcommand_group3_handler(
    command: SubCommandGroup3,
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(format!(
                "Subcommand Group 3 executed with value: {}",
                command.value
            )),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}
