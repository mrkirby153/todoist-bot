use std::{collections::HashMap, env, fs::File, io::BufWriter, path::Path};

use anyhow::{Result, anyhow};
use clap::Parser;
use dotenv::dotenv;
use thiserror::Error;
use todoist_bot::retrieve_current_user;
use tokio::fs::read_dir;
use twilight_http::Client;
use twilight_model::{
    guild::Emoji,
    id::{Id, marker::ApplicationMarker},
};

use base64::{Engine as _, engine::general_purpose};
use color_print::cprintln;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    app_id: Option<String>,
    #[arg(short, long)]
    out_file: Option<String>,
    #[arg(short, long)]
    in_dir: String,
    #[arg(short, long)]
    content_type: Option<String>,
}

#[derive(Error, Debug)]
enum Error {
    #[error("BOT_TOKEN environment variable must be set")]
    NoBotToken,
    #[error("Failed to read emoji directory: {0}")]
    DirectoryRead(#[from] std::io::Error),
    #[error("Invalid file name")]
    InvalidFileName,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let args = Args::parse();

    let in_path = Path::new(&args.in_dir);

    cprintln!(
        "Generating emoji file from directory: <cyan>{}</cyan>",
        in_path.display()
    );

    let bot_token = env::var("BOT_TOKEN").map_err(|_| Error::NoBotToken)?;
    let client = Client::new(bot_token);

    let current_user = retrieve_current_user(&client)
        .await
        .map_err(|e| anyhow!(e))?;

    cprintln!(
        "Updating emojis for the user: <cyan>{}#{}</cyan> (ID: <cyan>{}</cyan>)",
        current_user.name,
        current_user.discriminator,
        current_user.id
    );

    let application_id: Id<ApplicationMarker> = client
        .current_user_application()
        .await
        .map_err(|e| anyhow!(e))?
        .model()
        .await?
        .id;

    let current_emojis = client
        .get_application_emojis(application_id)
        .await?
        .model()
        .await?;

    let mut emojis: HashMap<String, Emoji> = HashMap::new();

    current_emojis.items.iter().for_each(|emoji| {
        emojis.insert(emoji.name.clone(), emoji.clone());
    });

    cprintln!(
        "Found <yellow>{}</yellow> existing emojis.",
        current_emojis.items.len()
    );

    let mut existing_emojis: HashMap<String, Emoji> = HashMap::new();

    // Create new emojis
    let mut entries = read_dir(in_path).await.map_err(Error::DirectoryRead)?;
    while let Some(entry) = entries.next_entry().await? {
        let entry = entry;
        let path = entry.path();

        if path.is_file() {
            let file_name = path
                .file_stem()
                .and_then(|os_str| os_str.to_str())
                .ok_or(Error::InvalidFileName)?;

            if let Some(e) = emojis.get(file_name) {
                existing_emojis.insert(file_name.to_string(), e.clone());
                continue;
            }
            let image_data = tokio::fs::read(&path).await?;
            let encoded = general_purpose::STANDARD.encode(&image_data);

            let content_type = if let Some(ct) = &args.content_type {
                ct
            } else {
                "image/png"
            };
            let resp = client
                .add_application_emoji(
                    application_id,
                    file_name,
                    &format!("data:{};base64,{}", content_type, encoded),
                )
                .await?
                .model()
                .await?;
            cprintln!(
                "[<green>+</green>] Created emoji <cyan>{}</cyan>",
                file_name
            );
            existing_emojis.insert(file_name.to_string(), resp);
        }
    }

    // Remove old emojis
    for emoji in current_emojis.items.iter() {
        if existing_emojis.contains_key(emoji.name.as_str()) {
            continue;
        }
        cprintln!(
            "[<red>-</red>] Deleting emoji <cyan>{}</cyan> (ID: <cyan>{}</cyan>).",
            emoji.name,
            emoji.id
        );
        client
            .delete_application_emoji(application_id, emoji.id)
            .await?;
    }

    // Write to output file
    let out_file = args.out_file.unwrap_or("emojis.json".to_string());
    let out_file = Path::new(&out_file);
    let mut writer = BufWriter::new(File::create(out_file)?);

    let to_write = existing_emojis
        .iter()
        .map(|(name, emoji)| (name, emoji.id))
        .collect::<HashMap<_, _>>();

    serde_json::to_writer(&mut writer, &to_write)?;
    cprintln!("Wrote emojis to file: <cyan>{}</cyan>", out_file.display());
    Ok(())
}
