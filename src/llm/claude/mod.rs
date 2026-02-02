use anyhow::{Context, Result};
use axum::http::HeaderValue;
use reqwest::Client;
use tracing::debug;

use crate::llm::{
    LLMProvider, PromptResponse,
    claude::{
        models::{InputMessage, MessageRequest, MessageResponse},
        prompt::get_system_prompt,
    },
};

pub struct ClaudeHttpClient {
    client: Client,
    pub model: String,
}

pub mod models;
pub mod prompt;

const CLAUDE_API_BASE_URL: &str = "https://api.anthropic.com/v1";

impl ClaudeHttpClient {
    pub fn new(api_token: &str, model: &str) -> Self {
        let client = Client::builder()
            .user_agent("todoist-bot/0.1")
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert("x-api-key", HeaderValue::from_str(api_token).unwrap());
                headers.insert(
                    "anthropic-version",
                    HeaderValue::from_str("2023-06-01").unwrap(),
                );
                headers
            })
            .build()
            .unwrap();
        Self {
            client,
            model: model.to_string(),
        }
    }

    pub fn get(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.get(self.make_url(url))
    }

    pub fn post(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.post(self.make_url(url))
    }

    pub fn delete(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.delete(self.make_url(url))
    }

    fn make_url(&self, endpoint: &str) -> String {
        if !endpoint.starts_with("/") {
            format!("{}/{}", CLAUDE_API_BASE_URL, endpoint)
        } else {
            format!("{}{}", CLAUDE_API_BASE_URL, endpoint)
        }
    }
}

pub async fn message_create(
    client: &ClaudeHttpClient,
    request: MessageRequest,
) -> Result<MessageResponse> {
    debug!("Sending Claude message request: {:#?}", request);
    let response = client.post("/messages").json(&request).send().await?;
    let text = response.text().await?;
    debug!("Claude response: {}", text);

    let message_response: MessageResponse = serde_json::from_str(&text)?;
    Ok(message_response)
}

#[async_trait::async_trait]
impl LLMProvider for ClaudeHttpClient {
    async fn generate_reminder(&self, user_input: &str) -> Result<PromptResponse> {
        debug!(
            "Generating reminder from user input with Claude: {}",
            user_input
        );
        let response = message_create(
            self,
            MessageRequest {
                model: self.model.clone(),
                messages: vec![InputMessage {
                    role: "user".to_string(),
                    content: format!(
                        "Create a reminder to add to my to-do list from the following message: {}",
                        user_input
                    ),
                }],
                max_tokens: 1000,
                system: Some(get_system_prompt()),
            },
        )
        .await?;

        let response_str: String = response.into();
        debug!("Claude generated response: {}", response_str);

        serde_json::from_str(&response_str).context("Failed to decode response from claude")
    }
}
