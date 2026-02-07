use crate::error::{Error, Result};
use crate::migrations::migrations;
use crate::types::{User, UserId, VerificationCode};
use chrono::{DateTime, Utc};
use lib_migrations::{MigrationRunner, SqliteMigrationBackend};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

use super::AuthStorage;

pub struct SqliteAuthStorage {
    conn: Mutex<Connection>,
}

impl SqliteAuthStorage {
    pub fn open(path: &Path) -> Result<Self> {
        let backend = SqliteMigrationBackend::open(path)
            .map_err(|e| Error::Storage(format!("Failed to open db: {}", e)))?;

        let runner = MigrationRunner::new(backend).add_migrations(migrations());

        runner
            .init()
            .map_err(|e| Error::Storage(format!("Migration init failed: {}", e)))?;

        let applied = runner
            .migrate()
            .map_err(|e| Error::Storage(format!("Migration failed: {}", e)))?;

        if applied > 0 {
            tracing::info!("Applied {} migration(s)", applied);
        }

        let conn = runner.into_backend().into_connection();
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl AuthStorage for SqliteAuthStorage {
    fn create_user(&self, user: &User) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO users (id, email, created_at, last_login_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                user.id.0.to_string(),
                user.email,
                user.created_at.to_rfc3339(),
                user.last_login_at.map(|t| t.to_rfc3339()),
            ],
        )?;

        Ok(())
    }

    fn get_user_by_id(&self, id: UserId) -> Result<User> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT id, email, created_at, last_login_at FROM users WHERE id = ?1",
            params![id.0.to_string()],
            |row| {
                let id_str: String = row.get(0)?;
                let created_at_str: String = row.get(2)?;
                let last_login_str: Option<String> = row.get(3)?;

                Ok(User {
                    id: UserId(Uuid::parse_str(&id_str).unwrap()),
                    email: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    last_login_at: last_login_str.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::UserNotFound(id.to_string()),
            _ => Error::Sqlite(e),
        })
    }

    fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let email_lower = email.to_lowercase();

        let result = conn.query_row(
            "SELECT id, email, created_at, last_login_at FROM users WHERE email = ?1",
            params![email_lower],
            |row| {
                let id_str: String = row.get(0)?;
                let created_at_str: String = row.get(2)?;
                let last_login_str: Option<String> = row.get(3)?;

                Ok(User {
                    id: UserId(Uuid::parse_str(&id_str).unwrap()),
                    email: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    last_login_at: last_login_str.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                })
            },
        );

        match result {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Sqlite(e)),
        }
    }

    fn update_last_login(&self, id: UserId) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let rows = conn.execute(
            "UPDATE users SET last_login_at = ?1 WHERE id = ?2",
            params![now, id.0.to_string()],
        )?;

        if rows == 0 {
            return Err(Error::UserNotFound(id.to_string()));
        }

        Ok(())
    }

    fn store_verification_code(&self, code: &VerificationCode) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO verification_codes (email, code, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                code.email,
                code.code,
                code.created_at.to_rfc3339(),
                code.expires_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    fn get_verification_code(&self, email: &str) -> Result<Option<VerificationCode>> {
        let conn = self.conn.lock().unwrap();
        let email_lower = email.to_lowercase();

        let result = conn.query_row(
            "SELECT email, code, created_at, expires_at FROM verification_codes
             WHERE email = ?1 ORDER BY created_at DESC LIMIT 1",
            params![email_lower],
            |row| {
                let created_at_str: String = row.get(2)?;
                let expires_at_str: String = row.get(3)?;

                Ok(VerificationCode {
                    email: row.get(0)?,
                    code: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    expires_at: DateTime::parse_from_rfc3339(&expires_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            },
        );

        match result {
            Ok(code) => Ok(Some(code)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Sqlite(e)),
        }
    }

    fn delete_verification_codes(&self, email: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let email_lower = email.to_lowercase();

        conn.execute(
            "DELETE FROM verification_codes WHERE email = ?1",
            params![email_lower],
        )?;

        Ok(())
    }

    fn cleanup_expired_codes(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let rows = conn.execute(
            "DELETE FROM verification_codes WHERE expires_at < ?1",
            params![now],
        )?;

        Ok(rows as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_storage() -> (SqliteAuthStorage, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("auth.sqlite");
        let storage = SqliteAuthStorage::open(&db_path).unwrap();
        (storage, dir)
    }

    #[test]
    fn test_create_and_get_user() {
        let (storage, _dir) = create_test_storage();

        let user = User::new("test@example.com");
        storage.create_user(&user).unwrap();

        let retrieved = storage.get_user_by_email("test@example.com").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().email, "test@example.com");
    }

    #[test]
    fn test_verification_codes() {
        let (storage, _dir) = create_test_storage();

        let code = VerificationCode::new("test@example.com", 10);
        storage.store_verification_code(&code).unwrap();

        let retrieved = storage.get_verification_code("test@example.com").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.email, "test@example.com");
        assert!(!retrieved.is_expired());

        storage
            .delete_verification_codes("test@example.com")
            .unwrap();
        let deleted = storage.get_verification_code("test@example.com").unwrap();
        assert!(deleted.is_none());
    }
}
