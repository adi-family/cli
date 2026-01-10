use async_trait::async_trait;
use reqwest::header::HeaderMap;

use crate::error::Result;

#[async_trait]
pub trait AuthStrategy: Send + Sync {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    api_key: String,
}

impl ApiKeyAuth {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for ApiKeyAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert("x-goog-api-key", self.api_key.parse().unwrap());
        Ok(())
    }
}
