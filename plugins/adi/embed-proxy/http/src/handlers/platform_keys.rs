use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;
use crate::auth::AuthUser;
use embed_proxy_core::error::ApiResult;
use embed_proxy_core::types::{PlatformProviderKey, ProviderType};
use embed_proxy_core::db;

#[derive(Deserialize)]
pub struct UpsertPlatformKeyRequest {
    pub provider_type: ProviderType,
    pub api_key: String,
    pub base_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdatePlatformKeyRequest {
    pub is_active: Option<bool>,
}

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
) -> ApiResult<Json<Vec<PlatformProviderKey>>> {
    let keys = db::list_platform_keys(state.db.pool()).await?;
    Ok(Json(keys))
}

pub async fn upsert(
    State(state): State<AppState>,
    _user: AuthUser,
    Json(body): Json<UpsertPlatformKeyRequest>,
) -> ApiResult<Json<PlatformProviderKey>> {
    let encrypted = state
        .secrets
        .encrypt(&body.api_key)
        .map_err(|e| embed_proxy_core::ApiError::EncryptionError(e.to_string()))?;

    let key = db::upsert_platform_key(
        state.db.pool(),
        body.provider_type,
        &encrypted,
        body.base_url.as_deref(),
    )
    .await?;
    Ok(Json(key))
}

pub async fn update(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdatePlatformKeyRequest>,
) -> ApiResult<Json<PlatformProviderKey>> {
    let key = db::set_platform_key_active(state.db.pool(), id, body.is_active.unwrap_or(true))
        .await?;
    Ok(Json(key))
}

pub async fn delete_one(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    db::delete_platform_key(state.db.pool(), id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
