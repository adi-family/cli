use serde::{Deserialize, Serialize};

/// Google service account credentials from JSON key file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccountCredentials {
    /// Service account type (should be "service_account").
    #[serde(rename = "type")]
    pub account_type: String,

    /// Project ID.
    pub project_id: String,

    /// Private key ID.
    pub private_key_id: String,

    /// Private key in PEM format.
    pub private_key: String,

    /// Service account email.
    pub client_email: String,

    /// Client ID.
    pub client_id: String,

    /// Auth URI.
    pub auth_uri: String,

    /// Token URI.
    pub token_uri: String,

    /// Auth provider certificate URL.
    pub auth_provider_x509_cert_url: String,

    /// Client certificate URL.
    pub client_x509_cert_url: String,

    /// Universe domain.
    #[serde(default = "default_universe_domain")]
    pub universe_domain: String,
}

fn default_universe_domain() -> String {
    "googleapis.com".to_string()
}

impl ServiceAccountCredentials {
    /// Load credentials from a JSON file.
    pub async fn from_file(path: &str) -> crate::Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        Self::from_json(&content)
    }

    /// Parse credentials from JSON string.
    pub fn from_json(json: &str) -> crate::Result<Self> {
        let creds: Self = serde_json::from_str(json)?;
        if creds.account_type != "service_account" {
            return Err(crate::Error::InvalidCredentials(format!(
                "Expected type 'service_account', got '{}'",
                creds.account_type
            )));
        }
        Ok(creds)
    }
}

/// Unified credentials type for Google authentication.
#[derive(Debug, Clone)]
pub enum Credentials {
    /// API key authentication.
    ApiKey(String),

    /// Service account credentials.
    ServiceAccount(ServiceAccountCredentials),

    /// OAuth2 credentials (client ID + secret).
    OAuth2 {
        client_id: String,
        client_secret: String,
        refresh_token: Option<String>,
    },
}

impl Credentials {
    /// Create API key credentials.
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey(key.into())
    }

    /// Create service account credentials from JSON.
    pub fn service_account(creds: ServiceAccountCredentials) -> Self {
        Self::ServiceAccount(creds)
    }

    /// Create OAuth2 credentials.
    pub fn oauth2(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self::OAuth2 {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            refresh_token: None,
        }
    }

    /// Create OAuth2 credentials with refresh token.
    pub fn oauth2_with_refresh(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        refresh_token: impl Into<String>,
    ) -> Self {
        Self::OAuth2 {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            refresh_token: Some(refresh_token.into()),
        }
    }
}
