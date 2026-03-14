use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;
use crate::auth::AuthUser;
use embed_proxy_core::error::ApiResult;
use embed_proxy_core::types::{ProviderType, UpstreamApiKey};
use embed_proxy_core::{db, providers};

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: String,
    pub base_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateKeyRequest {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub is_active: Option<bool>,
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<UpstreamApiKey>>> {
    let keys = db::list_upstream_keys(state.db.pool(), user.id).await?;
    Ok(Json(keys))
}

pub async fn get_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<UpstreamApiKey>> {
    let key = db::get_upstream_key(state.db.pool(), id, user.id).await?;
    Ok(Json(key))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateKeyRequest>,
) -> ApiResult<Json<UpstreamApiKey>> {
    let encrypted = state
        .secrets
        .encrypt(&body.api_key)
        .map_err(|e| embed_proxy_core::ApiError::EncryptionError(e.to_string()))?;

    let key = db::create_upstream_key(
        state.db.pool(),
        user.id,
        &body.name,
        body.provider_type,
        &encrypted,
        body.base_url.as_deref(),
    )
    .await?;
    Ok(Json(key))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateKeyRequest>,
) -> ApiResult<Json<UpstreamApiKey>> {
    let encrypted = body
        .api_key
        .as_deref()
        .map(|k| state.secrets.encrypt(k))
        .transpose()
        .map_err(|e| embed_proxy_core::ApiError::EncryptionError(e.to_string()))?;

    let key = db::update_upstream_key(
        state.db.pool(),
        id,
        user.id,
        body.name.as_deref(),
        encrypted.as_deref(),
        Some(body.base_url.as_deref()),
        body.is_active,
    )
    .await?;
    Ok(Json(key))
}

pub async fn delete_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    db::delete_upstream_key(state.db.pool(), id, user.id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn verify(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let key = db::get_upstream_key(state.db.pool(), id, user.id).await?;
    let decrypted = state
        .secrets
        .decrypt(&key.api_key_encrypted)
        .map_err(|e| embed_proxy_core::ApiError::EncryptionError(e.to_string()))?;
    let provider = providers::create_provider(key.provider_type, key.base_url.clone());

    match provider.list_models(&decrypted).await {
        Ok(models) => Ok(Json(serde_json::json!({
            "valid": true,
            "models": models.into_iter().map(|m| m.id).collect::<Vec<_>>()
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "valid": false,
            "error": e.to_string()
        }))),
    }
}
