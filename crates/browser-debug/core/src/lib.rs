//! ADI Browser Debug Core Library
//!
//! Provides types and signaling client for browser debugging functionality.

pub mod client;
pub mod token;

// Re-export key types from signaling protocol
pub use lib_signaling_protocol::{
    BrowserDebugTab, ConsoleEntry, ConsoleFilters, ConsoleLevel, NetworkEventData,
    NetworkEventType, NetworkFilters, NetworkRequest, SignalingMessage,
};

/// Error type for browser debug operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid token: {0}")]
    InvalidToken(String),
}

pub type Result<T> = std::result::Result<T, Error>;
