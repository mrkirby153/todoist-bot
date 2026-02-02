pub mod claude;

use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
pub struct PromptResponse {
    pub title: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due: Option<OffsetDateTime>,
    pub links: Option<Vec<String>>,
}

pub type Provider = dyn LLMProvider + Send + Sync;

#[async_trait]
pub trait LLMProvider {
    async fn generate_reminder(&self, user_input: &str) -> Result<PromptResponse>;
}
