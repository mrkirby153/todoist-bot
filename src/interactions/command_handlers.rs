use std::sync::Arc;

use crate::AppState;
use crate::claude::message_create;
use crate::claude::models::InputMessage;
use crate::claude::models::MessageRequest;
use crate::emoji::Emojis;
use crate::todoist;
use crate::todoist::http::models::Due;
use chrono::DateTime;
use chrono::FixedOffset;
use chrono::Local;
use todoist_derive::Command;
use tracing::debug;
use twilight_model::application::interaction::InteractionData;
use twilight_model::http::interaction::InteractionResponseData;
use twilight_model::http::interaction::InteractionResponseType;
use twilight_model::id::Id;
use twilight_model::{
    application::interaction::Interaction, channel::message::MessageFlags,
    http::interaction::InteractionResponse,
};
use twilight_util::builder::message::ContainerBuilder;
use twilight_util::builder::message::TextDisplayBuilder;

pub async fn add_reminder(
    interaction: Arc<Interaction>,
    state: Arc<AppState>,
) -> InteractionResponse {
    debug!("Received add_reminder interaction: {:#?}", interaction);

    let target_message = {
        if let Some(InteractionData::ApplicationCommand(c)) = interaction.data.as_ref()
            && let Some(resolved) = c.resolved.as_ref()
            && let Some(target) = c.target_id
        {
            resolved.messages.get(&Id::new(target.get()))
        } else {
            None
        }
    };

    if target_message.is_none() {
        return InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(format!(
                    "{} Could not find the target message to create a reminder from.",
                    Emojis::RED_X
                )),
                ..Default::default()
            }),
        };
    }
    let target_message = target_message.unwrap();

    let response = message_create(
        state.claude_client.as_ref(),
        MessageRequest {
            model: state.claude_client.model.clone(),
            max_tokens: 1000,
            messages: vec![InputMessage {
                role: "user".to_string(),
                content: format!("Create a reminder to add to my to-do list from the following message: {}", target_message.content),
            }],
            system: Some(
                "You are a helpful assistant that creates reminders. Use concise language, and only respond with the reminder text without any additional commentary. The reminder should be suitable for adding to a to-do list application."
                    .to_string(),
            ),
        }).await;

    println!("Claude response: {:?}", response);

    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(match response {
                Ok(message_response) => {
                    format!(
                        "{} Anthropic's response: ```\n{:#?}\n```",
                        Emojis::GREEN_TICK,
                        message_response
                    )
                }
                Err(e) => format!("{} An error occurred: {}", Emojis::RED_X, e),
            }),
            ..Default::default()
        }),
    }
}

#[derive(Command)]
#[command(name = "today", description = "Get reminders due today")]
pub struct TodayReminders;

pub async fn handle_today(
    _args: TodayReminders,
    _interaction: Arc<Interaction>,
    state: Arc<AppState>,
) -> InteractionResponse {
    let due_today = todoist::get_tasks_due_today(&state.todoist_client, Local).await;

    match due_today {
        Ok(tasks) => {
            let accent_color = if tasks.is_empty() {
                0x00AA00 // Green for no tasks
            } else {
                0xAAAA00 // Yellow for tasks due
            };

            let container = ContainerBuilder::new()
                .accent_color(Some(accent_color))
                .component(if tasks.is_empty() {
                    TextDisplayBuilder::new("You have no more tasks due today!".to_string()).build()
                } else {
                    let mut content = format!("There are **{}** tasks due today:\n", tasks.len());
                    for task in tasks {
                        let mut task_format = format!("[{}]({})", task.content, task.get_url());

                        if let Some(due) = &task.due
                            && !due.is_date_only()
                            && let Ok(due_date) =
                                <Due as TryInto<DateTime<FixedOffset>>>::try_into(due.clone())
                        {
                            let due_unix_time = due_date.timestamp();
                            task_format.push_str(&format!(" <t:{}:t>", due_unix_time));
                        }

                        content.push_str(&format!("- {}\n", task_format));
                    }
                    TextDisplayBuilder::new(content).build()
                })
                .build();

            InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    components: Some(vec![container.into()]),
                    flags: Some(MessageFlags::IS_COMPONENTS_V2 | MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            }
        }
        Err(e) => build_error_response(e),
    }
}

fn build_error_response(result: anyhow::Error) -> InteractionResponse {
    let container = ContainerBuilder::new()
        .accent_color(Some(0xFF0000))
        .component(TextDisplayBuilder::new(format!("An error occurred: {}", result)).build())
        .build();

    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            components: Some(vec![container.into()]),
            flags: Some(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2),
            ..Default::default()
        }),
    }
}
