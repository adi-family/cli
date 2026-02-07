//! Custom provider adapter for user-defined endpoints.
//!
//! Assumes OpenAI-compatible API structure.

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use rust_decimal::Decimal;
use std::time::Duration;

use super::traits::{LlmProvider, ProviderError, StreamResponse};
use crate::types::{ModelInfo, ProviderType, ProxyRequest, ProxyResponse, UsageInfo};

/// Custom provider for user-defined endpoints.
pub struct CustomProvider {
    client: Client,
    base_url: String,
}

impl CustomProvider {
    /// Create a new custom provider with the given base URL.
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Build the full URL for an endpoint.
    fn build_url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), endpoint)
    }
}

#[async_trait]
impl LlmProvider for CustomProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Custom
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    async fn forward(
        &self,
        api_key: &str,
        endpoint: &str,
        request: ProxyRequest,
        timeout_secs: u64,
    ) -> Result<ProxyResponse, ProviderError> {
        let url = self.build_url(endpoint);

        let mut req_builder = self
            .client
            .request(request.method.clone(), &url)
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(timeout_secs));

        // Only add auth header if api_key is not empty
        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", self.auth_header(api_key));
        }

        let response = req_builder.json(&request.body).send().await?;

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

        let mut req_builder = self
            .client
            .request(request.method.clone(), &url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .timeout(Duration::from_secs(timeout_secs));

        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", self.auth_header(api_key));
        }

        let response = req_builder.json(&request.body).send().await?;

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
        // Try OpenAI-style usage object
        let usage = response.body.get("usage")?;

        Some(UsageInfo {
            input_tokens: usage
                .get("prompt_tokens")
                .or_else(|| usage.get("input_tokens"))
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
            output_tokens: usage
                .get("completion_tokens")
                .or_else(|| usage.get("output_tokens"))
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
            total_tokens: usage
                .get("total_tokens")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
        })
    }

    fn extract_cost(&self, response: &ProxyResponse) -> Option<Decimal> {
        // Try to find cost in various places
        response
            .body
            .get("usage")
            .and_then(|u| u.get("cost"))
            .or_else(|| response.body.get("cost"))
            .and_then(|c| c.as_f64())
            .and_then(|f| Decimal::try_from(f).ok())
    }

    fn extract_request_id(&self, response: &ProxyResponse) -> Option<String> {
        // Try various header names
        for header_name in &["x-request-id", "request-id", "x-trace-id"] {
            if let Some(id) = response
                .headers
                .get(*header_name)
                .and_then(|v| v.to_str().ok())
            {
                return Some(id.to_string());
            }
        }

        // Fall back to response body
        response
            .body
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn extract_model(&self, response: &ProxyResponse) -> Option<String> {
        response
            .body
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    async fn list_models(&self, api_key: &str) -> Result<Vec<ModelInfo>, ProviderError> {
        // Try to fetch from /v1/models endpoint
        let url = self.build_url("/v1/models");

        let mut req_builder = self.client.get(&url);
        if !api_key.is_empty() {
            req_builder = req_builder.header("Authorization", self.auth_header(api_key));
        }

        let response = req_builder.send().await?;

        if !response.status().is_success() {
            // Custom endpoints might not have a models endpoint
            return Ok(vec![]);
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let models = body
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        let id = m.get("id")?.as_str()?;
                        Some(ModelInfo {
                            id: id.to_string(),
                            name: m
                                .get("name")
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string()),
                            description: None,
                            context_length: m
                                .get("context_length")
                                .and_then(|c| c.as_i64())
                                .map(|v| v as i32),
                            provider: ProviderType::Custom,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}
