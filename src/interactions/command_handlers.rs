use std::sync::Arc;

use crate::AppState;
use crate::todoist;
use crate::todoist::http::models::Due;
use chrono::DateTime;
use chrono::FixedOffset;
use chrono::Local;
use todoist_derive::Command;
use twilight_model::http::interaction::InteractionResponseData;
use twilight_model::http::interaction::InteractionResponseType;
use twilight_model::{
    application::interaction::Interaction, channel::message::MessageFlags,
    http::interaction::InteractionResponse,
};
use twilight_util::builder::message::ContainerBuilder;
use twilight_util::builder::message::TextDisplayBuilder;

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
        .accent_color(Some(0xFF000))
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
