use async_trait::async_trait;
use reqwest::header::HeaderMap;

use crate::error::Result;

#[async_trait]
pub trait AuthStrategy: Send + Sync {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()>;
}

/// Private token authentication.
#[derive(Debug, Clone)]
pub struct PrivateTokenAuth {
    token: String,
}

impl PrivateTokenAuth {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for PrivateTokenAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert("PRIVATE-TOKEN", self.token.parse().unwrap());
        Ok(())
    }
}

/// OAuth2 bearer token authentication.
#[derive(Debug, Clone)]
pub struct OAuthAuth {
    token: String,
}

impl OAuthAuth {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for OAuthAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert(
            "Authorization",
            format!("Bearer {}", self.token).parse().unwrap(),
        );
        Ok(())
    }
}

/// Job token authentication (for CI/CD).
#[derive(Debug, Clone)]
pub struct JobTokenAuth {
    token: String,
}

impl JobTokenAuth {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for JobTokenAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert("JOB-TOKEN", self.token.parse().unwrap());
        Ok(())
    }
}
