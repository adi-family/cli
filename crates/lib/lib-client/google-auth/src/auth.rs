use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::credentials::ServiceAccountCredentials;
use crate::error::Result;
use crate::token::{Token, TokenResponse, TokenStore};

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Authentication strategy for Google APIs.
#[async_trait]
pub trait AuthStrategy: Send + Sync {
    /// Apply authentication to request headers.
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()>;
}

/// API key authentication.
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    api_key: String,
}

impl ApiKeyAuth {
    /// Create new API key auth.
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

/// JWT claims for service account authentication.
#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    iss: String,
    sub: Option<String>,
    aud: String,
    iat: i64,
    exp: i64,
    scope: String,
}

/// Service account authentication using JWT.
pub struct ServiceAccountAuth {
    credentials: ServiceAccountCredentials,
    scopes: Vec<String>,
    subject: Option<String>,
    token: Arc<RwLock<Option<Token>>>,
    http: reqwest::Client,
}

impl ServiceAccountAuth {
    /// Create new service account auth.
    pub fn new(credentials: ServiceAccountCredentials, scopes: Vec<String>) -> Self {
        Self {
            credentials,
            scopes,
            subject: None,
            token: Arc::new(RwLock::new(None)),
            http: reqwest::Client::new(),
        }
    }

    /// Set subject (for domain-wide delegation).
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Create signed JWT assertion.
    fn create_jwt(&self) -> Result<String> {
        let now = Utc::now();
        let claims = JwtClaims {
            iss: self.credentials.client_email.clone(),
            sub: self.subject.clone(),
            aud: GOOGLE_TOKEN_URL.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::hours(1)).timestamp(),
            scope: self.scopes.join(" "),
        };

        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(self.credentials.private_key.as_bytes())?;
        Ok(encode(&header, &claims, &key)?)
    }

    /// Fetch access token using JWT assertion.
    async fn fetch_token(&self) -> Result<Token> {
        let jwt = self.create_jwt()?;

        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ];

        let response = self
            .http
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(crate::Error::TokenRefresh {
                message: format!("Status {}: {}", status, text),
            });
        }

        let token_resp: TokenResponse = response.json().await?;
        Ok(token_resp.into())
    }

    /// Get valid access token, refreshing if needed.
    async fn get_token(&self) -> Result<String> {
        {
            let token = self.token.read().await;
            if let Some(t) = token.as_ref() {
                if !t.is_expired() {
                    return Ok(t.access_token.clone());
                }
            }
        }

        let new_token = self.fetch_token().await?;
        let access_token = new_token.access_token.clone();

        let mut token = self.token.write().await;
        *token = Some(new_token);

        Ok(access_token)
    }
}

#[async_trait]
impl AuthStrategy for ServiceAccountAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        let token = self.get_token().await?;
        headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        Ok(())
    }
}

/// OAuth2 authentication with refresh token support.
pub struct OAuth2Auth {
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
    token_store: Option<Arc<dyn TokenStore>>,
    token: Arc<RwLock<Option<Token>>>,
    http: reqwest::Client,
}

impl OAuth2Auth {
    /// Create new OAuth2 auth.
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            scopes,
            token_store: None,
            token: Arc::new(RwLock::new(None)),
            http: reqwest::Client::new(),
        }
    }

    /// Set token store for persistence.
    pub fn with_token_store(mut self, store: Arc<dyn TokenStore>) -> Self {
        self.token_store = Some(store);
        self
    }

    /// Set initial token (e.g., from stored refresh token).
    pub fn with_token(mut self, token: Token) -> Self {
        self.token = Arc::new(RwLock::new(Some(token)));
        self
    }

    /// Generate authorization URL for user consent.
    pub fn authorization_url(&self, redirect_uri: &str, state: &str) -> String {
        let scope = self.scopes.join(" ");
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?\
            client_id={}&\
            redirect_uri={}&\
            response_type=code&\
            scope={}&\
            state={}&\
            access_type=offline&\
            prompt=consent",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scope),
            urlencoding::encode(state)
        )
    }

    /// Exchange authorization code for tokens.
    pub async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<Token> {
        let params = [
            ("code", code),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let response = self
            .http
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(crate::Error::AuthorizationFailed(format!(
                "Status {}: {}",
                status, text
            )));
        }

        let token_resp: TokenResponse = response.json().await?;
        let token: Token = token_resp.into();

        let mut stored = self.token.write().await;
        *stored = Some(token.clone());

        if let Some(store) = &self.token_store {
            store.store("google_oauth", &token).await?;
        }

        Ok(token)
    }

    /// Refresh access token using refresh token.
    async fn refresh_token(&self, refresh_token: &str) -> Result<Token> {
        let params = [
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .http
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(crate::Error::TokenRefresh {
                message: format!("Status {}: {}", status, text),
            });
        }

        let token_resp: TokenResponse = response.json().await?;
        let mut token: Token = token_resp.into();

        // Preserve refresh token if not returned
        if token.refresh_token.is_none() || token.refresh_token.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            token.refresh_token = Some(refresh_token.to_string());
        }

        Ok(token)
    }

    /// Get valid access token, refreshing if needed.
    async fn get_token(&self) -> Result<String> {
        {
            let token = self.token.read().await;
            if let Some(t) = token.as_ref() {
                if !t.is_expired() {
                    return Ok(t.access_token.clone());
                }
            }
        }

        let refresh_token = {
            let token = self.token.read().await;
            token
                .as_ref()
                .and_then(|t| t.refresh_token.clone())
                .ok_or(crate::Error::TokenExpired)?
        };

        let new_token = self.refresh_token(&refresh_token).await?;
        let access_token = new_token.access_token.clone();

        let mut token = self.token.write().await;
        *token = Some(new_token.clone());

        if let Some(store) = &self.token_store {
            store.store("google_oauth", &new_token).await?;
        }

        Ok(access_token)
    }
}

#[async_trait]
impl AuthStrategy for OAuth2Auth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        let token = self.get_token().await?;
        headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_auth() {
        let auth = ApiKeyAuth::new("test-key");
        assert_eq!(auth.api_key, "test-key");
    }

    #[test]
    fn test_authorization_url() {
        let auth = OAuth2Auth::new(
            "client-id",
            "client-secret",
            vec!["https://www.googleapis.com/auth/drive".to_string()],
        );
        let url = auth.authorization_url("http://localhost:8080/callback", "state123");
        assert!(url.contains("client_id=client-id"));
        assert!(url.contains("state=state123"));
    }
}
