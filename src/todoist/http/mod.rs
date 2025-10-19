use std::fmt::Debug;

use axum::http::HeaderMap;
use reqwest::Client;

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

    fn make_url(&self, endpoint: &str) -> String {
        if !endpoint.starts_with("/") {
            format!("{}/{}", TODOIST_API_BASE_URL, endpoint)
        } else {
            format!("{}{}", TODOIST_API_BASE_URL, endpoint)
        }
    }
}
