use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use tracing::{debug, warn};
use twilight_model::{
    application::interaction::InteractionData::ApplicationCommand, channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
};
use twilight_model::{
    application::interaction::{Interaction, InteractionType},
    http::interaction::InteractionResponse,
};

use crate::AppState;

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

                let slash_command_result = state
                    .slash_commands
                    .execute(name, Arc::clone(&interaction), Arc::new(state.clone()))
                    .await;
                if let Some(response) = slash_command_result {
                    response
                } else if let Some(handler) = state.context_commands.get(name) {
                    handler(Arc::clone(&interaction), Arc::new(state.clone())).await
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
    Ok(Json(resp))
}
