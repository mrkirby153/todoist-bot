use std::{sync::Arc, time::Duration};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use tokio::time::timeout;
use tracing::{debug, error, warn};
use twilight_model::{
    application::interaction::InteractionData::ApplicationCommand, channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
};
use twilight_model::{
    application::interaction::{Interaction, InteractionType},
    http::interaction::InteractionResponse,
};

use crate::{AppState, interactions::commands::resolve_command_path};

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
            if let Some(ApplicationCommand(ref command)) = interaction.data {
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
        _ => return Err(StatusCode::NOT_IMPLEMENTED),
    };

    let as_json = serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    debug!("Response JSON: {}", as_json);

    Ok(Json(resp))
}
