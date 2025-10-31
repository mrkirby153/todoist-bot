use std::{sync::Arc, time::Duration};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use twilight_model::{
    application::interaction::InteractionData, channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
};
use twilight_model::{
    application::interaction::{Interaction, InteractionType},
    http::interaction::InteractionResponse,
};

use crate::{
    AppState,
    emoji::Emojis,
    interactions::commands::resolve_command_path,
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
            kind: twilight_model::http::interaction::InteractionResponseType::Pong,
            data: None,
        },
        InteractionType::ApplicationCommand => {
            if let Some(InteractionData::ApplicationCommand(ref command)) = interaction.data {
                let name = &command.name;
                debug!("Processing application command: {}", name);

                let resolved_slash_command = resolve_command_path(command);

                let slash_result = if let Some((command_path, command_data)) =
                    resolved_slash_command
                {
                    debug!("Resolved command path: {}", command_path);

                    let state = Arc::new(state.clone());
                    let callback_state = Arc::clone(&state);
                    let interaction = Arc::clone(&interaction);
                    let callback_interaction = Arc::clone(&interaction);

                    let mut handle = tokio::spawn(async move {
                        let inner_state = Arc::clone(&state);
                        state
                            .slash_commands
                            .execute(&command_path, interaction, command_data, inner_state)
                            .await
                    });

                    match timeout(Duration::from_secs(1), &mut handle).await {
                        Ok(Ok(resp)) => resp,
                        Ok(Err(e)) => {
                            error!("Error executing slash command: {}", e);
                            None
                        }
                        Err(_) => {
                            tokio::spawn(async move {
                                let state = callback_state;
                                let interaction = callback_interaction;
                                let resp = handle.await.unwrap_or(None);
                                if let Some(response) = resp {
                                    let data = response.data.unwrap_or_default();

                                    let update_response = state
                                        .client
                                        .interaction(state.app_id)
                                        .update_response(&interaction.token)
                                        .attachments(&data.attachments.unwrap_or_default())
                                        .content(data.content.as_deref())
                                        .embeds(data.embeds.as_deref())
                                        .components(data.components.as_deref())
                                        .await;

                                    if let Err(e) = update_response {
                                        error!(
                                            "Failed to send delayed slash command response: {}",
                                            e
                                        );
                                    }
                                }
                            });
                            Some(InteractionResponse {
                                kind: twilight_model::http::interaction::InteractionResponseType::DeferredChannelMessageWithSource,
                                data: Some(InteractionResponseData{
                                    flags: Some(MessageFlags::EPHEMERAL),
                                    ..InteractionResponseData::default()
                                })
                            })
                        }
                    }
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

                    let mut handle = tokio::spawn(async move { handler(interaction, state).await });

                    match timeout(Duration::from_secs(1), &mut handle).await {
                        Ok(Ok(resp)) => {
                            debug!("Command handler returned quickly.");
                            resp
                        }
                        Ok(Err(e)) => {
                            error!("Error executing command handler: {}", e);
                            InteractionResponse {
                                kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                                data: Some(InteractionResponseData{
                                    content: Some("An error occurred while processing your command.".to_string()),
                                    flags: Some(MessageFlags::EPHEMERAL),
                                    ..InteractionResponseData::default()
                                })
                            }
                        }
                        Err(_) => {
                            debug!("Command handler timed out");
                            tokio::spawn(async move {
                                debug!("Preparing to send delayed response");
                                let resp = handle.await.unwrap_or(InteractionResponse {
                                            kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                                            data: Some(InteractionResponseData{
                                                content: Some("An error occurred while processing your command.".to_string()),
                                                flags: Some(MessageFlags::EPHEMERAL),
                                                ..InteractionResponseData::default()
                                            })
                                        });
                                debug!("Sending delayed response");

                                let data = resp.data.unwrap_or_default();

                                debug!(
                                    "Delayed response: {}",
                                    serde_json::to_string_pretty(&data).unwrap_or_default()
                                );

                                let response = handler_state
                                    .client
                                    .interaction(handler_state.app_id)
                                    .update_response(&handler_interaction.token)
                                    .attachments(&data.attachments.unwrap_or_default())
                                    .content(data.content.as_deref())
                                    .embeds(data.embeds.as_deref())
                                    .components(data.components.as_deref())
                                    .flags(data.flags.unwrap_or(MessageFlags::empty()))
                                    .await;
                                if let Err(e) = response {
                                    error!("Failed to send delayed response: {}", e);
                                }
                            });
                            InteractionResponse {
                                kind: twilight_model::http::interaction::InteractionResponseType::DeferredChannelMessageWithSource,
                                data: Some(InteractionResponseData{
                                    flags: Some(MessageFlags::EPHEMERAL),
                                    ..InteractionResponseData::default()
                                })
                            }
                        }
                    }
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
