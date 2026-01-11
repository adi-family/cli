//! Provider trait and common types.

use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use rust_decimal::Decimal;
use std::pin::Pin;
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
