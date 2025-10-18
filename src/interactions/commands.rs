use std::sync::Arc;

use twilight_model::{
    application::interaction::Interaction, http::interaction::InteractionResponse,
};

use crate::AppState;

pub async fn add_reminder(
    _interaction: Arc<Interaction>,
    _state: Arc<AppState>,
) -> InteractionResponse {
    todo!();
}
