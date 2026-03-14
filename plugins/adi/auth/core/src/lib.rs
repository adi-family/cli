pub mod email;
pub mod error;
pub mod migrations;
pub mod storage;
pub mod token;
pub mod types;

#[cfg(feature = "axum")]
pub mod middleware;

pub use email::{EmailSender, SmtpConfig};
pub use error::{Error, Result};
pub use storage::{AuthStorage, PostgresAuthStorage};
pub use token::TokenManager;
pub use types::{
    AnonymousCredentials, AuthToken, TokenClaims, TotpSetup, User, UserId, VerificationCode,
};

#[cfg(feature = "axum")]
pub use middleware::{AuthError, AuthUser, OptionalAuthUser};

use std::sync::Arc;
use totp_rs::{Algorithm, Secret, TOTP};

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

const CODE_EXPIRY_MINUTES: i64 = 10;
const TOTP_ISSUER: &str = "ADI";
const LOGIN_LENGTH: usize = 12;
const PASSWORD_LENGTH: usize = 24;

pub struct AuthManager {
    storage: Arc<dyn AuthStorage>,
    email: EmailSender,
    tokens: TokenManager,
}

impl AuthManager {
    /// Open without running migrations. Use `adi-auth-migrate` CLI first.
    pub async fn open(database_url: &str) -> Result<Self> {
        let storage = PostgresAuthStorage::open(database_url).await?;

        Ok(Self {
            storage: Arc::new(storage),
            email: EmailSender::from_env(),
            tokens: TokenManager::from_env(),
        })
    }

    /// Open and run all migrations. Useful for tests and development.
    pub async fn open_and_migrate(database_url: &str) -> Result<Self> {
        let storage = PostgresAuthStorage::open_and_migrate(database_url).await?;

        Ok(Self {
            storage: Arc::new(storage),
            email: EmailSender::from_env(),
            tokens: TokenManager::from_env(),
        })
    }

    pub async fn open_from_env() -> Result<Self> {
        let database_url = lib_env_parse::env_opt("DATABASE_URL")
            .ok_or_else(|| Error::Storage("DATABASE_URL not set".to_string()))?;
        Self::open(&database_url).await
    }

    pub fn with_email(mut self, email: EmailSender) -> Self {
        self.email = email;
        self
    }

    pub fn with_tokens(mut self, tokens: TokenManager) -> Self {
        self.tokens = tokens;
        self
    }

    pub async fn request_code(&self, email: &str) -> Result<()> {
        self.storage.cleanup_expired_codes().await?;
        self.storage.delete_verification_codes(email).await?;

        let code = VerificationCode::new(email, CODE_EXPIRY_MINUTES);
        self.storage.store_verification_code(&code).await?;
        self.email.send_verification_code(email, &code.code)?;

        tracing::info!("Verification code sent to {}", email);
        Ok(())
    }

