use crate::error::{Error, Result};
use crate::types::{AuthToken, TokenClaims, User};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use std::collections::HashSet;

const DEFAULT_EXPIRY_HOURS: i64 = 24 * 7;

pub struct TokenManager {
    secret: String,
    expiry_hours: i64,
    admin_emails: HashSet<String>,
}

impl TokenManager {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
            expiry_hours: DEFAULT_EXPIRY_HOURS,
            admin_emails: HashSet::new(),
        }
    }

    pub fn from_env() -> Self {
        use lib_env_parse::{env_opt, env_or, env_vars};
        env_vars! {
            JwtSecret    => "JWT_SECRET",
            JwtExpiryH   => "JWT_EXPIRY_HOURS",
            AdminEmails  => "ADMIN_EMAILS",
        }

        let secret =
            env_opt(EnvVar::JwtSecret.as_str()).expect("JWT_SECRET environment variable is required");

        let expiry_hours = env_opt(EnvVar::JwtExpiryH.as_str())
            .and_then(|h| h.parse().ok())
            .unwrap_or(DEFAULT_EXPIRY_HOURS);

        let admin_emails: HashSet<String> = env_or(EnvVar::AdminEmails.as_str(), "")
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            secret,
            expiry_hours,
            admin_emails,
        }
    }

    pub fn is_admin(&self, email: &str) -> bool {
        self.admin_emails.contains(&email.to_lowercase())
    }

    pub fn generate_token(&self, user: &User) -> Result<AuthToken> {
        let now = Utc::now().timestamp();
        let exp = now + (self.expiry_hours * 3600);
        let is_admin = self.is_admin(&user.email);

        let claims = TokenClaims {
            sub: user.id.0.to_string(),
            email: user.email.clone(),
            exp,
            iat: now,
            is_admin,
            is_anonymous: user.is_anonymous,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| Error::InvalidToken(e.to_string()))?;

        Ok(AuthToken {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: self.expiry_hours * 3600,
        })
    }

    /// Issue a short-lived subtoken for delegated operations (e.g. cocoon setup).
    pub fn generate_subtoken(&self, claims: &TokenClaims, ttl_seconds: i64) -> Result<AuthToken> {
        let now = Utc::now().timestamp();
        let exp = now + ttl_seconds;

        let sub_claims = TokenClaims {
            sub: claims.sub.clone(),
            email: claims.email.clone(),
            exp,
            iat: now,
            is_admin: claims.is_admin,
            is_anonymous: claims.is_anonymous,
        };

        let token = encode(
            &Header::default(),
            &sub_claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| Error::InvalidToken(e.to_string()))?;

        Ok(AuthToken {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: ttl_seconds,
        })
    }

    pub fn verify_token(&self, token: &str) -> Result<TokenClaims> {
        let token_data = decode::<TokenClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => Error::TokenExpired,
            _ => Error::InvalidToken(e.to_string()),
        })?;

        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UserId;

    #[test]
    fn test_token_generation_and_verification() {
        let manager = TokenManager::new("test-secret");

        let user = User {
            id: UserId::new(),
            email: "test@example.com".to_string(),
            created_at: Utc::now(),
            last_login_at: None,
            totp_secret: None,
            login: None,
            password_hash: None,
            is_anonymous: false,
        };

        let token = manager.generate_token(&user).unwrap();
        assert_eq!(token.token_type, "Bearer");

        let claims = manager.verify_token(&token.access_token).unwrap();
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.sub, user.id.0.to_string());
        assert!(!claims.is_admin);
        assert!(!claims.is_anonymous);
    }

    #[test]
    fn test_admin_token() {
        let mut manager = TokenManager::new("test-secret");
        manager.admin_emails.insert("admin@example.com".to_string());

        let user = User {
            id: UserId::new(),
            email: "admin@example.com".to_string(),
            created_at: Utc::now(),
            last_login_at: None,
            totp_secret: None,
            login: None,
            password_hash: None,
            is_anonymous: false,
        };

        let token = manager.generate_token(&user).unwrap();
        let claims = manager.verify_token(&token.access_token).unwrap();
        assert!(claims.is_admin);
    }

    #[test]
    fn test_invalid_token() {
        let manager = TokenManager::new("test-secret");
        let result = manager.verify_token("invalid-token");
        assert!(matches!(result, Err(Error::InvalidToken(_))));
    }
}
