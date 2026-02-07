//! Anthropic Messages API provider adapter.

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use rust_decimal::Decimal;
use std::time::Duration;

use super::traits::{LlmProvider, ProviderError, StreamResponse};
use crate::types::{ModelInfo, ProviderType, ProxyRequest, ProxyResponse, UsageInfo};

/// Default Anthropic API base URL.
const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";

/// Current Anthropic API version.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic Messages API provider.
pub struct AnthropicProvider {
    client: Client,
    base_url: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider.
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| ANTHROPIC_BASE_URL.to_string()),
        }
    }

    /// Build the full URL for an endpoint.
    fn build_url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), endpoint)
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn auth_header(&self, api_key: &str) -> String {
        // Anthropic uses x-api-key header, not Bearer token
        api_key.to_string()
    }

    async fn forward(
        &self,
        api_key: &str,
        endpoint: &str,
        request: ProxyRequest,
        timeout_secs: u64,
    ) -> Result<ProxyResponse, ProviderError> {
        let url = self.build_url(endpoint);

        let response = self
            .client
            .request(request.method.clone(), &url)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(timeout_secs))
            .json(&request.body)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => ProviderError::AuthenticationFailed,
                429 => ProviderError::RateLimited,
                404 => ProviderError::ModelNotFound(error_body),
                _ => ProviderError::RequestFailed(format!("{}: {}", status, error_body)),
            });
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        Ok(ProxyResponse {
            status,
            headers,
            body,
        })
    }

    async fn forward_stream(
        &self,
        api_key: &str,
        endpoint: &str,
        request: ProxyRequest,
        timeout_secs: u64,
    ) -> Result<StreamResponse, ProviderError> {
        let url = self.build_url(endpoint);

        // Ensure stream is enabled in the request body
        let mut body = request.body.clone();
        if let Some(obj) = body.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::json!(true));
        }

        let response = self
            .client
            .request(request.method.clone(), &url)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .timeout(Duration::from_secs(timeout_secs))
            .json(&body)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => ProviderError::AuthenticationFailed,
                429 => ProviderError::RateLimited,
                404 => ProviderError::ModelNotFound(error_body),
                _ => ProviderError::RequestFailed(format!("{}: {}", status, error_body)),
            });
        }

        let stream = response
            .bytes_stream()
            .map(|result: Result<bytes::Bytes, reqwest::Error>| {
                result.map_err(|e| ProviderError::Network(e.to_string()))
            });

        Ok(Box::pin(stream))
    }

    fn extract_usage(&self, response: &ProxyResponse) -> Option<UsageInfo> {
        let usage = response.body.get("usage")?;

        Some(UsageInfo {
            input_tokens: usage
                .get("input_tokens")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
            output_tokens: usage
                .get("output_tokens")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
            total_tokens: None, // Anthropic doesn't provide total, we compute it
        })
    }

    fn extract_cost(&self, _response: &ProxyResponse) -> Option<Decimal> {
        // Anthropic doesn't report cost in response
        None
    }

    fn extract_request_id(&self, response: &ProxyResponse) -> Option<String> {
        // Anthropic uses request-id header
        response
            .headers
            .get("request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .or_else(|| {
                response
                    .body
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
    }

    fn extract_model(&self, response: &ProxyResponse) -> Option<String> {
        response
            .body
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    async fn list_models(&self, _api_key: &str) -> Result<Vec<ModelInfo>, ProviderError> {
        // Anthropic doesn't have a models endpoint, return known models
        Ok(vec![
            ModelInfo {
                id: "claude-3-5-sonnet-20241022".to_string(),
                name: Some("Claude 3.5 Sonnet".to_string()),
                description: Some("Most intelligent model".to_string()),
                context_length: Some(200000),
                provider: ProviderType::Anthropic,
            },
            ModelInfo {
                id: "claude-3-5-haiku-20241022".to_string(),
                name: Some("Claude 3.5 Haiku".to_string()),
                description: Some("Fastest model".to_string()),
                context_length: Some(200000),
                provider: ProviderType::Anthropic,
            },
            ModelInfo {
                id: "claude-3-opus-20240229".to_string(),
                name: Some("Claude 3 Opus".to_string()),
                description: Some("Most capable model".to_string()),
                context_length: Some(200000),
                provider: ProviderType::Anthropic,
            },
            ModelInfo {
                id: "claude-3-sonnet-20240229".to_string(),
                name: Some("Claude 3 Sonnet".to_string()),
                description: Some("Balanced model".to_string()),
                context_length: Some(200000),
                provider: ProviderType::Anthropic,
            },
            ModelInfo {
                id: "claude-3-haiku-20240307".to_string(),
                name: Some("Claude 3 Haiku".to_string()),
                description: Some("Fast and compact".to_string()),
                context_length: Some(200000),
                provider: ProviderType::Anthropic,
            },
        ])
    }
}
