#![allow(dead_code)]
use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct MessageRequest {
    pub model: String,
    pub messages: Vec<InputMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct MessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub obj_type: String,
    pub role: String,
    pub content: Vec<OutputMessage>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: serde_json::Value,
    pub context_management: Option<serde_json::Value>,
    pub container: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InputMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub enum OutputMessage {
    #[serde(rename = "text")]
    Text {
        citations: Option<Vec<Citation>>,
        text: String,
    },
    #[serde(rename = "thinking")]
    Thinking { signature: String, thinking: String },
    #[serde(rename = "redacted_thinking")]
    RedactedThinking { data: String },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub enum Citation {
    #[serde(rename = "char_location")]
    CharacterLocation {
        cited_text: String,
        document_index: u32,
        document_title: Option<String>,
        end_char_index: u32,
        file_id: Option<String>,
        start_char_index: u32,
    },
    #[serde(rename = "page_location")]
    PageLocation {
        cited_text: String,
        document_index: u32,
        document_title: Option<String>,
        end_page_number: u32,
        file_id: Option<String>,
        start_page_number: u32,
    },
    #[serde(rename = "content_block_location")]
    ContentBlockLocation {
        cited_text: String,
        document_index: u32,
        document_title: Option<String>,
        end_block_index: u32,
        file_id: Option<String>,
        start_block_index: u32,
    },
    ResponseWebSearchResultLocationCitation {
        cited_text: String,
        encrypted_index: String,
        title: Option<String>,
    },
    ResponseSearchResultLocationCitation {
        cited_text: String,
        end_block_index: u32,
        search_result_index: u32,
        source: String,
        start_block_index: u32,
        title: Option<String>,
    },
}

impl Display for OutputMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputMessage::Text { text, .. } => {
                writeln!(f, "{}", text)?;
            }
            OutputMessage::Thinking {
                signature: _,
                thinking,
            } => {
                writeln!(f, "{}", thinking)?;
            }
            OutputMessage::RedactedThinking { data } => {
                writeln!(f, "{}", data)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Display for MessageResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let all = self
            .content
            .iter()
            .map(|c| format!("{}", c))
            .collect::<Vec<String>>();
        write!(f, "{}", all.join(" "))?;
        Ok(())
    }
}
