use std::{collections::HashMap, fmt::Debug};

use anyhow::Result;
use axum::http::HeaderMap;
use reqwest::Client;
use serde::de::DeserializeOwned;
use tracing::debug;
use url::Url;

use crate::todoist::http::models::CursorResponse;

const TODOIST_API_BASE_URL: &str = "https://api.todoist.com/api/v1";

pub mod models;

#[derive(Debug)]
pub struct TodoistHttpClient {
    client: reqwest::Client,
}

#[allow(dead_code)]
impl TodoistHttpClient {
    pub fn new(token: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        let client = Client::builder()
            .user_agent("todoist-bot/0.1")
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client }
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

    pub async fn get_all<T>(&self, url: &str) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let url = self.make_url(url);
        let mut url = Url::parse(&url)?;
        let mut cursor: Option<String> = None;

        let mut results = Vec::new();

        loop {
            if let Some(c) = &cursor.take() {
                let mut query_pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                query_pairs.insert("cursor".to_string(), c.to_string());
                url.query_pairs_mut().clear().extend_pairs(query_pairs);
            }
            let url_str = url.as_str();
            debug!("Fetching URL: {}", url);
            let resp = self.client.get(url_str).send().await?;
            let resp_json: CursorResponse<T> = resp.json().await?;

            results.extend(resp_json.results);
            cursor = resp_json.next_cursor;
            debug!("Next cursor: {:?}", cursor);

            if cursor.is_none() {
                break;
            }
        }

        Ok(results)
    }

    fn make_url(&self, endpoint: &str) -> String {
        if !endpoint.starts_with("/") {
            format!("{}/{}", TODOIST_API_BASE_URL, endpoint)
        } else {
            format!("{}{}", TODOIST_API_BASE_URL, endpoint)
        }
    }
}
