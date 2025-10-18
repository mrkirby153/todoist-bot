use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use tracing::debug;
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

    let interaction: Interaction =
        serde_json::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    let resp = match interaction.kind {
        InteractionType::Ping => InteractionResponse {
            kind: twilight_model::http::interaction::InteractionResponseType::Pong,
            data: None,
        },
        _ => return Err(StatusCode::NOT_IMPLEMENTED),
    };
    Ok(Json(resp))
}
