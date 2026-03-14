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

    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid TOTP code")]
    InvalidTotp,

    #[error("TOTP not configured")]
    TotpNotConfigured,

    #[error("TOTP already configured")]
    TotpAlreadyConfigured,

    #[error("TOTP error: {0}")]
    TotpError(String),

    #[error("Invalid credentials")]
    InvalidCredentials,
}
