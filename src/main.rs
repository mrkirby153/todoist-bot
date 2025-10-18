use std::{env, sync::Arc};

use anyhow::Result;
use axum::Router;
use axum::routing::{get, post};
use dotenv::dotenv;
use thiserror::Error;
use tokio::net::TcpListener;
use tracing::info;
use twilight_http::Client;
use twilight_model::user::CurrentUser;

use crate::interactions::verifier::Verifier;

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
