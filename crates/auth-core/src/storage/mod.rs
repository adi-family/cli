mod sqlite;

pub use sqlite::SqliteAuthStorage;

use crate::error::Result;
use crate::types::{User, UserId, VerificationCode};

pub trait AuthStorage: Send + Sync {
    fn create_user(&self, user: &User) -> Result<()>;
    fn get_user_by_id(&self, id: UserId) -> Result<User>;
    fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    fn update_last_login(&self, id: UserId) -> Result<()>;

    fn store_verification_code(&self, code: &VerificationCode) -> Result<()>;
    fn get_verification_code(&self, email: &str) -> Result<Option<VerificationCode>>;
    fn delete_verification_codes(&self, email: &str) -> Result<()>;
    fn cleanup_expired_codes(&self) -> Result<u64>;
}
