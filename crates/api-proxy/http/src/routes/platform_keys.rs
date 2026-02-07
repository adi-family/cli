//! Platform provider key management routes (admin only).
//!
//! These routes require admin authentication via `ADMIN_JWT_SECRET`.
//! Regular users cannot access platform key management.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{middleware::AdminUser, state::AppState};
use api_proxy_core::{db, ApiResult, ProviderType};

/// Platform key summary (without exposing encrypted key).
#[derive(Debug, Serialize)]
pub struct PlatformKeySummary {
    pub id: Uuid,
    pub provider_type: ProviderType,
    pub base_url: Option<String>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Request to create/update a platform provider key.
#[derive(Debug, Deserialize)]
pub struct UpsertPlatformKeyRequest {
    pub provider_type: ProviderType,
    pub api_key: String,
    pub base_url: Option<String>,
}

/// Request to update platform key status.
#[derive(Debug, Deserialize)]
pub struct UpdatePlatformKeyRequest {
    pub is_active: Option<bool>,
}

/// List all platform provider keys.
pub async fn list_platform_keys(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> ApiResult<Json<Vec<PlatformKeySummary>>> {
    let keys = db::platform_keys::list_platform_keys(state.db.pool()).await?;
    Ok(Json(
        keys.into_iter()
            .map(|k| PlatformKeySummary {
                id: k.id,
                provider_type: k.provider_type,
                base_url: k.base_url,
                is_active: k.is_active,
                created_at: k.created_at,
                updated_at: k.updated_at,
            })
            .collect(),
    ))
}

/// Create or update a platform provider key.
pub async fn upsert_platform_key(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<UpsertPlatformKeyRequest>,
) -> ApiResult<Json<PlatformKeySummary>> {
    // Encrypt the API key
    let encrypted = state.secrets.encrypt(&req.api_key)?;

    let key = db::platform_keys::upsert_platform_key(
        state.db.pool(),
        req.provider_type,
        &encrypted,
        req.base_url.as_deref(),
    )
    .await?;

    Ok(Json(PlatformKeySummary {
        id: key.id,
        provider_type: key.provider_type,
        base_url: key.base_url,
        is_active: key.is_active,
        created_at: key.created_at,
        updated_at: key.updated_at,
    }))
}

/// Update a platform provider key (status only).
pub async fn update_platform_key(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePlatformKeyRequest>,
) -> ApiResult<Json<PlatformKeySummary>> {
    let key = if let Some(is_active) = req.is_active {
        db::platform_keys::set_platform_key_active(state.db.pool(), id, is_active).await?
    } else {
        // No changes, just fetch
        let keys = db::platform_keys::list_platform_keys(state.db.pool()).await?;
        keys.into_iter().find(|k| k.id == id).ok_or_else(|| {
            api_proxy_core::ApiError::NotFound("Platform key not found".into())
        })?
    };

    Ok(Json(PlatformKeySummary {
        id: key.id,
        provider_type: key.provider_type,
        base_url: key.base_url,
        is_active: key.is_active,
        created_at: key.created_at,
        updated_at: key.updated_at,
    }))
}

/// Delete a platform provider key.
pub async fn delete_platform_key(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    db::platform_keys::delete_platform_key(state.db.pool(), id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Verify a platform provider key by testing connectivity.
#[derive(Debug, Serialize)]
pub struct VerifyKeyResponse {
    pub valid: bool,
    pub models: Option<Vec<String>>,
    pub error: Option<String>,
}

pub async fn verify_platform_key(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(provider_type): Path<ProviderType>,
) -> ApiResult<Json<VerifyKeyResponse>> {
    let key = db::platform_keys::get_platform_key(state.db.pool(), provider_type).await?;

    // Decrypt the API key
    let api_key = state.secrets.decrypt(&key.api_key_encrypted)?;

    // Create provider and test
    let provider =
        api_proxy_core::providers::create_provider(key.provider_type, key.base_url.clone());

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
