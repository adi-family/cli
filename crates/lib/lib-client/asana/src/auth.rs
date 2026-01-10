use async_trait::async_trait;
use reqwest::header::HeaderMap;

use crate::error::Result;

#[async_trait]
pub trait AuthStrategy: Send + Sync {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()>;
}

/// Bearer token authentication.
#[derive(Debug, Clone)]
pub struct BearerAuth {
    token: String,
}

impl BearerAuth {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for BearerAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.token).parse().unwrap(),
        );
        Ok(())
    }
}
