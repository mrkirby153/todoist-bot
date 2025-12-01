use std::{sync::Arc, time::Duration};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use tokio::time;
use tokio::{select, task::JoinHandle, time::timeout};
use tracing::{debug, error, info, warn};
use twilight_http::Client;
use twilight_model::http::interaction::InteractionResponseType;
use twilight_model::{
    application::interaction::InteractionData, channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
};
use twilight_model::{
    application::interaction::{Interaction, InteractionType},
    http::interaction::InteractionResponse,
};
use twilight_util::builder::message::{ContainerBuilder, TextDisplayBuilder};

use crate::{
    AppState,
    emoji::Emojis,
    interactions::resolve_command_path,
    todoist::{MoveTask, move_task},
};

pub async fn health() -> &'static str {
    "OK"
}

#[axum::debug_handler]
pub async fn interaction_callback(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: String,
) -> Result<Json<InteractionResponse>, StatusCode> {
    debug!("Received interaction callback");

    let signature = headers
        .get("x-signature-ed25519")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let timestamp = headers
        .get("x-signature-timestamp")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    debug!("Signature: {:?}", signature);
    debug!("Timestamp: {:?}", timestamp);

    if state
        .verifier
        .verify(signature, timestamp, body.as_bytes())
        .is_err()
    {
        debug!("Invalid signature");
        return Err(StatusCode::UNAUTHORIZED);
    }
    debug!("Signature verified");

    let interaction: Arc<Interaction> =
        Arc::new(serde_json::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?);

    let resp = match interaction.kind {
        InteractionType::Ping => InteractionResponse {
            kind: InteractionResponseType::Pong,
            data: None,
        },
        InteractionType::ApplicationCommand => {
            if let Some(InteractionData::ApplicationCommand(ref command)) = interaction.data {
                let name = &command.name;
                debug!("Processing application command: {}", name);

                let resolved_slash_command = resolve_command_path(command);

                let slash_result =
                    if let Some((command_path, command_data)) = resolved_slash_command {
                        debug!("Resolved command path: {}", command_path);

                        let state = Arc::new(state.clone());
                        let callback_state = Arc::clone(&state);
                        let interaction = Arc::clone(&interaction);
                        let callback_interaction = Arc::clone(&interaction);

                        let handle = tokio::spawn(async move {
                            let inner_state = Arc::clone(&state);
                            state
                                .slash_commands
                                .execute(&command_path, interaction, command_data, inner_state)
                                .await
                        });

                        handle_response(
                            handle,
                            callback_state.client.clone(),
                            callback_state,
                            callback_interaction,
                        )
                        .await
                    } else {
                        None
                    };

                if let Some(response) = slash_result {
                    debug!("Returning response: {:?}", response);
                    response
                } else if let Some(handler) = state.context_commands.get(name).cloned() {
                    let state = Arc::new(state.clone());
                    let handler_state = Arc::clone(&state);
                    let interaction = Arc::clone(&interaction);
                    let handler_interaction = Arc::clone(&interaction);

                    let handle = tokio::spawn(async move {
                        Some(handler(interaction, state).await.unwrap_or_else(|e| {
                            let container = ContainerBuilder::new()
                                .accent_color(Some(0xFF0000))
                                .component(
                                    TextDisplayBuilder::new(format!("An error occurred: {}", e))
                                        .build(),
                                )
                                .build();

                            InteractionResponse {
                                kind: InteractionResponseType::ChannelMessageWithSource,
                                data: Some(InteractionResponseData {
                                    components: Some(vec![container.into()]),
                                    flags: Some(
                                        MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2,
                                    ),
                                    ..Default::default()
                                }),
                            }
                        }))
                    });

                    handle_response(
                        handle,
                        handler_state.client.clone(),
                        handler_state,
                        handler_interaction,
                    )
                    .await
                    .unwrap_or_else(|| InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData {
                            content: Some(
                                "An error occurred while processing your command.".to_string(),
                            ),
                            flags: Some(MessageFlags::EPHEMERAL),
                            ..InteractionResponseData::default()
                        }),
                    })
                } else {
                    warn!("No handler found for command: {}", name);
                    InteractionResponse {
                        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData{
                            content: Some(format!("No handler found for command: `{}`", name)),
                            flags: Some(MessageFlags::EPHEMERAL),
                            ..InteractionResponseData::default()
                        })
                    }
                }
            } else {
                warn!("Unhandled interaction type: {:?}", interaction.data);
                InteractionResponse {
                    kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData{
                        content: Some(format!("Unhandled interaction type: `{:?}`", interaction.data)),
                        flags: Some(MessageFlags::EPHEMERAL),
                        ..InteractionResponseData::default()
                    })
                }
            }
        }
        InteractionType::MessageComponent => {
            if let Some(InteractionData::MessageComponent(ref data)) = interaction.data {
                debug!("Processing message component interaction: {:?}", data);
                let custom_id_parts = data.custom_id.split(":").collect::<Vec<&str>>();

                let command = custom_id_parts.first().unwrap_or(&"");
                match *command {
                    "section_select" => {
                        let task_id = custom_id_parts.get(1);
                        match task_id {
                            Some(task_id) => {
                                let project_section = data.values.first();
                                let default = "".to_string();
                                let parts = project_section
                                    .unwrap_or(&default)
                                    .split("-")
                                    .collect::<Vec<&str>>();
                                let (project_id, section_id) = match parts.as_slice() {
                                    [proj, sect] => {
                                        (Some(proj.to_string()), Some(sect.to_string()))
                                    }
                                    [proj] => (Some(proj.to_string()), None),
                                    _ => (None, None),
                                };

                                if project_id.is_none() {
                                    warn!("No project ID found in selection");
                                    return Ok(Json(InteractionResponse {
                                        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                                        data: Some(InteractionResponseData{
                                            content: Some(format!("{} No project ID found in selection.", Emojis::RED_X)),
                                            flags: Some(MessageFlags::EPHEMERAL),
                                            ..InteractionResponseData::default()
                                        })
                                    }));
                                }

                                let result = move_task(
                                    &state.todoist_client,
                                    MoveTask {
                                        task_id: task_id.to_string(),
                                        project_id: project_id.clone(),
                                        section_id: section_id.clone(),
                                        parent_id: None,
                                    },
                                )
                                .await;

                                match result {
                                    Ok(task) => {
                                        info!(
                                            "Moved task {} to project {:?} and section {:?}",
                                            task.id, project_id, section_id
                                        );
                                        return Ok(Json(InteractionResponse {
                                        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                                        data: Some(InteractionResponseData{
                                            content: Some(format!("{} Moved task to the selected project and section.", Emojis::GREEN_TICK)),
                                            flags: Some(MessageFlags::EPHEMERAL),
                                            ..InteractionResponseData::default()
                                        })
                                    }));
                                    }
                                    Err(e) => {
                                        error!("Error moving task: {}", e);
                                        return Ok(Json(InteractionResponse {
                                        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                                        data: Some(InteractionResponseData{
                                            content: Some(format!("{} Failed to move task: {}", Emojis::RED_X, e)),
                                            flags: Some(MessageFlags::EPHEMERAL),
                                            ..InteractionResponseData::default()
                                        })
                                    }));
                                    }
                                }
                            }
                            None => {
                                warn!("No task ID provided in section_select custom ID");
                                InteractionResponse {
                                    kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                                    data: Some(InteractionResponseData{
                                        content: Some("No task ID provided.".to_string()),
                                        flags: Some(MessageFlags::EPHEMERAL),
                                        ..InteractionResponseData::default()
                                    })
                                }
                            }
                        }
                    }
                    _ => {
                        warn!("No handler for message component command: {}", command);
                        InteractionResponse {
                            kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                            data: Some(InteractionResponseData{
                                content: Some(format!("No handler for message component command: `{}`", command)),
                                flags: Some(MessageFlags::EPHEMERAL),
                                ..InteractionResponseData::default()
                            })
                        }
                    }
                }
            } else {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
        _ => return Err(StatusCode::NOT_IMPLEMENTED),
    };

    let as_json = serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    debug!("Response JSON: {}", as_json);

    Ok(Json(resp))
}

