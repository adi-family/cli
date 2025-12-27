use crate::error::{Error, Result};
use crate::types::{AuthToken, TokenClaims, User};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

const DEFAULT_EXPIRY_HOURS: i64 = 24 * 7;

pub struct TokenManager {
    secret: String,
    expiry_hours: i64,
}

impl TokenManager {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
            expiry_hours: DEFAULT_EXPIRY_HOURS,
        }
    }

    pub fn from_env() -> Self {
        let secret = std::env::var("JWT_SECRET")
            .expect("JWT_SECRET environment variable is required");

        let expiry_hours = std::env::var("JWT_EXPIRY_HOURS")
            .ok()
            .and_then(|h| h.parse().ok())
            .unwrap_or(DEFAULT_EXPIRY_HOURS);

        Self {
            secret,
            expiry_hours,
        }
    }

    pub fn generate_token(&self, user: &User) -> Result<AuthToken> {
        let now = Utc::now().timestamp();
        let exp = now + (self.expiry_hours * 3600);

        let claims = TokenClaims {
            sub: user.id.0.to_string(),
            email: user.email.clone(),
            exp,
            iat: now,
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
        };

        let token = manager.generate_token(&user).unwrap();
        assert_eq!(token.token_type, "Bearer");

        let claims = manager.verify_token(&token.access_token).unwrap();
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.sub, user.id.0.to_string());
    }

    #[test]
    fn test_invalid_token() {
        let manager = TokenManager::new("test-secret");
        let result = manager.verify_token("invalid-token");
        assert!(matches!(result, Err(Error::InvalidToken(_))));
    }
}
