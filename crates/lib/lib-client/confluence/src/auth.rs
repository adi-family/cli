use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::header::HeaderMap;

use crate::error::Result;

#[async_trait]
pub trait AuthStrategy: Send + Sync {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()>;
}

/// Basic authentication (email:api_token).
#[derive(Debug, Clone)]
pub struct BasicAuth {
    email: String,
    api_token: String,
}

impl BasicAuth {
    pub fn new(email: impl Into<String>, api_token: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            api_token: api_token.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for BasicAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        let credentials = format!("{}:{}", self.email, self.api_token);
        let encoded = STANDARD.encode(credentials);
        headers.insert(
            "Authorization",
            format!("Basic {}", encoded).parse().unwrap(),
        );
        Ok(())
    }
}
