use std::{env, sync::Arc};

use anyhow::Result;
use axum::Router;
use axum::routing::{get, post};
use dotenv::dotenv;
use thiserror::Error;
use tokio::net::TcpListener;
use tracing::info;
use twilight_http::Client;
use twilight_model::application::command::Command;
use twilight_model::id::Id;
use twilight_model::user::CurrentUser;

use crate::interactions::verifier::Verifier;
use crate::interactions::{ContextCommandBuilder, ContextCommands};

mod interactions;
mod routes;

#[derive(Clone)]
pub struct AppState {
    verifier: Arc<Verifier>,
    client: Arc<Client>,
}
#[derive(Debug, Error)]
enum Error {
    #[error("INTERACTION_KEY environment variable must be set")]
    MissingInteractionKey,
    #[error("BOT_TOKEN environment variable must be set")]
    MissingBotToken,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let interaction_key = env::var("INTERACTION_KEY").map_err(|_| Error::MissingInteractionKey)?;
    let verifier = Arc::new(Verifier::try_new(&interaction_key)?);

    let bot_token = env::var("BOT_TOKEN").map_err(|_| Error::MissingBotToken)?;
    let client = Arc::new(Client::new(bot_token));

    let state = AppState { verifier, client };

    let user = retrieve_current_user(&state.client).await?;
    info!("Logged in as {}#{}", user.name, user.discriminator);

    let commands = register_commands();

    let guild_id = env::var("GUILD_ID").ok();
    update_commands(&state.client, commands, guild_id).await?;

    let app = Router::new()
        .route("/_health", get(routes::health))
        .route("/interactions", post(routes::interaction_callback))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn retrieve_current_user(client: &Client) -> Result<CurrentUser> {
    Ok(client.current_user().await?.model().await?)
}

async fn update_commands(
    client: &Client,
    context_commands: ContextCommands,
    guild_id: Option<String>,
) -> Result<()> {
    let application_id = {
        let response = client.current_user_application().await?;
        response.model().await?.id
    };

    let client = client.interaction(application_id);

    match guild_id {
        Some(guild_id) => {
            info!("Updating guild commands for guild ID {}", guild_id);
            let guild_id = Id::new(guild_id.parse::<u64>()?);
            let commands: Vec<Command> = context_commands.into();
            client.set_guild_commands(guild_id, &commands).await?;
        }
        None => {
            info!("Updating global commands");
            let commands: Vec<Command> = context_commands.into();
            client.set_global_commands(&commands).await?;
        }
    }

    Ok(())
}

fn register_commands() -> ContextCommands {
    let mut context_commands = ContextCommands::default();

    context_commands.register(
        ContextCommandBuilder::new("Add Reminder")
            .description("Adds a reminder for the current message"),
        interactions::commands::add_reminder,
    );

    context_commands
}
