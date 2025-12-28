use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Access token with expiration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// The access token string.
    pub access_token: String,

    /// Token type (usually "Bearer").
    pub token_type: String,

    /// Expiration time.
    pub expires_at: DateTime<Utc>,

    /// Refresh token (for OAuth2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// Scopes granted.
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl Token {
    /// Create a new token.
    pub fn new(access_token: String, expires_in_secs: i64) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
            expires_at: Utc::now() + Duration::seconds(expires_in_secs),
            refresh_token: None,
            scopes: Vec::new(),
        }
    }

    /// Check if the token is expired (with 60s buffer).
    pub fn is_expired(&self) -> bool {
        Utc::now() + Duration::seconds(60) >= self.expires_at
    }

    /// Set refresh token.
    pub fn with_refresh_token(mut self, token: String) -> Self {
        self.refresh_token = Some(token);
        self
    }

    /// Set scopes.
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }
}

/// Token response from Google OAuth2.
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

impl From<TokenResponse> for Token {
    fn from(resp: TokenResponse) -> Self {
        let scopes = resp
            .scope
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();

        Token::new(resp.access_token, resp.expires_in)
            .with_scopes(scopes)
            .with_refresh_token(resp.refresh_token.unwrap_or_default())
    }
}

/// Trait for persistent token storage.
#[async_trait]
pub trait TokenStore: Send + Sync {
    /// Load a stored token for the given key.
    async fn load(&self, key: &str) -> crate::Result<Option<Token>>;

    /// Store a token with the given key.
    async fn store(&self, key: &str, token: &Token) -> crate::Result<()>;

    /// Delete a stored token.
    async fn delete(&self, key: &str) -> crate::Result<()>;
}

/// In-memory token store (tokens lost on restart).
#[derive(Debug, Default)]
pub struct MemoryTokenStore {
    tokens: std::sync::RwLock<std::collections::HashMap<String, Token>>,
}

impl MemoryTokenStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TokenStore for MemoryTokenStore {
    async fn load(&self, key: &str) -> crate::Result<Option<Token>> {
        Ok(self.tokens.read().unwrap().get(key).cloned())
    }

    async fn store(&self, key: &str, token: &Token) -> crate::Result<()> {
        self.tokens
            .write()
            .unwrap()
            .insert(key.to_string(), token.clone());
        Ok(())
    }

    async fn delete(&self, key: &str) -> crate::Result<()> {
        self.tokens.write().unwrap().remove(key);
        Ok(())
    }
}
