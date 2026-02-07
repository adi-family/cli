//! Upstream API key management routes.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{middleware::AuthUser, state::AppState};
use llm_proxy_core::{db, ApiError, ApiResult, ProviderType, UpstreamApiKeySummary};

/// Request to create an upstream API key.
#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: String,
    pub base_url: Option<String>,
}

/// Response with created key info.
#[derive(Debug, Serialize)]
pub struct CreateKeyResponse {
    pub key: UpstreamApiKeySummary,
}

/// Create a new upstream API key.
pub async fn create_key(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateKeyRequest>,
) -> ApiResult<Json<CreateKeyResponse>> {
    // Encrypt the API key
    let encrypted = state.secrets.encrypt(&req.api_key)?;

    // Create the key
    let key = db::keys::create_upstream_key(
        state.db.pool(),
        user.id,
        &req.name,
        req.provider_type,
        &encrypted,
        req.base_url.as_deref(),
    )
    .await?;

    Ok(Json(CreateKeyResponse { key: key.into() }))
}

/// List all upstream API keys for the user.
pub async fn list_keys(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<UpstreamApiKeySummary>>> {
    let keys = db::keys::list_upstream_keys(state.db.pool(), user.id).await?;
    Ok(Json(keys.into_iter().map(|k| k.into()).collect()))
}

/// Get a specific upstream API key.
pub async fn get_key(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<UpstreamApiKeySummary>> {
    let key = db::keys::get_upstream_key(state.db.pool(), id, user.id).await?;
    Ok(Json(key.into()))
}

/// Request to update an upstream API key.
#[derive(Debug, Deserialize)]
pub struct UpdateKeyRequest {
    pub name: Option<String>,
    pub api_key: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_option")]
    pub base_url: Option<Option<String>>,
    pub is_active: Option<bool>,
}

// Helper to deserialize Option<Option<T>> for nullable updates
fn deserialize_option_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

/// Update an upstream API key.
pub async fn update_key(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateKeyRequest>,
) -> ApiResult<Json<UpstreamApiKeySummary>> {
    // Encrypt the new API key if provided
    let encrypted = req
        .api_key
        .as_ref()
        .map(|k| state.secrets.encrypt(k))
        .transpose()?;

    let key = db::keys::update_upstream_key(
        state.db.pool(),
        id,
        user.id,
        req.name.as_deref(),
        encrypted.as_deref(),
        req.base_url.as_ref().map(|o| o.as_deref()),
        req.is_active,
    )
    .await?;

    Ok(Json(key.into()))
}

/// Delete an upstream API key.
pub async fn delete_key(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    db::keys::delete_upstream_key(state.db.pool(), id, user.id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Response for key verification.
#[derive(Debug, Serialize)]
pub struct VerifyKeyResponse {
    pub valid: bool,
    pub models: Option<Vec<String>>,
    pub error: Option<String>,
}

/// Verify an upstream API key by testing connectivity.
pub async fn verify_key(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<VerifyKeyResponse>> {
    let key = db::keys::get_upstream_key(state.db.pool(), id, user.id).await?;

    // Decrypt the API key
    let api_key = state.secrets.decrypt(&key.api_key_encrypted)?;

    // Create provider and test
    let provider =
        llm_proxy_core::providers::create_provider(key.provider_type, key.base_url.clone());

    match provider.list_models(&api_key).await {
        Ok(models) => Ok(Json(VerifyKeyResponse {
            valid: true,
            models: Some(models.into_iter().take(10).map(|m| m.id).collect()),
            error: None,
        })),
        Err(e) => Ok(Json(VerifyKeyResponse {
            valid: false,
            models: None,
            error: Some(e.to_string()),
        })),
    }
}
