use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Migration {version} failed: {message}")]
    MigrationFailed { version: u64, message: String },

    #[error("Migration {0} does not support rollback")]
    RollbackNotSupported(u64),

    #[error("Invalid migration order: {0}")]
    InvalidOrder(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Migration {0} not found")]
    NotFound(u64),

    #[error("Migration {0} already applied")]
    AlreadyApplied(u64),
}

impl Error {
    pub fn store(msg: impl Into<String>) -> Self {
        Self::Store(msg.into())
    }

    pub fn failed(version: u64, msg: impl Into<String>) -> Self {
        Self::MigrationFailed {
            version,
            message: msg.into(),
        }
    }
}
