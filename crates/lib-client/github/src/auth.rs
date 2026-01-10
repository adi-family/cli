use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

use crate::error::Result;

#[async_trait]
pub trait AuthStrategy: Send + Sync {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()>;
}

pub struct NoAuth;

#[async_trait]
impl AuthStrategy for NoAuth {
    async fn apply(&self, _headers: &mut HeaderMap) -> Result<()> {
        Ok(())
    }
}

pub fn no_auth() -> NoAuth {
    NoAuth
}

pub struct TokenAuth {
    token: String,
}

impl TokenAuth {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for TokenAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        let value = format!("Bearer {}", self.token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&value).expect("valid header value"),
        );
        Ok(())
    }
}

pub struct BasicAuth {
    username: String,
    password: String,
}

impl BasicAuth {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

#[async_trait]
impl AuthStrategy for BasicAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = STANDARD.encode(credentials.as_bytes());
        let value = format!("Basic {}", encoded);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&value).expect("valid header value"),
        );
        Ok(())
    }
}

pub fn token(token: impl Into<String>) -> TokenAuth {
    TokenAuth::new(token)
}

pub fn basic(username: impl Into<String>, password: impl Into<String>) -> BasicAuth {
    BasicAuth::new(username, password)
}
