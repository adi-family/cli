//! Authentication strategies for the OpenAI API.

use crate::error::Result;
use async_trait::async_trait;
use reqwest::header::HeaderMap;

/// Authentication strategy trait.
#[async_trait]
pub trait AuthStrategy: Send + Sync {
    /// Apply authentication to the request headers.
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()>;
}

/// API key authentication (Bearer token).
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    api_key: String,
    organization: Option<String>,
}

impl ApiKeyAuth {
    /// Create a new API key authentication strategy.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            organization: None,
        }
    }

    /// Set the organization ID.
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }
}

#[async_trait]
impl AuthStrategy for ApiKeyAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        let auth_value = format!("Bearer {}", self.api_key);
        headers.insert("Authorization", auth_value.parse().unwrap());

        if let Some(org) = &self.organization {
            headers.insert("OpenAI-Organization", org.parse().unwrap());
        }

        Ok(())
    }
}
