//! Anthropic Messages API provider adapter.

use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;

use super::traits::{self, LlmProvider, ProviderError, StreamResponse};
use crate::types::{ModelInfo, ProviderType, ProxyRequest, ProxyResponse, UsageInfo};

const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic Messages API provider.
pub struct AnthropicProvider {
    client: Client,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| ANTHROPIC_BASE_URL.to_string()),
        }
    }

    fn auth_headers(&self, api_key: &str) -> Vec<(&str, String)> {
        vec![
            ("x-api-key", api_key.to_string()),
            ("anthropic-version", ANTHROPIC_VERSION.to_string()),
        ]
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
        api_key.to_string()
    }

    async fn forward(
        &self,
        api_key: &str,
        endpoint: &str,
        request: ProxyRequest,
        timeout_secs: u64,
    ) -> Result<ProxyResponse, ProviderError> {
        let url = traits::build_url(&self.base_url, endpoint);
        traits::send_request(&self.client, &url, &request, self.auth_headers(api_key), timeout_secs).await
    }

    async fn forward_stream(
        &self,
        api_key: &str,
        endpoint: &str,
        request: ProxyRequest,
        timeout_secs: u64,
    ) -> Result<StreamResponse, ProviderError> {
        let url = traits::build_url(&self.base_url, endpoint);

        // Anthropic requires `stream: true` in the request body for SSE
        let mut body = request.body.clone();
        if let Some(obj) = body.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::json!(true));
        }

        traits::send_stream(&self.client, &url, &request, Some(&body), self.auth_headers(api_key), timeout_secs).await
    }

    fn extract_usage(&self, response: &ProxyResponse) -> Option<UsageInfo> {
        traits::extract_anthropic_usage(response)
    }

    fn extract_cost(&self, _response: &ProxyResponse) -> Option<Decimal> {
        None
    }

    fn extract_request_id(&self, response: &ProxyResponse) -> Option<String> {
        traits::extract_request_id_from(response, &["request-id"])
    }

    fn extract_model(&self, response: &ProxyResponse) -> Option<String> {
        traits::extract_model_from_body(response)
    }

    async fn list_models(&self, _api_key: &str) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(vec![
            model("claude-sonnet-4-6-20250514", "Claude Sonnet 4.6", 200_000),
            model("claude-opus-4-6-20250514", "Claude Opus 4.6", 200_000),
            model("claude-haiku-4-5-20251001", "Claude Haiku 4.5", 200_000),
            model("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet", 200_000),
            model("claude-3-5-haiku-20241022", "Claude 3.5 Haiku", 200_000),
            model("claude-3-opus-20240229", "Claude 3 Opus", 200_000),
        ])
    }
}

fn model(id: &str, name: &str, ctx: i32) -> ModelInfo {
    ModelInfo {
        id: id.to_string(),
        name: Some(name.to_string()),
        description: None,
        context_length: Some(ctx),
        provider: ProviderType::Anthropic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_provider_defaults() {
        let provider = AnthropicProvider::new(None);
        assert_eq!(provider.provider_type(), ProviderType::Anthropic);
        assert_eq!(provider.base_url(), "https://api.anthropic.com");
    }

    #[test]
    fn test_anthropic_custom_base_url() {
        let provider = AnthropicProvider::new(Some("https://my-anthropic.example.com".to_string()));
        assert_eq!(provider.base_url(), "https://my-anthropic.example.com");
    }

    #[test]
    fn test_auth_header_is_raw_key() {
        let provider = AnthropicProvider::new(None);
        // Anthropic uses x-api-key header, not Bearer
        assert_eq!(provider.auth_header("sk-ant-test"), "sk-ant-test");
    }

    #[test]
    fn test_auth_headers_include_version() {
        let provider = AnthropicProvider::new(None);
        let headers = provider.auth_headers("sk-ant-test");
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0], ("x-api-key", "sk-ant-test".to_string()));
        assert_eq!(headers[1].0, "anthropic-version");
    }

    #[test]
    fn test_extract_cost_always_none() {
        let provider = AnthropicProvider::new(None);
        let response = ProxyResponse {
            status: http::StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: serde_json::json!({"usage": {"input_tokens": 10, "output_tokens": 20}}),
        };
        assert!(provider.extract_cost(&response).is_none());
    }

    #[test]
    fn test_extract_usage_anthropic_style() {
        let provider = AnthropicProvider::new(None);
        let response = ProxyResponse {
            status: http::StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: serde_json::json!({
                "usage": {"input_tokens": 42, "output_tokens": 77}
            }),
        };
        let usage = provider.extract_usage(&response).unwrap();
        assert_eq!(usage.input_tokens, Some(42));
        assert_eq!(usage.output_tokens, Some(77));
        assert_eq!(usage.total_tokens, None);
    }

    #[test]
    fn test_extract_request_id_uses_request_id_header() {
        let provider = AnthropicProvider::new(None);

        let mut headers = http::HeaderMap::new();
        headers.insert("request-id", "req-anthropic-123".parse().unwrap());

        let response = ProxyResponse {
            status: http::StatusCode::OK,
            headers,
            body: serde_json::json!({}),
        };
        assert_eq!(provider.extract_request_id(&response), Some("req-anthropic-123".to_string()));
    }

    #[tokio::test]
    async fn test_list_models_returns_known_models() {
        let provider = AnthropicProvider::new(None);
        let models = provider.list_models("unused").await.unwrap();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id.contains("claude-sonnet-4")));
        assert!(models.iter().all(|m| m.provider == ProviderType::Anthropic));
        assert!(models.iter().all(|m| m.context_length == Some(200_000)));
    }
}
