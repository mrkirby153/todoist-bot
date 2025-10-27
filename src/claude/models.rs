#![allow(dead_code)]
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
    id: String,
    #[serde(rename = "type")]
    obj_type: String,
    role: String,
    content: Vec<OutputMessage>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: serde_json::Value,
    context_management: Option<serde_json::Value>,
    container: Option<serde_json::Value>,
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
    Thinking { signature: String, tinking: String },
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
        doucment_title: Option<String>,
        end_char_index: u32,
        file_id: Option<String>,
        start_char_index: u32,
    },
    #[serde(rename = "page_location")]
    PageLocation {
        cited_text: String,
        document_index: u32,
        doucment_title: Option<String>,
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
