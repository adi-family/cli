use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Invalid verification code")]
    InvalidCode,

    #[error("Verification code expired")]
    CodeExpired,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Email sending failed: {0}")]
    EmailError(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
