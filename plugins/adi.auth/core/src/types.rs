use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing)]
    pub totp_secret: Option<String>,
    pub login: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub is_anonymous: bool,
}

impl User {
    pub fn new(email: &str) -> Self {
        Self {
            id: UserId::new(),
            email: email.to_lowercase(),
            created_at: Utc::now(),
            last_login_at: None,
            totp_secret: None,
            login: None,
            password_hash: None,
            is_anonymous: false,
        }
    }

    pub fn has_totp(&self) -> bool {
        self.totp_secret.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct VerificationCode {
    pub email: String,
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl VerificationCode {
    pub fn new(email: &str, expiry_minutes: i64) -> Self {
        let code = generate_code();
        let now = Utc::now();
        Self {
            email: email.to_lowercase(),
            code,
            created_at: now,
            expires_at: now + chrono::Duration::minutes(expiry_minutes),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn matches(&self, code: &str) -> bool {
        self.code == code
    }
}

fn generate_code() -> String {
    format!("{:06}", fastrand::u32(0..1_000_000))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub is_anonymous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousCredentials {
    pub login: String,
    pub password: String,
    pub token: AuthToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpSetup {
    pub secret: String,
    pub otpauth_url: String,
    pub qr_code_base64: String,
}
