//! Domain types for adi-llm-proxy.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported LLM provider types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    /// OpenAI API (or compatible)
    OpenAI,
    /// Anthropic Messages API
    Anthropic,
    /// OpenRouter (OpenAI-compatible with extra features)
    OpenRouter,
    /// Custom user-defined endpoint
    Custom,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::OpenRouter => write!(f, "openrouter"),
            ProviderType::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ProviderType::OpenAI),
            "anthropic" => Ok(ProviderType::Anthropic),
            "openrouter" => Ok(ProviderType::OpenRouter),
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
    /// User provides their own API key (BYOK)
    Byok,
    /// Use platform-managed API keys
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
    /// Request completed successfully
    Success,
    /// Error in proxy (auth, transform, etc.)
    Error,
    /// Error from upstream provider
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
    pub request_script: Option<String>,
    pub response_script: Option<String>,
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

/// Token usage information extracted from LLM response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageInfo {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

/// Usage log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyUsageLog {
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
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub reported_cost_usd: Option<Decimal>,
    pub endpoint: String,
    pub is_streaming: bool,
    pub latency_ms: Option<i32>,
    pub ttft_ms: Option<i32>,
    pub status: RequestStatus,
    pub status_code: Option<i16>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub request_body: Option<serde_json::Value>,
    pub response_body: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Model information returned by providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub context_length: Option<i32>,
    pub provider: ProviderType,
}

/// Proxy request wrapper.
#[derive(Debug, Clone)]
pub struct ProxyRequest {
    pub method: http::Method,
    pub path: String,
    pub headers: http::HeaderMap,
    pub body: serde_json::Value,
}

/// Proxy response wrapper.
#[derive(Debug, Clone)]
pub struct ProxyResponse {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub body: serde_json::Value,
}

/// Provider-specific configuration.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: String,
    pub base_url: Option<String>,
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

/// Wire-format usage log entry with `reported_cost_usd` as string (matching api.tsp).
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
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub reported_cost_usd: Option<String>,
    pub endpoint: String,
    pub is_streaming: bool,
    pub latency_ms: Option<i32>,
    pub ttft_ms: Option<i32>,
    pub status: RequestStatus,
    pub status_code: Option<i16>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_display_roundtrip() {
        for pt in [ProviderType::OpenAI, ProviderType::Anthropic, ProviderType::OpenRouter, ProviderType::Custom] {
            let s = pt.to_string();
            let parsed: ProviderType = s.parse().unwrap();
            assert_eq!(parsed, pt);
        }
    }

    #[test]
    fn test_provider_type_case_insensitive() {
        assert_eq!("OPENAI".parse::<ProviderType>().unwrap(), ProviderType::OpenAI);
        assert_eq!("Anthropic".parse::<ProviderType>().unwrap(), ProviderType::Anthropic);
    }

    #[test]
    fn test_provider_type_unknown() {
        assert!("unknown".parse::<ProviderType>().is_err());
    }

    #[test]
    fn test_key_mode_display() {
        assert_eq!(KeyMode::Byok.to_string(), "byok");
        assert_eq!(KeyMode::Platform.to_string(), "platform");
    }

    #[test]
    fn test_request_status_display() {
        assert_eq!(RequestStatus::Success.to_string(), "success");
        assert_eq!(RequestStatus::Error.to_string(), "error");
        assert_eq!(RequestStatus::UpstreamError.to_string(), "upstream_error");
    }

    #[test]
    fn test_upstream_key_to_summary() {
        let key = UpstreamApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "test-key".to_string(),
            provider_type: ProviderType::OpenAI,
            api_key_encrypted: "ENC:secret".to_string(),
            base_url: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let summary: UpstreamApiKeySummary = key.clone().into();
        assert_eq!(summary.id, key.id);
        assert_eq!(summary.name, "test-key");
        assert_eq!(summary.provider_type, ProviderType::OpenAI);
        assert!(summary.is_active);
    }

    #[test]
    fn test_proxy_token_to_summary() {
        let token = ProxyToken {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "my-token".to_string(),
            token_hash: "abc".to_string(),
            token_prefix: "adi_pk_12...".to_string(),
            key_mode: KeyMode::Byok,
            upstream_key_id: Some(Uuid::new_v4()),
            platform_provider: None,
            request_script: None,
            response_script: None,
            allowed_models: Some(vec!["gpt-4".to_string()]),
            blocked_models: None,
            log_requests: true,
            log_responses: false,
            is_active: true,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let summary: ProxyTokenSummary = token.clone().into();
        assert_eq!(summary.id, token.id);
        assert_eq!(summary.name, "my-token");
        assert_eq!(summary.key_mode, KeyMode::Byok);
        assert_eq!(summary.allowed_models, Some(vec!["gpt-4".to_string()]));
        assert!(summary.log_requests);
        assert!(!summary.log_responses);
    }

    #[test]
    fn test_provider_type_serde_roundtrip() {
        let pt = ProviderType::OpenRouter;
        let json = serde_json::to_string(&pt).unwrap();
        assert_eq!(json, "\"openrouter\"");
        let back: ProviderType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, pt);
    }

    #[test]
    fn test_key_mode_serde_roundtrip() {
        let km = KeyMode::Byok;
        let json = serde_json::to_string(&km).unwrap();
        assert_eq!(json, "\"byok\"");
        let back: KeyMode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, km);
    }

    #[test]
    fn test_usage_info_default() {
        let usage = UsageInfo::default();
        assert_eq!(usage.input_tokens, None);
        assert_eq!(usage.output_tokens, None);
        assert_eq!(usage.total_tokens, None);
    }

    #[test]
    fn test_deleted_response_serde() {
        let resp = DeletedResponse { deleted: true };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["deleted"], true);
    }

    #[test]
    fn test_verify_key_response_serde() {
        let resp = VerifyKeyResponse {
            valid: true,
            models: Some(vec!["gpt-4".to_string()]),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["valid"], true);
        assert_eq!(json["models"][0], "gpt-4");
    }
}
