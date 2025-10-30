use std::io::Write;
use std::{
    env::var,
    fs::{File, read_dir},
    path::Path,
};

const FILE_TEMPLATE: &str = "
use std::{collections::HashMap, fs::read_to_string};

use once_cell::sync::OnceCell;
use thiserror::Error;
use anyhow::Result;
use std::fmt::Display;
use twilight_model::id::Id;
use twilight_model::id::marker::EmojiMarker;

static EMOJI_MAP: OnceCell<HashMap<String, String>> = OnceCell::new();

pub struct Emojis;

#[derive(Error, Debug)]
pub enum Error {
    #[error(\"Failed to initialize emoji map. Already initialized?\")]
    InitializationError,
    #[error(\"Failed to read emoji file: {0}\")]
    JsonParseError(serde_json::Error),
}

pub struct Emoji(&'static str, &'static str);

impl Emojis {
    $$EMOJIS$$

    pub fn initialize(file_path: &str) -> Result<()> {
        let json = read_to_string(file_path)?;
        let map: HashMap<String, String> =
            serde_json::from_str(&json).map_err(Error::JsonParseError)?;
        EMOJI_MAP.set(map).map_err(|_| Error::InitializationError)?;
        Ok(())
    }
}

impl Display for Emoji {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let map = EMOJI_MAP.get().expect(\"Emoji map not initialized.\");
        if let Some(id) = map.get(self.0) {
            write!(f, \"<:{}:{}>\", self.0, id)
        } else {
            panic!(\"Emoji ID not found for {}\", self.1);
        }
    }
}

impl Emoji {
    pub fn name(&self) -> &str {
        self.0
    }

    pub fn id(&self) -> Id<EmojiMarker> {
        Id::new(EMOJI_MAP.get().expect(\"Emoji map not initialized.\").get(self.0).expect(\"Emoji ID not found.\").parse().expect(\"Invalid Emoji ID.\"))
    }
}
";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let emoji_folder = Path::new("emoji");

    // Get all files in the emojis folder
    let entries = read_dir(emoji_folder).expect("Failed to read emojis directory");
    println!("cargo:rerun-if-changed={}", emoji_folder.display());

    let mut emojis: Vec<String> = Vec::new();

    for entry in entries {
        let entry = entry.expect("Failed to read emoji file entry");
        let path = entry.path();

        if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            let const_name = stem.to_uppercase();
            emojis.push(format!(
                "pub const {}: Emoji = Emoji(\"{}\", \"{}\");",
                const_name, stem, file_name
            ));
        }
    }

    let emojis_code = emojis.join("\n");
    let file_content = FILE_TEMPLATE.replace("$$EMOJIS$$", &emojis_code);
    let out_path = var("OUT_DIR").expect("OUT_DIR not set");

    let mut file =
        File::create(Path::new(&out_path).join("emojis.rs")).expect("Failed to create emojis.rs");

    // Write the generated content to the file
    file.write_all(file_content.as_bytes())
        .expect("Failed to write to emojis.rs");
}