    pub async fn verify_code(&self, email: &str, code: &str) -> Result<AuthToken> {
        let stored_code = self
            .storage
            .get_verification_code(email)
            .await?
            .ok_or(Error::InvalidCode)?;

        if stored_code.is_expired() {
            self.storage.delete_verification_codes(email).await?;
            return Err(Error::CodeExpired);
        }

        if !stored_code.matches(code) {
            return Err(Error::InvalidCode);
        }

        self.storage.delete_verification_codes(email).await?;

        let user = match self.storage.get_user_by_email(email).await? {
            Some(user) => {
                self.storage.update_last_login(user.id).await?;
                user
            }
            None => {
                let user = User::new(email);
                self.storage.create_user(&user).await?;
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

    /// Issue a short-lived subtoken from a valid parent token.
    pub fn generate_subtoken(&self, parent_token: &str, ttl_seconds: i64) -> Result<AuthToken> {
        let claims = self.tokens.verify_token(parent_token)?;
        self.tokens.generate_subtoken(&claims, ttl_seconds)
    }

    pub async fn get_user(&self, user_id: UserId) -> Result<User> {
        self.storage.get_user_by_id(user_id).await
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        self.storage.get_user_by_email(email).await
    }

    pub async fn setup_totp(&self, user_id: UserId) -> Result<TotpSetup> {
        let user = self.storage.get_user_by_id(user_id).await?;

        if user.has_totp() {
            return Err(Error::TotpAlreadyConfigured);
        }

        let secret = Secret::generate_secret();
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret
                .to_bytes()
                .map_err(|e| Error::TotpError(e.to_string()))?,
            Some(TOTP_ISSUER.to_string()),
            user.email.clone(),
        )
        .map_err(|e| Error::TotpError(e.to_string()))?;

        let otpauth_url = totp.get_url();
        let qr_code = totp
            .get_qr_base64()
            .map_err(|e| Error::TotpError(e.to_string()))?;

        Ok(TotpSetup {
            secret: secret.to_encoded().to_string(),
            otpauth_url,
            qr_code_base64: qr_code,
        })
    }

    pub async fn enable_totp(&self, user_id: UserId, secret: &str, code: &str) -> Result<()> {
        let user = self.storage.get_user_by_id(user_id).await?;

        if user.has_totp() {
            return Err(Error::TotpAlreadyConfigured);
        }

        let totp = self.create_totp_from_secret(secret)?;

        if !totp
            .check_current(code)
            .map_err(|e| Error::TotpError(e.to_string()))?
        {
            return Err(Error::InvalidTotp);
        }

        self.storage.set_totp_secret(user_id, Some(secret)).await?;
        tracing::info!("TOTP enabled for user: {}", user.email);

        Ok(())
    }

    pub async fn verify_totp(&self, email: &str, code: &str) -> Result<AuthToken> {
        let user = self
            .storage
            .get_user_by_email(email)
            .await?
            .ok_or(Error::UserNotFound(email.to_string()))?;

        let secret = user.totp_secret.as_ref().ok_or(Error::TotpNotConfigured)?;
        let totp = self.create_totp_from_secret(secret)?;

        if !totp
            .check_current(code)
            .map_err(|e| Error::TotpError(e.to_string()))?
        {
            return Err(Error::InvalidTotp);
        }

        self.storage.update_last_login(user.id).await?;
        let token = self.tokens.generate_token(&user)?;
        tracing::info!("User logged in via TOTP: {}", email);

        Ok(token)
    }

    pub async fn disable_totp(&self, user_id: UserId) -> Result<()> {
        let user = self.storage.get_user_by_id(user_id).await?;

        if !user.has_totp() {
            return Err(Error::TotpNotConfigured);
        }

        self.storage.set_totp_secret(user_id, None).await?;
        tracing::info!("TOTP disabled for user: {}", user.email);

        Ok(())
    }

    /// Create an anonymous account with auto-generated login and password.
    pub async fn create_anonymous(&self) -> Result<AnonymousCredentials> {
        let login = generate_login();
        let password = generate_password();
        let hash = hash_password(&password)?;

        let mut user = User::new(&format!("{}@anonymous", login));
        user.login = Some(login.clone());
        user.password_hash = Some(hash);
        user.is_anonymous = true;

        self.storage.create_user(&user).await?;
        let token = self.tokens.generate_token(&user)?;

        tracing::info!("Created anonymous user: {}", login);

        Ok(AnonymousCredentials {
            login,
            password,
            token,
        })
    }

    /// Authenticate with login and password credentials.
    pub async fn login_with_credentials(&self, login: &str, password: &str) -> Result<AuthToken> {
        let user = self
            .storage
            .get_user_by_login(login)
            .await?
            .ok_or(Error::InvalidCredentials)?;

        let stored_hash = user
            .password_hash
            .as_deref()
            .ok_or(Error::InvalidCredentials)?;

        verify_password(password, stored_hash)?;

        self.storage.update_last_login(user.id).await?;
        let token = self.tokens.generate_token(&user)?;

        tracing::info!("User logged in via credentials: {}", login);

        Ok(token)
    }

    fn create_totp_from_secret(&self, secret: &str) -> Result<TOTP> {
        let secret = Secret::Encoded(secret.to_string());
        let secret_bytes = secret
            .to_bytes()
            .map_err(|e| Error::TotpError(e.to_string()))?;

        TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            None,
            String::new(),
        )
        .map_err(|e| Error::TotpError(e.to_string()))
    }
}

fn generate_login() -> String {
    let charset = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..LOGIN_LENGTH)
        .map(|_| charset[fastrand::usize(..charset.len())] as char)
        .collect()
}

fn generate_password() -> String {
    let charset = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%&*";
    (0..PASSWORD_LENGTH)
        .map(|_| charset[fastrand::usize(..charset.len())] as char)
        .collect()
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| Error::Storage(format!("Password hashing failed: {}", e)))
}

fn verify_password(password: &str, hash: &str) -> Result<()> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| Error::Storage(format!("Invalid password hash: {}", e)))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| Error::InvalidCredentials)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_manager() -> AuthManager {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/adi_auth_test".to_string());
        AuthManager::open_and_migrate(&database_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_auth_flow() {
        let manager = create_test_manager().await;

        manager.request_code("test@example.com").await.unwrap();

        let code = manager
            .storage
            .get_verification_code("test@example.com")
            .await
            .unwrap()
            .unwrap();

        let token = manager
            .verify_code("test@example.com", &code.code)
            .await
            .unwrap();
        assert_eq!(token.token_type, "Bearer");

        let claims = manager.verify_token(&token.access_token).unwrap();
        assert_eq!(claims.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_invalid_code() {
        let manager = create_test_manager().await;

        manager.request_code("test@example.com").await.unwrap();

        let result = manager.verify_code("test@example.com", "000000").await;
        assert!(matches!(result, Err(Error::InvalidCode)));
    }
}
