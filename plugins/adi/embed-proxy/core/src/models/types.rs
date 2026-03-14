use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported embedding provider types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    OpenAI,
    Cohere,
    Google,
    Custom,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Cohere => write!(f, "cohere"),
            ProviderType::Google => write!(f, "google"),
            ProviderType::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ProviderType::OpenAI),
            "cohere" => Ok(ProviderType::Cohere),
            "google" => Ok(ProviderType::Google),
            "custom" => Ok(ProviderType::Custom),
            _ => Err(format!("Unknown provider type: {}", s)),
        }
    }
}

/// Key mode for proxy tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum KeyMode {
    Byok,
    Platform,
}

impl std::fmt::Display for KeyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyMode::Byok => write!(f, "byok"),
            KeyMode::Platform => write!(f, "platform"),
        }
    }
}

/// Request status for usage logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RequestStatus {
    Success,
    Error,
    UpstreamError,
}

impl std::fmt::Display for RequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestStatus::Success => write!(f, "success"),
            RequestStatus::Error => write!(f, "error"),
            RequestStatus::UpstreamError => write!(f, "upstream_error"),
        }
    }
}

/// Platform-managed provider API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformProviderKey {
    pub id: Uuid,
    pub provider_type: ProviderType,
    pub api_key_encrypted: String,
    pub base_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Model allowed in platform mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAllowedModel {
    pub id: Uuid,
    pub provider_type: ProviderType,
    pub model_id: String,
    pub display_name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// User's upstream API key (BYOK).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key_encrypted: String,
    pub base_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Summary of upstream API key (without encrypted key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamApiKeySummary {
    pub id: Uuid,
    pub name: String,
    pub provider_type: ProviderType,
    pub base_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UpstreamApiKey> for UpstreamApiKeySummary {
    fn from(key: UpstreamApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            provider_type: key.provider_type,
            base_url: key.base_url,
            is_active: key.is_active,
            created_at: key.created_at,
            updated_at: key.updated_at,
        }
    }
}

/// Proxy token for API access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub token_hash: String,
    pub token_prefix: String,
    pub key_mode: KeyMode,
    pub upstream_key_id: Option<Uuid>,
    pub platform_provider: Option<ProviderType>,
    pub allowed_models: Option<Vec<String>>,
    pub blocked_models: Option<Vec<String>>,
    pub log_requests: bool,
    pub log_responses: bool,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Summary of proxy token (for listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyTokenSummary {
    pub id: Uuid,
    pub name: String,
    pub token_prefix: String,
    pub key_mode: KeyMode,
    pub upstream_key_id: Option<Uuid>,
    pub platform_provider: Option<ProviderType>,
    pub allowed_models: Option<Vec<String>>,
    pub blocked_models: Option<Vec<String>>,
    pub log_requests: bool,
    pub log_responses: bool,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ProxyToken> for ProxyTokenSummary {
    fn from(token: ProxyToken) -> Self {
        Self {
            id: token.id,
            name: token.name,
            token_prefix: token.token_prefix,
            key_mode: token.key_mode,
            upstream_key_id: token.upstream_key_id,
            platform_provider: token.platform_provider,
            allowed_models: token.allowed_models,
            blocked_models: token.blocked_models,
            log_requests: token.log_requests,
            log_responses: token.log_responses,
            is_active: token.is_active,
            expires_at: token.expires_at,
            created_at: token.created_at,
        }
    }
}

/// Embedding usage information extracted from response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbedUsageInfo {
    pub input_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

/// Usage log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedUsageLog {
    pub id: Uuid,
    pub proxy_token_id: Uuid,
    pub user_id: Uuid,
    pub request_id: String,
    pub upstream_request_id: Option<String>,
    pub requested_model: Option<String>,
    pub actual_model: Option<String>,
    pub provider_type: ProviderType,
    pub key_mode: KeyMode,
    pub input_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub dimensions: Option<i32>,
    pub input_count: Option<i32>,
    pub reported_cost_usd: Option<Decimal>,
    pub endpoint: String,
    pub latency_ms: Option<i32>,
    pub status: RequestStatus,
    pub status_code: Option<i16>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub request_body: Option<serde_json::Value>,
    pub response_body: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Embedding model information returned by providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedModelInfo {
    pub id: String,
    pub name: Option<String>,
    pub dimensions: Option<i32>,
    pub provider: ProviderType,
}

/// Embedding request to upstream provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub model: String,
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
}

/// Embedding response from upstream provider.
#[derive(Debug, Clone)]
pub struct EmbedResponse {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub body: serde_json::Value,
}

// -- Types required by generated AdiService handler (from api.tsp) --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedResponse {
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyKeyResponse {
    pub valid: bool,
    pub models: Option<Vec<String>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenResponse {
    pub token: ProxyTokenSummary,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotateTokenResponse {
    pub token: ProxyTokenSummary,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformKeySummary {
    pub id: Uuid,
    pub provider_type: ProviderType,
    pub base_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedModelInfo {
    pub model_id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub provider_type: ProviderType,
    pub is_available: bool,
    pub allowed_models: Vec<AllowedModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResponse {
    pub logs: Vec<UsageLogEntry>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLogEntry {
    pub id: Uuid,
    pub proxy_token_id: Uuid,
    pub user_id: Uuid,
    pub request_id: String,
    pub upstream_request_id: Option<String>,
    pub requested_model: Option<String>,
    pub actual_model: Option<String>,
    pub provider_type: ProviderType,
    pub key_mode: KeyMode,
    pub input_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub dimensions: Option<i32>,
    pub input_count: Option<i32>,
    pub reported_cost_usd: Option<String>,
    pub endpoint: String,
    pub latency_ms: Option<i32>,
    pub status: RequestStatus,
    pub status_code: Option<i16>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}
