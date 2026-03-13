//! OpenAI-compatible provider adapter.
//!
//! Works with OpenAI API, OpenRouter, and other compatible endpoints.
//! Also serves as the base for Custom providers (OpenAI-compatible with optional auth).

use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;

use super::traits::{
    self, LlmProvider, ProviderError, StreamResponse,
};
use crate::types::{ModelInfo, ProviderType, ProxyRequest, ProxyResponse, UsageInfo};

const OPENAI_BASE_URL: &str = "https://api.openai.com";
const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api";

/// OpenAI-compatible provider (also used for OpenRouter and Custom endpoints).
pub struct OpenAIProvider {
    client: Client,
    base_url: String,
    provider_type: ProviderType,
    /// When true, skip auth header if api_key is empty (Custom mode).
    optional_auth: bool,
}

impl OpenAIProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| OPENAI_BASE_URL.to_string()),
            provider_type: ProviderType::OpenAI,
            optional_auth: false,
        }
    }

    pub fn openrouter(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| OPENROUTER_BASE_URL.to_string()),
            provider_type: ProviderType::OpenRouter,
            optional_auth: false,
        }
    }

    /// Create a custom OpenAI-compatible provider with optional auth.
    pub fn custom(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            provider_type: ProviderType::Custom,
            optional_auth: true,
        }
    }

    fn auth_headers(&self, api_key: &str) -> Vec<(&str, String)> {
        if self.optional_auth && api_key.is_empty() {
            vec![]
        } else {
            vec![("Authorization", self.auth_header(api_key))]
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    fn provider_type(&self) -> ProviderType {
        self.provider_type
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
        traits::send_stream(&self.client, &url, &request, None, self.auth_headers(api_key), timeout_secs).await
    }

    fn extract_usage(&self, response: &ProxyResponse) -> Option<UsageInfo> {
        traits::extract_openai_usage(response)
    }

    fn extract_cost(&self, response: &ProxyResponse) -> Option<Decimal> {
        // Only OpenRouter reports cost in response
        if self.provider_type == ProviderType::OpenRouter {
            response
                .body
                .get("usage")
                .and_then(|u| u.get("cost"))
                .and_then(|c| c.as_f64())
                .and_then(|f| Decimal::try_from(f).ok())
        } else {
            None
        }
    }

    fn extract_request_id(&self, response: &ProxyResponse) -> Option<String> {
        let headers = if self.optional_auth {
            // Custom: try multiple header names
            &["x-request-id", "request-id", "x-trace-id"][..]
        } else {
            &["x-request-id"][..]
        };
        traits::extract_request_id_from(response, headers)
    }

    fn extract_model(&self, response: &ProxyResponse) -> Option<String> {
        traits::extract_model_from_body(response)
    }

    async fn list_models(&self, api_key: &str) -> Result<Vec<ModelInfo>, ProviderError> {
        let auth_value = self.auth_header(api_key);
        traits::fetch_openai_models(
            &self.client,
            &self.base_url,
            api_key,
            "Authorization",
            &auth_value,
            self.provider_type,
            self.optional_auth, // Custom: silent on error
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_defaults() {
        let provider = OpenAIProvider::new(None);
        assert_eq!(provider.provider_type(), ProviderType::OpenAI);
        assert_eq!(provider.base_url(), "https://api.openai.com");
        assert!(!provider.optional_auth);
    }

    #[test]
    fn test_openai_custom_base_url() {
        let provider = OpenAIProvider::new(Some("https://my-proxy.example.com".to_string()));
        assert_eq!(provider.base_url(), "https://my-proxy.example.com");
    }

    #[test]
    fn test_openrouter_defaults() {
        let provider = OpenAIProvider::openrouter(None);
        assert_eq!(provider.provider_type(), ProviderType::OpenRouter);
        assert_eq!(provider.base_url(), "https://openrouter.ai/api");
    }

    #[test]
    fn test_custom_provider_optional_auth() {
        let provider = OpenAIProvider::custom("http://localhost:1234".to_string());
        assert_eq!(provider.provider_type(), ProviderType::Custom);
        assert!(provider.optional_auth);
        assert_eq!(provider.base_url(), "http://localhost:1234");
    }

    #[test]
    fn test_auth_headers_skipped_when_empty_custom() {
        let provider = OpenAIProvider::custom("http://localhost:1234".to_string());
        let headers = provider.auth_headers("");
        assert!(headers.is_empty());
    }

    #[test]
    fn test_auth_headers_present_when_key_given_custom() {
        let provider = OpenAIProvider::custom("http://localhost:1234".to_string());
        let headers = provider.auth_headers("sk-test");
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "Authorization");
        assert_eq!(headers[0].1, "Bearer sk-test");
    }

    #[test]
    fn test_auth_headers_always_present_for_openai() {
        let provider = OpenAIProvider::new(None);
        let headers = provider.auth_headers("");
        assert_eq!(headers.len(), 1); // Still sends auth even for empty key
    }

    #[test]
    fn test_extract_cost_openrouter_only() {
        let provider_openai = OpenAIProvider::new(None);
        let provider_openrouter = OpenAIProvider::openrouter(None);

        let response = ProxyResponse {
            status: http::StatusCode::OK,
            headers: http::HeaderMap::new(),
            body: serde_json::json!({
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 20,
                    "cost": 0.0015
                }
            }),
        };

        assert!(provider_openai.extract_cost(&response).is_none());
        assert!(provider_openrouter.extract_cost(&response).is_some());
    }

    #[test]
    fn test_extract_request_id_custom_tries_multiple_headers() {
        let provider = OpenAIProvider::custom("http://localhost".to_string());

        let mut headers = http::HeaderMap::new();
        headers.insert("x-trace-id", "trace-789".parse().unwrap());

        let response = ProxyResponse {
            status: http::StatusCode::OK,
            headers,
            body: serde_json::json!({}),
        };
        assert_eq!(provider.extract_request_id(&response), Some("trace-789".to_string()));
    }

    #[test]
    fn test_extract_request_id_openai_only_x_request_id() {
        let provider = OpenAIProvider::new(None);

        let mut headers = http::HeaderMap::new();
        headers.insert("x-trace-id", "trace-789".parse().unwrap());

        let response = ProxyResponse {
            status: http::StatusCode::OK,
            headers,
            body: serde_json::json!({}),
        };
        // x-trace-id not checked for OpenAI, falls back to body
        assert!(provider.extract_request_id(&response).is_none());
    }
}
