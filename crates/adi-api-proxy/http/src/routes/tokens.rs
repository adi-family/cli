//! Proxy token management routes.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{middleware::AuthUser, state::AppState};
use adi_api_proxy_core::{db, ApiError, ApiResult, KeyMode, ProviderType, ProxyTokenSummary};

/// Request to create a proxy token.
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub key_mode: KeyMode,
    /// Required if key_mode is 'byok'
    pub upstream_key_id: Option<Uuid>,
    /// Required if key_mode is 'platform'
    pub platform_provider: Option<ProviderType>,
    pub request_script: Option<String>,
    pub response_script: Option<String>,
    pub allowed_models: Option<Vec<String>>,
    pub blocked_models: Option<Vec<String>>,
    #[serde(default)]
    pub log_requests: bool,
    #[serde(default)]
    pub log_responses: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Response with created token (includes secret, shown only once).
#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    pub token: ProxyTokenSummary,
    /// The raw token secret - only shown once!
    pub secret: String,
}

/// Create a new proxy token.
pub async fn create_token(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateTokenRequest>,
) -> ApiResult<Json<CreateTokenResponse>> {
    // Validate key mode configuration
    match req.key_mode {
        KeyMode::Byok => {
            if req.upstream_key_id.is_none() {
                return Err(ApiError::BadRequest(
                    "upstream_key_id is required for BYOK mode".to_string(),
                ));
            }
            if req.platform_provider.is_some() {
                return Err(ApiError::BadRequest(
                    "platform_provider should not be set for BYOK mode".to_string(),
                ));
            }
            // Verify the upstream key exists and belongs to user
            db::keys::get_upstream_key(state.db.pool(), req.upstream_key_id.unwrap(), user.id)
                .await?;
        }
        KeyMode::Platform => {
            if req.platform_provider.is_none() {
                return Err(ApiError::BadRequest(
                    "platform_provider is required for Platform mode".to_string(),
                ));
            }
            if req.upstream_key_id.is_some() {
                return Err(ApiError::BadRequest(
                    "upstream_key_id should not be set for Platform mode".to_string(),
                ));
            }
            // Verify the platform provider is configured
            db::platform_keys::get_platform_key(state.db.pool(), req.platform_provider.unwrap())
                .await?;
        }
    }

    // Validate Rhai scripts if provided
    if let Some(script) = &req.request_script {
        state.transform.compile(script)?;
    }
    if let Some(script) = &req.response_script {
        state.transform.compile(script)?;
    }

    // Generate token
    let (raw_token, prefix, hash) = db::tokens::generate_token();

    // Create the token
    let token = db::tokens::create_proxy_token(
        state.db.pool(),
        user.id,
        &req.name,
        &hash,
        &prefix,
        req.key_mode,
        req.upstream_key_id,
        req.platform_provider,
        req.request_script.as_deref(),
        req.response_script.as_deref(),
        req.allowed_models.as_deref(),
        req.blocked_models.as_deref(),
        req.log_requests,
        req.log_responses,
        req.expires_at,
    )
    .await?;

    Ok(Json(CreateTokenResponse {
        token: token.into(),
        secret: raw_token,
    }))
}

/// List all proxy tokens for the user.
pub async fn list_tokens(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<ProxyTokenSummary>>> {
    let tokens = db::tokens::list_proxy_tokens(state.db.pool(), user.id).await?;
    Ok(Json(tokens.into_iter().map(|t| t.into()).collect()))
}

/// Get a specific proxy token.
pub async fn get_token(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ProxyTokenSummary>> {
    let token = db::tokens::get_proxy_token(state.db.pool(), id, user.id).await?;
    Ok(Json(token.into()))
}

/// Request to update a proxy token.
#[derive(Debug, Deserialize)]
pub struct UpdateTokenRequest {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_option")]
    pub request_script: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_option_option")]
    pub response_script: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_option_option")]
    pub allowed_models: Option<Option<Vec<String>>>,
    #[serde(default, deserialize_with = "deserialize_option_option")]
    pub blocked_models: Option<Option<Vec<String>>>,
    pub log_requests: Option<bool>,
    pub log_responses: Option<bool>,
    pub is_active: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_option_option")]
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

fn deserialize_option_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

/// Update a proxy token.
pub async fn update_token(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTokenRequest>,
) -> ApiResult<Json<ProxyTokenSummary>> {
    // Validate Rhai scripts if provided
    if let Some(Some(script)) = &req.request_script {
        state.transform.compile(script)?;
    }
    if let Some(Some(script)) = &req.response_script {
        state.transform.compile(script)?;
    }

    let token = db::tokens::update_proxy_token(
        state.db.pool(),
        id,
        user.id,
        req.name.as_deref(),
        req.request_script.as_ref().map(|o| o.as_deref()),
        req.response_script.as_ref().map(|o| o.as_deref()),
        req.allowed_models.as_ref().map(|o| o.as_deref()),
        req.blocked_models.as_ref().map(|o| o.as_deref()),
        req.log_requests,
        req.log_responses,
        req.is_active,
        req.expires_at,
    )
    .await?;

    Ok(Json(token.into()))
}

/// Delete a proxy token.
pub async fn delete_token(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    db::tokens::delete_proxy_token(state.db.pool(), id, user.id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Response for token rotation.
#[derive(Debug, Serialize)]
pub struct RotateTokenResponse {
    pub token: ProxyTokenSummary,
    /// The new raw token secret - only shown once!
    pub secret: String,
}

/// Rotate a proxy token (generate new secret).
pub async fn rotate_token(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RotateTokenResponse>> {
    let (token, secret) = db::tokens::rotate_proxy_token(state.db.pool(), id, user.id).await?;

    Ok(Json(RotateTokenResponse {
        token: token.into(),
        secret,
    }))
}
