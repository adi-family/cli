use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;
use std::time::Duration;
use thiserror::Error;

use crate::types::{EmbedModelInfo, EmbedResponse, EmbedUsageInfo, ProviderType};

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Rate limited")]
    RateLimited,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Upstream timeout")]
    Timeout,

    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

impl From<reqwest::Error> for ProviderError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ProviderError::Timeout
        } else if err.is_connect() {
            ProviderError::Network(err.to_string())
        } else {
            ProviderError::RequestFailed(err.to_string())
        }
    }
}

/// Trait for embedding provider adapters.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn provider_type(&self) -> ProviderType;
    fn base_url(&self) -> &str;

    /// Forward an embedding request to the upstream provider.
    async fn embed(
        &self,
        api_key: &str,
        body: serde_json::Value,
        timeout_secs: u64,
    ) -> Result<EmbedResponse, ProviderError>;

    /// Extract usage information from a response.
    fn extract_usage(&self, response: &EmbedResponse) -> Option<EmbedUsageInfo>;

    /// Extract cost from response (if provider reports it).
    fn extract_cost(&self, response: &EmbedResponse) -> Option<Decimal>;

    /// Extract the upstream request ID.
    fn extract_request_id(&self, response: &EmbedResponse) -> Option<String>;

    /// Extract the actual model used from the response.
    fn extract_model(&self, response: &EmbedResponse) -> Option<String>;

    /// List available embedding models from the provider.
    async fn list_models(&self, api_key: &str) -> Result<Vec<EmbedModelInfo>, ProviderError>;

    fn auth_header(&self, api_key: &str) -> String {
        format!("Bearer {}", api_key)
    }
}

// -- Shared helpers --

pub fn map_error_status(status: u16, body: String) -> ProviderError {
    match status {
        401 => ProviderError::AuthenticationFailed,
        429 => ProviderError::RateLimited,
        404 => ProviderError::ModelNotFound(body),
        _ => ProviderError::RequestFailed(format!("{status}: {body}")),
    }
}

pub async fn send_embed_request(
    client: &Client,
    url: &str,
    body: &serde_json::Value,
    headers: Vec<(&str, String)>,
    timeout_secs: u64,
) -> Result<EmbedResponse, ProviderError> {
    let mut builder = client
        .post(url)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(timeout_secs));

    for (name, value) in headers {
        builder = builder.header(name, value);
    }

    let response = builder.json(body).send().await?;
    let status = response.status();
    let resp_headers = response.headers().clone();

    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(map_error_status(status.as_u16(), error_body));
    }

    let resp_body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ProviderError::Parse(e.to_string()))?;

    Ok(EmbedResponse {
        status,
        headers: resp_headers,
        body: resp_body,
    })
}

pub fn extract_model_from_body(response: &EmbedResponse) -> Option<String> {
    response
        .body
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
}

pub fn extract_request_id_from(
    response: &EmbedResponse,
    header_names: &[&str],
) -> Option<String> {
    for name in header_names {
        if let Some(id) = response.headers.get(*name).and_then(|v| v.to_str().ok()) {
            return Some(id.to_string());
        }
    }
    response
        .body
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
}

pub fn extract_openai_embed_usage(response: &EmbedResponse) -> Option<EmbedUsageInfo> {
    let usage = response.body.get("usage")?;
    Some(EmbedUsageInfo {
        input_tokens: usage
            .get("prompt_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        total_tokens: usage
            .get("total_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
    })
}

pub fn build_url(base_url: &str, endpoint: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), endpoint)
}
