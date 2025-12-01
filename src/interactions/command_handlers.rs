use anyhow::Result;
use futures::future;
use std::sync::Arc;
use twilight_model::channel::Message;
use twilight_model::channel::message::Component;
use twilight_model::channel::message::component::Container;
use twilight_model::channel::message::component::Section;
use twilight_model::channel::message::component::SelectMenuType;
use twilight_util::builder::message::ActionRowBuilder;
use twilight_util::builder::message::ButtonBuilder;
use twilight_util::builder::message::SectionBuilder;
use twilight_util::builder::message::SelectMenuBuilder;
use twilight_util::builder::message::SelectMenuOptionBuilder;
use twilight_util::builder::message::SeparatorBuilder;

use crate::AppState;
use crate::claude::message_create;
use crate::claude::models::InputMessage;
use crate::claude::models::MessageRequest;
use crate::emoji::Emojis;
use crate::get_timezone_override;
use crate::todoist;
use crate::todoist::NewTask;
use crate::todoist::http::models::Due;
use chrono::DateTime;
use chrono::FixedOffset;
use std::env;
use tracing::debug;
use twilight_commands::Command;
use twilight_model::application::interaction::InteractionData;
use twilight_model::channel::message::EmojiReactionType;
use twilight_model::channel::message::component::ButtonStyle;
use twilight_model::channel::message::component::SeparatorSpacingSize;
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
) -> Result<InteractionResponse> {
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
        return Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(format!(
                    "{} Could not find the target message to create a reminder from.",
                    Emojis::RED_X
                )),
                ..Default::default()
            }),
        });
    }
    let target_message = target_message.unwrap();
    let content = message_to_string(target_message);
    debug!("Asking Claude to create reminder from text: {}", content);

    let response = message_create(
        state.claude_client.as_ref(),
        MessageRequest {
            model: state.claude_client.model.clone(),
            max_tokens: 1000,
            messages: vec![InputMessage {
                role: "user".to_string(),
                content: format!("Create a reminder to add to my to-do list from the following message: {}", content),
            }],
            system: Some(
                "You are a helpful assistant that creates reminders. Use concise language, and only respond with the reminder text without any additional commentary. The reminder should be suitable for adding to a to-do list application."
                    .to_string(),
            ),
        }).await?;

    if env::var("DRY_RUN").unwrap_or("false".to_string()) == "true" {
        debug!("Dry run enabled, not creating task in Todoist.");
        return Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(format!(
                    "{} (Dry Run) Created reminder: **{}**",
                    Emojis::GREEN_TICK,
                    response
                )),
                ..Default::default()
            }),
        });
    }

    debug!("Claude response: {}", response);
    let projects = todoist::get_projects(&state.todoist_client).await?;
    debug!("Retrieved {} projects from Todoist", projects.len());
    let projects_with_sections = future::join_all(projects.iter().map(|project| {
        let client = state.todoist_client.clone();
        let project_id = project.id.as_str().to_string();
        async move {
            let sections = todoist::get_sections(&client, &project_id)
                .await
                .unwrap_or(Vec::new());
            debug!(
                "Retrieved {} sections for project {}",
                sections.len(),
                project.name
            );
            (project, sections)
        }
    }))
    .await;

    // Create the task
    let new_task = todoist::create_task(
        &state.todoist_client,
        NewTask {
            content: format!("{}", response),
            description: Some(format!(
                "Created from message: https://discord.com/channels/{}/{}/{}",
                interaction
                    .guild_id
                    .map(|id| id.get().to_string())
                    .unwrap_or("@me".to_string()),
                target_message.channel_id,
                target_message.id
            )),
            ..Default::default()
        },
    )
    .await?;

    debug!("Created new task in Todoist: {:#?}", new_task);
    let mut section_component = SelectMenuBuilder::new(
        format!("section_select:{}", new_task.id),
        SelectMenuType::Text,
    )
    .placeholder("Update Section");

    for (project, sections) in projects_with_sections.iter() {
        // let builder = SelectMenuOptionBuilder::new("")
        section_component = section_component.option(
            SelectMenuOptionBuilder::new(project.name.clone(), project.id.clone())
                .description(format!("Add to project: {}", project.name))
                .build(),
        );
        for section in sections {
            let option = SelectMenuOptionBuilder::new(
                format!("{} / {}", project.name, section.name),
                format!("{}-{}", project.id, section.id),
            )
            .description(format!("Add to section: {}", section.name))
            .build();
            section_component = section_component.option(option);
        }
    }

    let section_component = section_component.build();
    let section_component = ActionRowBuilder::new().component(section_component).build();

    let header = TextDisplayBuilder::new(format!(
        "{} Created task:\n**{}**",
        Emojis::GREEN_TICK,
        new_task.content
    ))
    .build();

    let accessory = ButtonBuilder::new(ButtonStyle::Link)
        .label("View Task")
        .url(new_task.get_url())
        .emoji(EmojiReactionType::Unicode {
            name: "🔗".to_string(),
        })
        .build();

    let container = ContainerBuilder::new()
        .accent_color(Some(0x00AA00))
        .component(SectionBuilder::new(accessory).component(header).build())
        .component(
            SeparatorBuilder::new()
                .divider(true)
                .spacing(SeparatorSpacingSize::Large)
                .build(),
        )
        .component(section_component)
        .build();

    Ok(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            components: Some(vec![container.into()]),
            flags: Some(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2),
            ..Default::default()
        }),
    })
}

