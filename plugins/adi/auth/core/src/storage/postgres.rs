use crate::error::{Error, Result};
use crate::types::{User, UserId, VerificationCode};
use chrono::Utc;
use sqlx::{PgPool, Row};

use super::AuthStorage;

pub struct PostgresAuthStorage {
    pool: PgPool,
}

impl PostgresAuthStorage {
    /// Open database connection without running migrations.
    /// Use `adi-auth-migrate` CLI to run migrations separately.
    pub async fn open(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| Error::Storage(format!("Failed to connect to database: {}", e)))?;

        Ok(Self { pool })
    }

    /// Open database and run all migrations. Useful for tests and development.
    pub async fn open_and_migrate(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| Error::Storage(format!("Failed to connect to database: {}", e)))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| Error::Storage(format!("Migration failed: {}", e)))?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

const USER_COLUMNS: &str = "id, email, created_at, last_login_at, totp_secret, login, password_hash, is_anonymous";

fn row_to_user(row: sqlx::postgres::PgRow) -> User {
    User {
        id: UserId(row.get("id")),
        email: row.get("email"),
        created_at: row.get("created_at"),
        last_login_at: row.get("last_login_at"),
        totp_secret: row.get("totp_secret"),
        login: row.get("login"),
        password_hash: row.get("password_hash"),
        is_anonymous: row.get("is_anonymous"),
    }
}

#[async_trait::async_trait]
impl AuthStorage for PostgresAuthStorage {
    async fn create_user(&self, user: &User) -> Result<()> {
        sqlx::query(
            "INSERT INTO users (id, email, created_at, last_login_at, totp_secret, login, password_hash, is_anonymous)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(user.id.0)
        .bind(&user.email)
        .bind(user.created_at)
        .bind(user.last_login_at)
        .bind(&user.totp_secret)
        .bind(&user.login)
        .bind(&user.password_hash)
        .bind(user.is_anonymous)
        .execute(&self.pool)
        .await
        .map_err(Error::Sqlx)?;

        Ok(())
    }

    async fn get_user_by_id(&self, id: UserId) -> Result<User> {
        let query = format!("SELECT {} FROM users WHERE id = $1", USER_COLUMNS);
        let row = sqlx::query(&query)
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::Sqlx)?
            .ok_or_else(|| Error::UserNotFound(id.to_string()))?;

        Ok(row_to_user(row))
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let email_lower = email.to_lowercase();
        let query = format!("SELECT {} FROM users WHERE email = $1", USER_COLUMNS);

        let row = sqlx::query(&query)
            .bind(&email_lower)
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::Sqlx)?;

        Ok(row.map(row_to_user))
    }

    async fn update_last_login(&self, id: UserId) -> Result<()> {
        let now = Utc::now();

        let result = sqlx::query("UPDATE users SET last_login_at = $1 WHERE id = $2")
            .bind(now)
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(Error::Sqlx)?;

        if result.rows_affected() == 0 {
            return Err(Error::UserNotFound(id.to_string()));
        }

        Ok(())
    }

    async fn store_verification_code(&self, code: &VerificationCode) -> Result<()> {
        sqlx::query(
            "INSERT INTO verification_codes (email, code, created_at, expires_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&code.email)
        .bind(&code.code)
        .bind(code.created_at)
        .bind(code.expires_at)
        .execute(&self.pool)
        .await
        .map_err(Error::Sqlx)?;

        Ok(())
    }

    async fn get_verification_code(&self, email: &str) -> Result<Option<VerificationCode>> {
        let email_lower = email.to_lowercase();

        let row = sqlx::query(
            "SELECT email, code, created_at, expires_at
             FROM verification_codes
             WHERE email = $1
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(&email_lower)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Sqlx)?;

        Ok(row.map(|row| VerificationCode {
            email: row.get("email"),
            code: row.get("code"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
        }))
    }

    async fn delete_verification_codes(&self, email: &str) -> Result<()> {
        let email_lower = email.to_lowercase();

        sqlx::query("DELETE FROM verification_codes WHERE email = $1")
            .bind(&email_lower)
            .execute(&self.pool)
            .await
            .map_err(Error::Sqlx)?;

        Ok(())
    }

    async fn cleanup_expired_codes(&self) -> Result<u64> {
        let now = Utc::now();

        let result = sqlx::query("DELETE FROM verification_codes WHERE expires_at < $1")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(Error::Sqlx)?;

        Ok(result.rows_affected())
    }

    async fn set_totp_secret(&self, id: UserId, secret: Option<&str>) -> Result<()> {
        let result = sqlx::query("UPDATE users SET totp_secret = $1 WHERE id = $2")
            .bind(secret)
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(Error::Sqlx)?;

        if result.rows_affected() == 0 {
            return Err(Error::UserNotFound(id.to_string()));
        }

        Ok(())
    }

    async fn get_user_by_login(&self, login: &str) -> Result<Option<User>> {
        let query = format!("SELECT {} FROM users WHERE login = $1", USER_COLUMNS);

        let row = sqlx::query(&query)
            .bind(login)
            .fetch_optional(&self.pool)
            .await
            .map_err(Error::Sqlx)?;

        Ok(row.map(row_to_user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_storage() -> PostgresAuthStorage {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/adi_auth_test".to_string());
        PostgresAuthStorage::open_and_migrate(&database_url)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_user() {
        let storage = create_test_storage().await;

        let user = User::new("test@example.com");
        storage.create_user(&user).await.unwrap();

        let retrieved = storage.get_user_by_email("test@example.com").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().email, "test@example.com");
    }

    #[tokio::test]
    async fn test_verification_codes() {
        let storage = create_test_storage().await;

        let code = VerificationCode::new("test@example.com", 10);
        storage.store_verification_code(&code).await.unwrap();

        let retrieved = storage
            .get_verification_code("test@example.com")
            .await
            .unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.email, "test@example.com");
        assert!(!retrieved.is_expired());

        storage
            .delete_verification_codes("test@example.com")
            .await
            .unwrap();
        let deleted = storage
            .get_verification_code("test@example.com")
            .await
            .unwrap();
        assert!(deleted.is_none());
    }
}
