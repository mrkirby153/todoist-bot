use twilight_model::{
    application::interaction::Interaction, http::interaction::InteractionResponse,
};

use crate::AppState;

pub async fn add_reminder(_interaction: Interaction, _state: AppState) -> InteractionResponse {
    todo!();
}
