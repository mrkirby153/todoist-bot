use std::sync::Arc;

use anyhow::Result;
use twilight_http::Client;
use twilight_model::user::CurrentUser;

use crate::interactions::ContextCommands;
use crate::interactions::commands::CommandExecutor;
use crate::interactions::verifier::Verifier;
use crate::todoist::http::TodoistHttpClient;

pub mod emoji;
pub mod interactions;
pub mod routes;
pub mod todoist;

#[derive(Clone)]
pub struct AppState {
    pub verifier: Arc<Verifier>,
    pub client: Arc<Client>,
    pub context_commands: Arc<ContextCommands<AppState>>,
    pub slash_commands: Arc<CommandExecutor<AppState>>,
    pub todoist_client: Arc<TodoistHttpClient>,
}

/// Gets the current user associated with the provided Discord client.
pub async fn retrieve_current_user(client: &Client) -> Result<CurrentUser> {
    Ok(client.current_user().await?.model().await?)
}