async fn handle_response(
    mut handle: JoinHandle<Option<InteractionResponse>>,
    client: Arc<Client>,
    state: Arc<AppState>,
    interaction: Arc<Interaction>,
) -> Option<InteractionResponse> {
    select! {
        result = &mut handle => {
            match result {
                Ok(resp) => {
                    resp
                }
                Err(e) => {
                    Some(InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData{
                            content: Some(format!("{} An error occurred while processing your command: {}", Emojis::RED_X, e)),
                            flags: Some(MessageFlags::EPHEMERAL),
                            ..InteractionResponseData::default()
                        })
                    })
                }
            }
        }
        _  = time::sleep(Duration::from_secs(1)) => {
            debug!("Handler timed out. Returning deferred response.");
            tokio::spawn(async move {
                let resp = timeout(Duration::from_secs(10 * 60), handle).await.ok().and_then(|r| r.ok()).flatten();
                if let Some(response) = resp {
                    let data = response.data.unwrap_or_default();
                    let update_response = client
                        .interaction(state.app_id)
                        .update_response(&interaction.token)
                        .attachments(&data.attachments.unwrap_or_default())
                        .content(data.content.as_deref())
                        .embeds(data.embeds.as_deref())
                        .components(data.components.as_deref())
                        .await;

                    if let Err(e) = update_response {
                        error!(
                            "Failed to send delayed response: {}",
                            e
                        );
                    }
                }
            });
            Some(InteractionResponse {
                kind: InteractionResponseType::DeferredChannelMessageWithSource,
                data: Some(InteractionResponseData{
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..InteractionResponseData::default()
                })
            })
        }
    }
}
