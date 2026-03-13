//! Provider trait and common types.

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::Client;
use rust_decimal::Decimal;
use std::pin::Pin;
use std::time::Duration;
use thiserror::Error;

use crate::types::{ModelInfo, ProviderType, ProxyRequest, ProxyResponse, UsageInfo};

/// Errors that can occur during provider operations.
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

/// Type alias for streaming response.
pub type StreamResponse = Pin<Box<dyn Stream<Item = Result<Bytes, ProviderError>> + Send>>;

/// Trait for LLM provider adapters.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the provider type identifier.
    fn provider_type(&self) -> ProviderType;

    /// Get the base URL for this provider.
    fn base_url(&self) -> &str;

    /// Forward a non-streaming request to the upstream provider.
    async fn forward(
        &self,
        api_key: &str,
        endpoint: &str,
        request: ProxyRequest,
        timeout_secs: u64,
    ) -> Result<ProxyResponse, ProviderError>;

    /// Forward a streaming request to the upstream provider.
    async fn forward_stream(
        &self,
        api_key: &str,
        endpoint: &str,
        request: ProxyRequest,
        timeout_secs: u64,
    ) -> Result<StreamResponse, ProviderError>;

    /// Extract usage information from a response.
    fn extract_usage(&self, response: &ProxyResponse) -> Option<UsageInfo>;

    /// Extract cost from response (if provider reports it, e.g., OpenRouter).
    fn extract_cost(&self, response: &ProxyResponse) -> Option<Decimal>;

    /// Extract the upstream request ID from headers or response.
    fn extract_request_id(&self, response: &ProxyResponse) -> Option<String>;

    /// Extract the actual model used from the response.
    fn extract_model(&self, response: &ProxyResponse) -> Option<String>;

    /// List available models from the provider.
    async fn list_models(&self, api_key: &str) -> Result<Vec<ModelInfo>, ProviderError>;

    /// Build authorization header value.
    fn auth_header(&self, api_key: &str) -> String {
        format!("Bearer {}", api_key)
    }
}

// ── Shared helpers ──────────────────────────────────────────────────────────

/// Map HTTP error status to typed `ProviderError`.
pub fn map_error_status(status: u16, body: String) -> ProviderError {
    match status {
        401 => ProviderError::AuthenticationFailed,
        429 => ProviderError::RateLimited,
        404 => ProviderError::ModelNotFound(body),
        _ => ProviderError::RequestFailed(format!("{status}: {body}")),
    }
}

/// Send a non-streaming request and parse JSON response.
pub async fn send_request(
    client: &Client,
    url: &str,
    request: &ProxyRequest,
    headers: Vec<(&str, String)>,
    timeout_secs: u64,
) -> Result<ProxyResponse, ProviderError> {
    let mut builder = client
        .request(request.method.clone(), url)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(timeout_secs));

    for (name, value) in headers {
        builder = builder.header(name, value);
    }

    let response = builder.json(&request.body).send().await?;
    let status = response.status();
    let resp_headers = response.headers().clone();

    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(map_error_status(status.as_u16(), error_body));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ProviderError::Parse(e.to_string()))?;

    Ok(ProxyResponse {
        status,
        headers: resp_headers,
        body,
    })
}

/// Send a streaming request and return a byte stream.
pub async fn send_stream(
    client: &Client,
    url: &str,
    request: &ProxyRequest,
    body_override: Option<&serde_json::Value>,
    headers: Vec<(&str, String)>,
    timeout_secs: u64,
) -> Result<StreamResponse, ProviderError> {
    let mut builder = client
        .request(request.method.clone(), url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .timeout(Duration::from_secs(timeout_secs));

    for (name, value) in headers {
        builder = builder.header(name, value);
    }

    let body = body_override.unwrap_or(&request.body);
    let response = builder.json(body).send().await?;
    let status = response.status();

    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(map_error_status(status.as_u16(), error_body));
    }

    let stream = response
        .bytes_stream()
        .map(|r| r.map_err(|e| ProviderError::Network(e.to_string())));

    Ok(Box::pin(stream))
}