#[derive(Command)]
#[command(name = "today", description = "Get reminders due today")]
pub struct TodayReminders;

pub async fn handle_today(
    _args: TodayReminders,
    _interaction: Arc<Interaction>,
    state: Arc<AppState>,
) -> Result<InteractionResponse> {
    let timezone = get_timezone_override();
    debug!("Using timezone: {:?}", timezone);

    let tasks = todoist::get_tasks_due_today(&state.todoist_client, timezone).await?;

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

    Ok(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            components: Some(vec![container.into()]),
            flags: Some(MessageFlags::IS_COMPONENTS_V2 | MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    })
}

fn message_to_string(message: &Message) -> String {
    let mut content = String::new();
    content.push_str(message.content.as_str());

    // Handle embeds
    for embed in &message.embeds {
        let mut embed_content = String::new();
        if let Some(title) = &embed.title {
            embed_content.push_str(title);
            embed_content.push('\n');
        }
        if let Some(description) = &embed.description {
            embed_content.push_str(description);
            embed_content.push('\n');
        }
        embed_content.push_str(
            embed
                .fields
                .iter()
                .map(|field| format!("{}: {}", field.name, field.value))
                .collect::<Vec<String>>()
                .join("\n")
                .as_str(),
        );
        content.push_str(embed_content.as_str());
        content.push('\n');
    }

    // Handle cv2 components
    for component in &message.components {
        let mut component_content = String::new();

        if let Some(comp_str) = handle_component(component) {
            component_content.push_str(comp_str.as_str());

            content.push_str(component_content.as_str());
            content.push('\n');
        }
    }
    content
}

fn handle_component(component: &Component) -> Option<String> {
    debug!("Handling component: {:#?}", component);
    match component {
        Component::Section(section) => handle_section(section),
        Component::Container(container) => handle_container(container),
        Component::TextDisplay(text) => Some(text.content.clone()),
        _ => None,
    }
}

fn handle_container(container: &Container) -> Option<String> {
    let mut container_contents = String::new();
    for comp in &container.components {
        if let Some(comp_str) = handle_component(comp) {
            container_contents.push_str(comp_str.as_str());
            container_contents.push('\n');
        }
    }
    if !container_contents.is_empty() {
        Some(container_contents)
    } else {
        None
    }
}

fn handle_section(section: &Section) -> Option<String> {
    let mut section_contents = String::new();
    for comp in &section.components {
        if let Some(comp_str) = handle_component(comp) {
            section_contents.push_str(comp_str.as_str());
            section_contents.push('\n');
        }
    }
    if !section_contents.is_empty() {
        Some(section_contents)
    } else {
        None
    }
}
