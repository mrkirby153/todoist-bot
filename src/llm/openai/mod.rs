use std::sync::Arc;

use crate::llm::LLMProvider;
use crate::llm::PromptResponse;
use crate::llm::prompt::substitute_system_prompt;
use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use openai_api_rust::Auth;
use openai_api_rust::Message;
use openai_api_rust::OpenAI;
use openai_api_rust::Role;
use openai_api_rust::chat::ChatApi;
use openai_api_rust::chat::ChatBody;
use tokio::task::spawn_blocking;
use tracing::debug;
use tracing::info;

pub struct OpenAIProvider {
    client: Arc<OpenAI>,
    model: String,
    system_prompt: String,
}

impl OpenAIProvider {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_TOKEN")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_TOKEN environment variable not set"))?;
        let mut api_endpoint = std::env::var("OPENAI_API_ENDPOINT")
            .unwrap_or("https://api.openai.com/v1/".to_string());
        if !api_endpoint.ends_with('/') {
            api_endpoint.push('/');
        }
        let model = std::env::var("OPENAI_MODEL").unwrap_or("gpt-5-nano".to_string());
        info!(
            "Using OpenAI model: {} and endpoint: {}",
            model, api_endpoint
        );
        let auth = Auth::new(api_key.as_str());

        let system_prompt = {
            let path = std::env::var("OPENAI_SYSTEM_PROMPT_PATH");
            match path {
                Ok(p) => std::fs::read_to_string(p)
                    .map_err(|e| anyhow::anyhow!("Failed to read system prompt file: {}", e))?,
                Err(_) => include_str!("../claude/system_prompt.txt").to_string(),
            }
        };

        let client = OpenAI::new(auth, &api_endpoint);
        Ok(Self {
            client: Arc::new(client),
            model,
            system_prompt,
        })
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn generate_reminder(&self, user_input: &str) -> Result<PromptResponse> {
        debug!("Generating reminder with OpenAI for input: {}", user_input);
        let user_input = user_input.to_string();
        let client = self.client.clone();
        let system_prompt = substitute_system_prompt(&self.system_prompt);
        let model = self.model.clone();
        spawn_blocking(move || {
            let body = ChatBody {
            model: model.clone(),
            max_tokens: Some(1000),
            frequency_penalty: None,
            logit_bias: None,
            messages: vec![
                Message {
                    role: Role::System,
                    content: system_prompt.clone(),
                },
                Message {
                    role: Role::User,
                    content: format!(
                        "Create a reminder to add to my to-do list from the following message: {}",
                        user_input
                    ),
                },
            ],
            n: Some(1),
            presence_penalty: None,
            stop: None,
            user: None,
            stream: Some(false),
            temperature: None,
            top_p: None,
        };
        let rs = client.chat_completion_create(&body).map_err(|e| anyhow!(e))?;
        let choice = rs.choices;
        let message = &choice[0].message.as_ref().unwrap();
        debug!("OpenAI response message: {}", message.content);

        serde_json::from_str(message.content.as_str()).context("Failed to parse OpenAI response")
        }).await.unwrap()
    }
}
