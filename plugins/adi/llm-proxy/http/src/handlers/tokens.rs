use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;
use crate::auth::AuthUser;
use llm_proxy_core::error::ApiResult;
use llm_proxy_core::types::{KeyMode, ProviderType, ProxyToken};
use llm_proxy_core::db;

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub key_mode: KeyMode,
    pub upstream_key_id: Option<Uuid>,
    pub platform_provider: Option<ProviderType>,
    pub request_script: Option<String>,
    pub response_script: Option<String>,
    pub allowed_models: Option<Vec<String>>,
    pub blocked_models: Option<Vec<String>>,
    pub log_requests: Option<bool>,
    pub log_responses: Option<bool>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct UpdateTokenRequest {
    pub name: Option<String>,
    pub request_script: Option<String>,
    pub response_script: Option<String>,
    pub allowed_models: Option<Vec<String>>,
    pub blocked_models: Option<Vec<String>>,
    pub log_requests: Option<bool>,
    pub log_responses: Option<bool>,
    pub is_active: Option<bool>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<ProxyToken>>> {
    let tokens = db::list_proxy_tokens(state.db.pool(), user.id).await?;
    Ok(Json(tokens))
}

pub async fn get_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ProxyToken>> {
    let token = db::get_proxy_token(state.db.pool(), id, user.id).await?;
    Ok(Json(token))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateTokenRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let (raw_token, prefix, hash) = db::tokens::generate_token();
    let token = db::create_proxy_token(
        state.db.pool(),
        user.id,
        &body.name,
        &hash,
        &prefix,
        body.key_mode,
        body.upstream_key_id,
        body.platform_provider,
        body.request_script.as_deref(),
        body.response_script.as_deref(),
        body.allowed_models.as_deref(),
        body.blocked_models.as_deref(),
        body.log_requests.unwrap_or(false),
        body.log_responses.unwrap_or(false),
        body.expires_at,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "token": token,
        "secret": raw_token
    })))
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTokenRequest>,
) -> ApiResult<Json<ProxyToken>> {
    let token = db::update_proxy_token(
        state.db.pool(),
        id,
        user.id,
        body.name.as_deref(),
        body.request_script.as_ref().map(|s| Some(s.as_str())),
        body.response_script.as_ref().map(|s| Some(s.as_str())),
        body.allowed_models.as_ref().map(|v| Some(v.as_slice())),
        body.blocked_models.as_ref().map(|v| Some(v.as_slice())),
        body.log_requests,
        body.log_responses,
        body.is_active,
        body.expires_at.map(Some),
    )
    .await?;
    Ok(Json(token))
}

pub async fn delete_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    db::delete_proxy_token(state.db.pool(), id, user.id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn rotate(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let (token, secret) = db::rotate_proxy_token(state.db.pool(), id, user.id).await?;
    Ok(Json(serde_json::json!({
        "token": token,
        "secret": secret
    })))
}
