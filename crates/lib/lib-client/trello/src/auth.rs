use async_trait::async_trait;
use reqwest::header::HeaderMap;

use crate::error::Result;

#[async_trait]
pub trait AuthStrategy: Send + Sync {
    fn query_params(&self) -> Vec<(&'static str, String)>;

    async fn apply(&self, _headers: &mut HeaderMap) -> Result<()> {
        Ok(())
    }
}

/// API key + token authentication (query params).
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    api_key: String,
    token: String,
}

impl ApiKeyAuth {
    pub fn new(api_key: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            token: token.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for ApiKeyAuth {
    fn query_params(&self) -> Vec<(&'static str, String)> {
        vec![("key", self.api_key.clone()), ("token", self.token.clone())]
    }
}
