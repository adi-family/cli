//! Error types for MCP operations.

use thiserror::Error;

/// MCP error type.
#[derive(Error, Debug)]
pub enum Error {
    /// JSON-RPC error response from server.
    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc {
        code: i32,
        message: String,
        data: Option<serde_json::Value>,
    },

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error during transport operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Transport-level error.
    #[error("Transport error: {0}")]
    Transport(String),

    /// Connection closed unexpectedly.
    #[error("Connection closed")]
    ConnectionClosed,

    /// Request timeout.
    #[error("Request timeout")]
    Timeout,

    /// Invalid message format.
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    /// Method not found.
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Invalid params.
    #[error("Invalid params: {0}")]
    InvalidParams(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Tool not found.
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Resource not found.
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// Prompt not found.
    #[error("Prompt not found: {0}")]
    PromptNotFound(String),

    /// HTTP error (for SSE transport).
    #[cfg(any(feature = "sse-client", feature = "sse-server"))]
    #[error("HTTP error: {0}")]
    Http(String),

    /// Channel send error.
    #[error("Channel send error")]
    ChannelSend,

    /// Channel receive error.
    #[error("Channel receive error")]
    ChannelRecv,
}

impl Error {
    /// Create a JSON-RPC error from error code constants.
    pub fn json_rpc(code: i32, message: impl Into<String>) -> Self {
        Self::JsonRpc {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a JSON-RPC error with additional data.
    pub fn json_rpc_with_data(
        code: i32,
        message: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self::JsonRpc {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Convert to JSON-RPC error code.
    pub fn to_json_rpc_code(&self) -> i32 {
        match self {
            Self::JsonRpc { code, .. } => *code,
            Self::Json(_) => JSON_RPC_PARSE_ERROR,
            Self::InvalidMessage(_) | Self::InvalidParams(_) => JSON_RPC_INVALID_PARAMS,
            Self::MethodNotFound(_) => JSON_RPC_METHOD_NOT_FOUND,
            Self::ToolNotFound(_) | Self::ResourceNotFound(_) | Self::PromptNotFound(_) => {
                JSON_RPC_INVALID_PARAMS
            }
            _ => JSON_RPC_INTERNAL_ERROR,
        }
    }
}

/// Result type alias using MCP Error.
pub type Result<T> = std::result::Result<T, Error>;

// JSON-RPC 2.0 standard error codes
pub const JSON_RPC_PARSE_ERROR: i32 = -32700;
pub const JSON_RPC_INVALID_REQUEST: i32 = -32600;
pub const JSON_RPC_METHOD_NOT_FOUND: i32 = -32601;
pub const JSON_RPC_INVALID_PARAMS: i32 = -32602;
pub const JSON_RPC_INTERNAL_ERROR: i32 = -32603;

// MCP-specific error codes (reserved range: -32000 to -32099)
pub const MCP_TOOL_EXECUTION_ERROR: i32 = -32001;
pub const MCP_RESOURCE_READ_ERROR: i32 = -32002;
pub const MCP_PROMPT_ERROR: i32 = -32003;
