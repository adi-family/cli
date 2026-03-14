use std::time::Instant;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use uuid::Uuid;

use crate::AppState;
use crate::auth::ProxyTokenAuth;
use embed_proxy_core::db;
use embed_proxy_core::db::tokens;
use embed_proxy_core::error::ApiError;
use embed_proxy_core::providers;
use embed_proxy_core::types::{KeyMode, RequestStatus};

pub async fn forward(
    State(state): State<AppState>,
    token_auth: ProxyTokenAuth,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, ApiError> {
    let start = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let endpoint = "/v1/embeddings".to_string();

    // Resolve proxy token
    let token_hash = tokens::hash_token(&token_auth.raw_token);
    let token = db::get_proxy_token_by_hash(state.db.pool(), &token_hash).await?;

    // Validate token
    if !token.is_active {
        return Err(ApiError::Unauthorized);
    }
    if let Some(expires) = token.expires_at {
        if expires < Utc::now() {
            return Err(ApiError::Unauthorized);
        }
    }

    // Check model allowlist/blocklist
    let requested_model = body.get("model").and_then(|m| m.as_str()).map(String::from);
    if let Some(ref model) = requested_model {
        if let Some(ref allowed) = token.allowed_models {
            if !allowed.iter().any(|m| m == model) {
                return Err(ApiError::Forbidden(format!("Model '{model}' not allowed")));
            }
        }
        if let Some(ref blocked) = token.blocked_models {
            if blocked.iter().any(|m| m == model) {
                return Err(ApiError::Forbidden(format!("Model '{model}' is blocked")));
            }
        }
    }

    // Resolve API key
    let (api_key, provider_type, base_url) = match token.key_mode {
        KeyMode::Byok => {
            let key_id = token
                .upstream_key_id
                .ok_or_else(|| ApiError::BadRequest("BYOK token missing upstream_key_id".into()))?;
            let key = db::get_upstream_key(state.db.pool(), key_id, token.user_id).await?;
            let decrypted = state
                .secrets
                .decrypt(&key.api_key_encrypted)
                .map_err(|e| ApiError::EncryptionError(e.to_string()))?;
            (decrypted, key.provider_type, key.base_url)
        }
        KeyMode::Platform => {
            let provider = token
                .platform_provider
                .ok_or_else(|| ApiError::BadRequest("Platform token missing provider".into()))?;
            let keys = db::list_platform_keys(state.db.pool()).await?;
            let key = keys
                .into_iter()
                .find(|k| k.provider_type == provider && k.is_active)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("No active platform key for {provider:?}"))
                })?;
            let decrypted = state
                .secrets
                .decrypt(&key.api_key_encrypted)
                .map_err(|e| ApiError::EncryptionError(e.to_string()))?;
            (decrypted, key.provider_type, key.base_url)
        }
    };

    let provider = providers::create_provider(provider_type, base_url);
    let timeout = state.config.upstream_timeout_secs;

    // Extract input count for logging
    let input_count = extract_input_count(&body);
    let dimensions = body.get("dimensions").and_then(|d| d.as_i64()).map(|d| d as i32);

    let result = provider.embed(&api_key, body.clone(), timeout).await;

    match result {
        Ok(response) => {
            let latency_ms = start.elapsed().as_millis() as i32;
            let usage = provider.extract_usage(&response);
            let cost = provider.extract_cost(&response);
            let upstream_req_id = provider.extract_request_id(&response);
            let actual_model = provider.extract_model(&response);

            let response_body = response.body.clone();

            // Log usage asynchronously
            let pool = state.db.pool().clone();
            let req_id = request_id.clone();
            let req_model = requested_model.clone();
            let actual = actual_model.clone();
            let upstream_id = upstream_req_id.clone();
            let log_req = if token.log_requests {
                Some(body.clone())
            } else {
                None
            };
            let log_resp = if token.log_responses {
                Some(response_body.clone())
            } else {
                None
            };
            tokio::spawn(async move {
                let _ = db::log_usage(
                    &pool,
                    token.id,
                    token.user_id,
                    &req_id,
                    upstream_id.as_deref(),
                    req_model.as_deref(),
                    actual.as_deref(),
                    provider_type,
                    token.key_mode,
                    usage.as_ref().and_then(|u| u.input_tokens),
                    usage.as_ref().and_then(|u| u.total_tokens),
                    dimensions,
                    input_count,
                    cost,
                    &endpoint,
                    Some(latency_ms),
                    RequestStatus::Success,
                    Some(response.status.as_u16() as i16),
                    None,
                    None,
                    log_req.as_ref(),
                    log_resp.as_ref(),
                )
                .await;
            });

            Ok((
                StatusCode::from_u16(response.status.as_u16()).unwrap_or(StatusCode::OK),
                [("x-request-id", request_id)],
                Json(response_body),
            )
                .into_response())
        }
        Err(e) => {
            log_error(
                &state,
                &token,
                &request_id,
                requested_model.as_deref(),
                &endpoint,
                &start,
                &e,
            )
            .await;
            Err(provider_error_to_api_error(e))
        }
    }
}

