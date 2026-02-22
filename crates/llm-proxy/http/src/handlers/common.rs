//! Common utilities for proxy handlers.

use std::time::{Duration, Instant};

use llm_proxy_core::{
    db, providers, ApiError, ApiResult, KeyMode, ProviderConfig, ProxyRequest, ProxyToken,
    RequestStatus, UsageInfo,
};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::state::AppState;

/// Context for a proxy request.
pub struct ProxyContext {
    pub request_id: String,
    pub token: ProxyToken,
    pub provider_config: ProviderConfig,
    pub provider: Box<dyn providers::LlmProvider>,
    pub start_time: Instant,
    pub is_streaming: bool,
    pub endpoint: String,
    pub requested_model: Option<String>,
}

impl ProxyContext {
    /// Create a new proxy context.
    pub async fn new(
        state: &AppState,
        token: ProxyToken,
        endpoint: &str,
        is_streaming: bool,
        requested_model: Option<String>,
    ) -> ApiResult<Self> {
        let request_id = Uuid::new_v4().to_string();

        // Get the API key based on key mode
        let provider_config = match token.key_mode {
            KeyMode::Byok => {
                let upstream_key_id = token.upstream_key_id.ok_or_else(|| {
                    ApiError::Internal("BYOK token missing upstream_key_id".to_string())
                })?;

                let key =
                    db::keys::get_upstream_key_by_id(state.db.pool(), upstream_key_id).await?;

                if !key.is_active {
                    return Err(ApiError::Forbidden("Upstream key is inactive".to_string()));
                }

                let api_key = state.secrets.decrypt(&key.api_key_encrypted)?;

                ProviderConfig {
                    provider_type: key.provider_type,
                    api_key,
                    base_url: key.base_url,
                }
            }
            KeyMode::Platform => {
                let provider_type = token.platform_provider.ok_or_else(|| {
                    ApiError::Internal("Platform token missing platform_provider".to_string())
                })?;

                let platform_key =
                    db::platform_keys::get_platform_key(state.db.pool(), provider_type).await?;

                let api_key = state.secrets.decrypt(&platform_key.api_key_encrypted)?;

                ProviderConfig {
                    provider_type,
                    api_key,
                    base_url: platform_key.base_url,
                }
            }
        };

        let provider = providers::create_provider(
            provider_config.provider_type,
            provider_config.base_url.clone(),
        );

        Ok(Self {
            request_id,
            token,
            provider_config,
            provider,
            start_time: Instant::now(),
            is_streaming,
            endpoint: endpoint.to_string(),
            requested_model,
        })
    }

    /// Get the API key.
    pub fn api_key(&self) -> &str {
        &self.provider_config.api_key
    }

    /// Get elapsed time in milliseconds.
    pub fn elapsed_ms(&self) -> i32 {
        self.start_time.elapsed().as_millis() as i32
    }

    /// Log the request result.
    pub async fn log_result(
        &self,
        state: &AppState,
        status: RequestStatus,
        status_code: Option<i16>,
        usage: Option<UsageInfo>,
        cost: Option<Decimal>,
        upstream_request_id: Option<String>,
        actual_model: Option<String>,
        ttft_ms: Option<i32>,
        error_type: Option<&str>,
        error_message: Option<&str>,
        request_body: Option<&serde_json::Value>,
        response_body: Option<&serde_json::Value>,
    ) -> ApiResult<()> {
        // Determine what to log based on token settings
        let log_req = if self.token.log_requests {
            request_body
        } else {
            None
        };
        let log_resp = if self.token.log_responses {
            response_body
        } else {
            None
        };

        // Log to database
        db::usage::log_usage(
            state.db.pool(),
            self.token.id,
            self.token.user_id,
            &self.request_id,
            upstream_request_id.as_deref(),
            self.requested_model.as_deref(),
            actual_model.as_deref(),
            self.provider_config.provider_type,
            self.token.key_mode,
            usage.as_ref().and_then(|u| u.input_tokens),
            usage.as_ref().and_then(|u| u.output_tokens),
            usage.as_ref().and_then(|u| u.total_tokens),
            cost,
            &self.endpoint,
            self.is_streaming,
            Some(self.elapsed_ms()),
            ttft_ms,
            status,
            status_code,
            error_type,
            error_message,
            log_req,
            log_resp,
        )
        .await?;

        Ok(())
    }
}

/// Transform a request body using the token's request script.
pub fn transform_request(
    state: &AppState,
    token: &ProxyToken,
    method: &str,
    path: &str,
    body: serde_json::Value,
) -> ApiResult<serde_json::Value> {
    if let Some(script) = &token.request_script {
        let headers = http::HeaderMap::new();
        state
            .transform
            .transform_request_body(script, method, path, &headers, body)
    } else {
        Ok(body)
    }
}

/// Transform a response body using the token's response script.
pub fn transform_response(
    state: &AppState,
    token: &ProxyToken,
    status_code: u16,
    body: serde_json::Value,
    usage: &Option<UsageInfo>,
) -> ApiResult<serde_json::Value> {
    if let Some(script) = &token.response_script {
        let headers = http::HeaderMap::new();
        state.transform.transform_response_body(
            script,
            status_code,
            &headers,
            body,
            usage.as_ref().and_then(|u| u.input_tokens),
            usage.as_ref().and_then(|u| u.output_tokens),
        )
    } else {
        Ok(body)
    }
}
