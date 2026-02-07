use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("Permission denied for tool: {tool}, pattern: {pattern}")]
    PermissionDenied { tool: String, pattern: String },

    #[error("User aborted the operation")]
    UserAborted,

    #[error("Max iterations reached: {0}")]
    MaxIterationsReached(usize),

    #[error("Token limit exceeded: {used} > {limit}")]
    TokenLimitExceeded { used: usize, limit: usize },

    #[error("Timeout after {0} ms")]
    Timeout(u64),

    #[error("Invalid tool arguments: {0}")]
    InvalidArguments(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Context management error: {0}")]
    ContextError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    // Quota errors
    #[error("Tool '{0}' is disabled")]
    ToolDisabled(String),

    #[error("Quota exceeded for tool '{tool}': {message}")]
    QuotaExceeded { tool: String, message: String },

    // Configuration errors
    #[error("Configuration validation failed for tool '{tool}': {errors:?}")]
    ConfigValidation { tool: String, errors: Vec<String> },

    #[error("TOML deserialization error: {0}")]
    TomlDeError(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerError(String),

    // Provider errors
    #[error("Anthropic API error: {0}")]
    AnthropicError(String),

    #[error("OpenAI API error: {0}")]
    OpenAiError(String),

    #[error("OpenRouter API error: {0}")]
    OpenRouterError(String),

    #[error("Ollama error: {0}")]
    OllamaError(String),

    #[error("Provider configuration error: {0}")]
    ProviderConfig(String),

    #[error("API key not found: {0}")]
    ApiKeyMissing(String),

    #[error("Rate limited, retry after {0}s")]
    RateLimited(u64),
}

pub type Result<T> = std::result::Result<T, AgentError>;