pub async fn list_models(
    State(state): State<AppState>,
    token_auth: ProxyTokenAuth,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token_hash = tokens::hash_token(&token_auth.raw_token);
    let token = db::get_proxy_token_by_hash(state.db.pool(), &token_hash).await?;

    if !token.is_active {
        return Err(ApiError::Unauthorized);
    }
    if let Some(expires) = token.expires_at {
        if expires < Utc::now() {
            return Err(ApiError::Unauthorized);
        }
    }

    let (api_key, provider_type, base_url) = match token.key_mode {
        KeyMode::Byok => {
            let key_id = token
                .upstream_key_id
                .ok_or_else(|| ApiError::BadRequest("BYOK token missing upstream_key_id".into()))?;
            let key = db::get_upstream_key(state.db.pool(), key_id, token.user_id).await?;
            let decrypted = state
                .secrets
                .decrypt(&key.api_key_encrypted)
                .map_err(|e| ApiError::EncryptionError(e.to_string()))?;
            (decrypted, key.provider_type, key.base_url)
        }
        KeyMode::Platform => {
            let provider = token
                .platform_provider
                .ok_or_else(|| ApiError::BadRequest("Platform token missing provider".into()))?;
            let keys = db::list_platform_keys(state.db.pool()).await?;
            let key = keys
                .into_iter()
                .find(|k| k.provider_type == provider && k.is_active)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("No active platform key for {provider:?}"))
                })?;
            let decrypted = state
                .secrets
                .decrypt(&key.api_key_encrypted)
                .map_err(|e| ApiError::EncryptionError(e.to_string()))?;
            (decrypted, key.provider_type, key.base_url)
        }
    };

    let provider = providers::create_provider(provider_type, base_url);
    let models = provider
        .list_models(&api_key)
        .await
        .map_err(provider_error_to_api_error)?;

    let filtered: Vec<_> = models
        .into_iter()
        .filter(|m| {
            if let Some(ref allowed) = token.allowed_models {
                if !allowed.iter().any(|a| a == &m.id) {
                    return false;
                }
            }
            if let Some(ref blocked) = token.blocked_models {
                if blocked.iter().any(|b| b == &m.id) {
                    return false;
                }
            }
            true
        })
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "object": "model",
                "owned_by": format!("{:?}", m.provider).to_lowercase(),
                "dimensions": m.dimensions,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "object": "list",
        "data": filtered,
    })))
}

fn extract_input_count(body: &serde_json::Value) -> Option<i32> {
    match body.get("input") {
        Some(serde_json::Value::Array(arr)) => Some(arr.len() as i32),
        Some(serde_json::Value::String(_)) => Some(1),
        _ => None,
    }
}

async fn log_error(
    state: &AppState,
    token: &embed_proxy_core::types::ProxyToken,
    request_id: &str,
    requested_model: Option<&str>,
    endpoint: &str,
    start: &Instant,
    error: &providers::ProviderError,
) {
    let latency_ms = start.elapsed().as_millis() as i32;
    let _ = db::log_usage(
        state.db.pool(),
        token.id,
        token.user_id,
        request_id,
        None,
        requested_model,
        None,
        token
            .platform_provider
            .unwrap_or(embed_proxy_core::types::ProviderType::Custom),
        token.key_mode,
        None,
        None,
        None,
        None,
        None,
        endpoint,
        Some(latency_ms),
        RequestStatus::Error,
        None,
        Some(error_type_str(error)),
        Some(&error.to_string()),
        None,
        None,
    )
    .await;
}

fn error_type_str(e: &providers::ProviderError) -> &'static str {
    match e {
        providers::ProviderError::RequestFailed(_) => "request_failed",
        providers::ProviderError::InvalidResponse(_) => "invalid_response",
        providers::ProviderError::AuthenticationFailed => "auth_failed",
        providers::ProviderError::RateLimited => "rate_limited",
        providers::ProviderError::ModelNotFound(_) => "model_not_found",
        providers::ProviderError::Timeout => "timeout",
        providers::ProviderError::Network(_) => "network",
        providers::ProviderError::Parse(_) => "parse",
    }
}

fn provider_error_to_api_error(e: providers::ProviderError) -> ApiError {
    match e {
        providers::ProviderError::AuthenticationFailed => ApiError::Unauthorized,
        providers::ProviderError::RateLimited => ApiError::RateLimited,
        providers::ProviderError::Timeout => ApiError::UpstreamError("Upstream timeout".into()),
        e => ApiError::UpstreamError(e.to_string()),
    }
}
