//! Authentication strategies for the OpenRouter API.

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
    site_url: Option<String>,
    site_name: Option<String>,
}

impl ApiKeyAuth {
    /// Create a new API key authentication strategy.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            site_url: None,
            site_name: None,
        }
    }

    /// Set the site URL (sent as HTTP-Referer header).
    /// This helps OpenRouter track usage and may unlock higher rate limits.
    pub fn with_site_url(mut self, url: impl Into<String>) -> Self {
        self.site_url = Some(url.into());
        self
    }

    /// Set the site name (sent as X-Title header).
    /// This is displayed in OpenRouter's dashboard.
    pub fn with_site_name(mut self, name: impl Into<String>) -> Self {
        self.site_name = Some(name.into());
        self
    }
}

#[async_trait]
impl AuthStrategy for ApiKeyAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        let auth_value = format!("Bearer {}", self.api_key);
        headers.insert("Authorization", auth_value.parse().unwrap());

        if let Some(url) = &self.site_url {
            headers.insert("HTTP-Referer", url.parse().unwrap());
        }

        if let Some(name) = &self.site_name {
            headers.insert("X-Title", name.parse().unwrap());
        }

        Ok(())
    }
}
