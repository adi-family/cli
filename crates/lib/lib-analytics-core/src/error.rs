use thiserror::Error;

/// Analytics error types
#[derive(Error, Debug)]
pub enum AnalyticsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Event channel closed")]
    ChannelClosed,

    #[error("Worker not running")]
    WorkerNotRunning,
}

pub type Result<T> = std::result::Result<T, AnalyticsError>;
