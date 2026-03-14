mod postgres;

pub use postgres::PostgresAuthStorage;

use crate::error::Result;
use crate::types::{User, UserId, VerificationCode};

#[async_trait::async_trait]
pub trait AuthStorage: Send + Sync {
    async fn create_user(&self, user: &User) -> Result<()>;
    async fn get_user_by_id(&self, id: UserId) -> Result<User>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn update_last_login(&self, id: UserId) -> Result<()>;

    async fn store_verification_code(&self, code: &VerificationCode) -> Result<()>;
    async fn get_verification_code(&self, email: &str) -> Result<Option<VerificationCode>>;
    async fn delete_verification_codes(&self, email: &str) -> Result<()>;
    async fn cleanup_expired_codes(&self) -> Result<u64>;

    async fn set_totp_secret(&self, id: UserId, secret: Option<&str>) -> Result<()>;

    async fn get_user_by_login(&self, login: &str) -> Result<Option<User>>;
}
