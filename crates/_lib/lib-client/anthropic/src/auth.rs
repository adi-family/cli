//! Authentication strategies for the Anthropic API.

use crate::error::Result;
use async_trait::async_trait;
use reqwest::header::HeaderMap;

/// Authentication strategy trait.
#[async_trait]
pub trait AuthStrategy: Send + Sync {
    /// Apply authentication to the request headers.
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()>;
}

/// API key authentication (x-api-key header).
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    api_key: String,
}

impl ApiKeyAuth {
    /// Create a new API key authentication strategy.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for ApiKeyAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert("x-api-key", self.api_key.parse().unwrap());
        Ok(())
    }
}