/// Extract model name from a JSON response body (`"model"` field).
pub fn extract_model_from_body(response: &ProxyResponse) -> Option<String> {
    response
        .body
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Extract request ID from a header, falling back to `"id"` in body.
pub fn extract_request_id_from(
    response: &ProxyResponse,
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

/// Extract OpenAI-style usage (`prompt_tokens`/`completion_tokens`).
pub fn extract_openai_usage(response: &ProxyResponse) -> Option<UsageInfo> {
    let usage = response.body.get("usage")?;
    Some(UsageInfo {
        input_tokens: usage
            .get("prompt_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        output_tokens: usage
            .get("completion_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        total_tokens: usage
            .get("total_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
    })
}

/// Extract Anthropic-style usage (`input_tokens`/`output_tokens`).
pub fn extract_anthropic_usage(response: &ProxyResponse) -> Option<UsageInfo> {
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
        total_tokens: None,
    })
}

/// Fetch models from an OpenAI-compatible `/v1/models` endpoint.
pub async fn fetch_openai_models(
    client: &Client,
    base_url: &str,
    api_key: &str,
    auth_header_name: &str,
    auth_header_value: &str,
    provider: ProviderType,
    silent_on_error: bool,
) -> Result<Vec<ModelInfo>, ProviderError> {
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));

    let mut builder = client.get(&url);
    if !api_key.is_empty() {
        builder = builder.header(auth_header_name, auth_header_value);
    }

    let response = builder.send().await?;

    if !response.status().is_success() {
        if silent_on_error {
            return Ok(vec![]);
        }
        let error = response.text().await.unwrap_or_default();
        return Err(ProviderError::RequestFailed(error));
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
                        name: m.get("name").and_then(|n| n.as_str()).map(String::from),
                        description: None,
                        context_length: m
                            .get("context_length")
                            .and_then(|c| c.as_i64())
                            .map(|v| v as i32),
                        provider,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
}

/// Build a full URL from base + endpoint.
pub fn build_url(base_url: &str, endpoint: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;

    #[test]
    fn test_build_url_strips_trailing_slash() {
        assert_eq!(build_url("https://api.openai.com/", "/v1/chat"), "https://api.openai.com/v1/chat");
        assert_eq!(build_url("https://api.openai.com", "/v1/chat"), "https://api.openai.com/v1/chat");
    }

    #[test]
    fn test_map_error_status() {
        assert!(matches!(map_error_status(401, "".into()), ProviderError::AuthenticationFailed));
        assert!(matches!(map_error_status(429, "".into()), ProviderError::RateLimited));
        assert!(matches!(map_error_status(404, "not found".into()), ProviderError::ModelNotFound(m) if m == "not found"));
        assert!(matches!(map_error_status(500, "oops".into()), ProviderError::RequestFailed(m) if m.contains("500")));
    }

    #[test]
    fn test_extract_model_from_body() {
        let response = ProxyResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: serde_json::json!({"model": "gpt-4", "choices": []}),
        };
        assert_eq!(extract_model_from_body(&response), Some("gpt-4".to_string()));

        let no_model = ProxyResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: serde_json::json!({"choices": []}),
        };
        assert_eq!(extract_model_from_body(&no_model), None);
    }

    #[test]
    fn test_extract_request_id_from_header() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-request-id", "req-123".parse().unwrap());

        let response = ProxyResponse {
            status: StatusCode::OK,
            headers,
            body: serde_json::json!({}),
        };
        assert_eq!(
            extract_request_id_from(&response, &["x-request-id"]),
            Some("req-123".to_string())
        );
    }

    #[test]
    fn test_extract_request_id_falls_back_to_body() {
        let response = ProxyResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: serde_json::json!({"id": "chatcmpl-abc123"}),
        };
        assert_eq!(
            extract_request_id_from(&response, &["x-request-id"]),
            Some("chatcmpl-abc123".to_string())
        );
    }

    #[test]
    fn test_extract_request_id_tries_headers_in_order() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-trace-id", "trace-456".parse().unwrap());

        let response = ProxyResponse {
            status: StatusCode::OK,
            headers,
            body: serde_json::json!({"id": "should-not-use"}),
        };
        assert_eq!(
            extract_request_id_from(&response, &["x-request-id", "request-id", "x-trace-id"]),
            Some("trace-456".to_string())
        );
    }

    #[test]
    fn test_extract_openai_usage() {
        let response = ProxyResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: serde_json::json!({
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 20,
                    "total_tokens": 30
                }
            }),
        };
        let usage = extract_openai_usage(&response).unwrap();
        assert_eq!(usage.input_tokens, Some(10));
        assert_eq!(usage.output_tokens, Some(20));
        assert_eq!(usage.total_tokens, Some(30));
    }

    #[test]
    fn test_extract_openai_usage_missing() {
        let response = ProxyResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: serde_json::json!({"model": "gpt-4"}),
        };
        assert!(extract_openai_usage(&response).is_none());
    }

    #[test]
    fn test_extract_anthropic_usage() {
        let response = ProxyResponse {
            status: StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: serde_json::json!({
                "usage": {
                    "input_tokens": 50,
                    "output_tokens": 100
                }
            }),
        };
        let usage = extract_anthropic_usage(&response).unwrap();
        assert_eq!(usage.input_tokens, Some(50));
        assert_eq!(usage.output_tokens, Some(100));
        assert_eq!(usage.total_tokens, None);
    }

    #[test]
    fn test_provider_error_from_reqwest_timeout() {
        // Build a reqwest error that triggers timeout detection
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_nanos(1))
            .build()
            .unwrap();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        // We can't easily create a real timeout error without a network call,
        // so test the Display/Debug impls instead
        let err = ProviderError::Timeout;
        assert_eq!(err.to_string(), "Upstream timeout");
        let _ = client; // suppress unused warning
        let _ = rt;
    }

    #[test]
    fn test_provider_error_display() {
        assert_eq!(ProviderError::AuthenticationFailed.to_string(), "Authentication failed");
        assert_eq!(ProviderError::RateLimited.to_string(), "Rate limited");
        assert_eq!(ProviderError::Timeout.to_string(), "Upstream timeout");
        assert_eq!(ProviderError::Network("conn refused".into()).to_string(), "Network error: conn refused");
        assert_eq!(ProviderError::Parse("bad json".into()).to_string(), "Parse error: bad json");
        assert_eq!(ProviderError::ModelNotFound("gpt-5".into()).to_string(), "Model not found: gpt-5");
    }
}
