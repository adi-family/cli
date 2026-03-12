use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Migration {version} failed: {message}")]
    MigrationFailed { version: i64, message: String },

    #[error("Migration {version} not found")]
    MigrationNotFound { version: i64 },

    #[error("Database locked: {0}")]
    Locked(String),

    #[error("Invalid migration order: {0}")]
    InvalidOrder(String),
}

pub type Result<T> = std::result::Result<T, Error>;
