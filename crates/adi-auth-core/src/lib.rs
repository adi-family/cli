pub mod email;
pub mod error;
mod migrations;
pub mod storage;
pub mod token;
pub mod types;

#[cfg(feature = "axum")]
pub mod middleware;

pub use email::{EmailSender, SmtpConfig};
pub use error::{Error, Result};
pub use storage::{AuthStorage, SqliteAuthStorage};
pub use token::TokenManager;
pub use types::{AuthToken, TokenClaims, User, UserId, VerificationCode};

#[cfg(feature = "axum")]
pub use middleware::{AuthError, AuthUser, OptionalAuthUser};

use std::path::{Path, PathBuf};
use std::sync::Arc;

const CODE_EXPIRY_MINUTES: i64 = 10;

pub struct AuthManager {
    storage: Arc<dyn AuthStorage>,
    email: EmailSender,
    tokens: TokenManager,
}

impl AuthManager {
    pub fn open(db_path: &Path) -> Result<Self> {
        let storage = SqliteAuthStorage::open(db_path)?;

        Ok(Self {
            storage: Arc::new(storage),
            email: EmailSender::from_env(),
            tokens: TokenManager::from_env(),
        })
    }

    pub fn open_global() -> Result<Self> {
        let global_dir = Self::global_path();
        std::fs::create_dir_all(&global_dir)?;

        Self::open(&global_dir.join("auth.sqlite"))
    }

    pub fn global_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("adi")
            .join("auth")
    }

    pub fn with_email(mut self, email: EmailSender) -> Self {
        self.email = email;
        self
    }

    pub fn with_tokens(mut self, tokens: TokenManager) -> Self {
        self.tokens = tokens;
        self
    }

    pub fn request_code(&self, email: &str) -> Result<()> {
        self.storage.cleanup_expired_codes()?;
        self.storage.delete_verification_codes(email)?;

        let code = VerificationCode::new(email, CODE_EXPIRY_MINUTES);
        self.storage.store_verification_code(&code)?;
        self.email.send_verification_code(email, &code.code)?;

        tracing::info!("Verification code sent to {}", email);
        Ok(())
    }

    pub fn verify_code(&self, email: &str, code: &str) -> Result<AuthToken> {
        let stored_code = self
            .storage
            .get_verification_code(email)?
            .ok_or(Error::InvalidCode)?;

        if stored_code.is_expired() {
            self.storage.delete_verification_codes(email)?;
            return Err(Error::CodeExpired);
        }

        if !stored_code.matches(code) {
            return Err(Error::InvalidCode);
        }

        self.storage.delete_verification_codes(email)?;

        let user = match self.storage.get_user_by_email(email)? {
            Some(user) => {
                self.storage.update_last_login(user.id)?;
                user
            }
            None => {
                let user = User::new(email);
                self.storage.create_user(&user)?;
                tracing::info!("Created new user: {}", email);
                user
            }
        };

        let token = self.tokens.generate_token(&user)?;
        tracing::info!("User logged in: {}", email);

        Ok(token)
    }

    pub fn verify_token(&self, token: &str) -> Result<TokenClaims> {
        self.tokens.verify_token(token)
    }

    pub fn get_user(&self, user_id: UserId) -> Result<User> {
        self.storage.get_user_by_id(user_id)
    }

    pub fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        self.storage.get_user_by_email(email)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_auth_flow() {
        let dir = tempdir().unwrap();
        let manager = AuthManager::open(&dir.path().join("auth.sqlite")).unwrap();

        manager.request_code("test@example.com").unwrap();

        let code = manager
            .storage
            .get_verification_code("test@example.com")
            .unwrap()
            .unwrap();

        let token = manager.verify_code("test@example.com", &code.code).unwrap();
        assert_eq!(token.token_type, "Bearer");

        let claims = manager.verify_token(&token.access_token).unwrap();
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_invalid_code() {
        let dir = tempdir().unwrap();
        let manager = AuthManager::open(&dir.path().join("auth.sqlite")).unwrap();

        manager.request_code("test@example.com").unwrap();

        let result = manager.verify_code("test@example.com", "000000");
        assert!(matches!(result, Err(Error::InvalidCode)));
    }
}
