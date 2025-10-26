use std::{env, sync::Arc};

use anyhow::Result;
use axum::Router;
use axum::routing::{get, post};
use dotenv::dotenv;
use thiserror::Error;
use todoist_bot::claude::ClaudeHttpClient;
use todoist_bot::{AppState, interactions, retrieve_current_user, routes};
use tokio::net::TcpListener;
use tracing::info;
use twilight_http::Client;
use twilight_model::application::command::Command;
use twilight_model::id::Id;

use todoist_bot::emoji::Emojis;
use todoist_bot::interactions::ContextCommands;
use todoist_bot::interactions::commands::CommandExecutor;
use todoist_bot::interactions::verifier::Verifier;
use todoist_bot::todoist::http::TodoistHttpClient;

#[derive(Debug, Error)]
enum MissingEnvironemntVariable {
    #[error("INTERACTION_KEY environment variable must be set")]
    InteractionKey,
    #[error("BOT_TOKEN environment variable must be set")]
    BotToken,
    #[error("TODOIST_API_TOKEN environment variable must be set")]
    TodoistApiToken,
    #[error("CLAUDE_API_TOKEN environment variable must be set")]
    ClaudeApiToken,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let todoist_token =
        env::var("TODOIST_API_TOKEN").map_err(|_| MissingEnvironemntVariable::TodoistApiToken)?;
    let todoist_client = Arc::new(TodoistHttpClient::new(&todoist_token));

    let claude_token =
        env::var("CLAUDE_API_TOKEN").map_err(|_| MissingEnvironemntVariable::ClaudeApiToken)?;
    let claude_client = Arc::new(ClaudeHttpClient::new(&claude_token, "claude-sonnet-4-5"));

    let interaction_key =
        env::var("INTERACTION_KEY").map_err(|_| MissingEnvironemntVariable::InteractionKey)?;
    let verifier = Arc::new(Verifier::try_new(&interaction_key)?);

    let bot_token = env::var("BOT_TOKEN").map_err(|_| MissingEnvironemntVariable::BotToken)?;
    let client = Arc::new(Client::new(bot_token));

    let (context_commands, slash_commands) = register_commands();
    let context_commands = Arc::new(context_commands);
    let slash_commands = Arc::new(slash_commands);

    let app_id = {
        let response = client.current_user_application().await?;
        response.model().await?.id
    };

    let state = AppState {
        app_id,
        verifier,
        client,
        context_commands,
        slash_commands,
        todoist_client,
        claude_client,
    };

    Emojis::initialize("emojis.json")?;

    let user = retrieve_current_user(&state.client).await?;
    info!("Logged in as {}#{}", user.name, user.discriminator);

    let guild_id = env::var("GUILD_ID").ok();
    update_commands(
        &state.client,
        &state.context_commands,
        &state.slash_commands,
        guild_id,
    )
    .await?;

    let app = Router::new()
        .route("/_health", get(routes::health))
        .route("/interactions", post(routes::interaction_callback))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn update_commands(
    client: &Client,
    context_commands: &ContextCommands<AppState>,
    slash_commands: &CommandExecutor<AppState>,
    guild_id: Option<String>,
) -> Result<()> {
    let application_id = {
        let response = client.current_user_application().await?;
        response.model().await?.id
    };

    let client = client.interaction(application_id);
    let mut commands: Vec<Command> = context_commands.into();
    commands.append(&mut slash_commands.build_commands());

    match guild_id {
        Some(guild_id) => {
            info!("Updating guild commands for guild ID {}", guild_id);
            let guild_id = Id::new(guild_id.parse::<u64>()?);
            client.set_guild_commands(guild_id, &commands).await?;
        }
        None => {
            info!("Updating global commands");
            client.set_global_commands(&commands).await?;
        }
    }

    Ok(())
}

fn register_commands() -> (ContextCommands<AppState>, CommandExecutor<AppState>) {
    let mut context_commands = ContextCommands::new();

    context_commands.register("Add To-Do", interactions::command_handlers::add_reminder);

    let mut command_executor = CommandExecutor::new();
    command_executor.register(interactions::command_handlers::handle_today);

    (context_commands, command_executor)
}
