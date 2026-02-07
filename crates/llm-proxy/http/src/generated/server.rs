//! Auto-generated server handlers from TypeSpec.
//! DO NOT EDIT.
//!
//! Implement the handler traits and use the generated router.

#![allow(unused_imports)]

use super::models::*;
use super::enums::*;
use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;


#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    pub status: u16,
    pub code: String,
    pub message: String,
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}


#[async_trait]
pub trait KeysServiceHandler: Send + Sync + 'static {
    async fn create_key(&self, body: CreateKeyRequest) -> Result<CreateKeyResponse, ApiError>;
    async fn list_keys(&self) -> Result<Vec<UpstreamApiKeySummary>, ApiError>;
    async fn get_key(&self, id: Uuid) -> Result<UpstreamApiKeySummary, ApiError>;
    async fn update_key(&self, id: Uuid, body: UpdateKeyRequest) -> Result<UpstreamApiKeySummary, ApiError>;
    async fn delete_key(&self, id: Uuid) -> Result<DeletedResponse, ApiError>;
    async fn verify_key(&self, id: Uuid) -> Result<VerifyKeyResponse, ApiError>;
}

async fn keys_service_create_key<S: KeysServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<CreateKeyRequest>,
) -> Result<(StatusCode, Json<CreateKeyResponse>), ApiError> {
    let result = state.create_key(body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn keys_service_list_keys<S: KeysServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Vec<UpstreamApiKeySummary>>, ApiError> {
    let result = state.list_keys().await?;
    Ok(Json(result))
}

async fn keys_service_get_key<S: KeysServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<UpstreamApiKeySummary>, ApiError> {
    let result = state.get_key(id).await?;
    Ok(Json(result))
}

async fn keys_service_update_key<S: KeysServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateKeyRequest>,
) -> Result<Json<UpstreamApiKeySummary>, ApiError> {
    let result = state.update_key(id, body).await?;
    Ok(Json(result))
}

async fn keys_service_delete_key<S: KeysServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let result = state.delete_key(id).await?;
    Ok(Json(result))
}

async fn keys_service_verify_key<S: KeysServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<VerifyKeyResponse>, ApiError> {
    let result = state.verify_key(id).await?;
    Ok(Json(result))
}

pub fn keys_service_routes<S: KeysServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/api/llm-proxy/keys", post(keys_service_create_key::<S>).get(keys_service_list_keys::<S>))
        .route("/api/llm-proxy/keys/:id", get(keys_service_get_key::<S>).patch(keys_service_update_key::<S>).delete(keys_service_delete_key::<S>))
        .route("/api/llm-proxy/keys/:id/verify", post(keys_service_verify_key::<S>))
}

#[async_trait]
pub trait PlatformKeysServiceHandler: Send + Sync + 'static {
    async fn list_platform_keys(&self) -> Result<Vec<PlatformKeySummary>, ApiError>;
    async fn upsert_platform_key(&self, body: UpsertPlatformKeyRequest) -> Result<PlatformKeySummary, ApiError>;
    async fn update_platform_key(&self, id: Uuid, body: UpdatePlatformKeyRequest) -> Result<PlatformKeySummary, ApiError>;
    async fn delete_platform_key(&self, id: Uuid) -> Result<DeletedResponse, ApiError>;
    async fn verify_platform_key(&self, provider_type: String) -> Result<VerifyKeyResponse, ApiError>;
}

async fn platform_keys_service_list_platform_keys<S: PlatformKeysServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Vec<PlatformKeySummary>>, ApiError> {
    let result = state.list_platform_keys().await?;
    Ok(Json(result))
}

async fn platform_keys_service_upsert_platform_key<S: PlatformKeysServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<UpsertPlatformKeyRequest>,
) -> Result<Json<PlatformKeySummary>, ApiError> {
    let result = state.upsert_platform_key(body).await?;
    Ok(Json(result))
}

async fn platform_keys_service_update_platform_key<S: PlatformKeysServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdatePlatformKeyRequest>,
) -> Result<Json<PlatformKeySummary>, ApiError> {
    let result = state.update_platform_key(id, body).await?;
    Ok(Json(result))
}

async fn platform_keys_service_delete_platform_key<S: PlatformKeysServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let result = state.delete_platform_key(id).await?;
    Ok(Json(result))
}

async fn platform_keys_service_verify_platform_key<S: PlatformKeysServiceHandler>(
    State(state): State<Arc<S>>,
    Path(provider_type): Path<String>,
) -> Result<Json<VerifyKeyResponse>, ApiError> {
    let result = state.verify_platform_key(provider_type).await?;
    Ok(Json(result))
}

pub fn platform_keys_service_routes<S: PlatformKeysServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/api/llm-proxy/platform-keys", get(platform_keys_service_list_platform_keys::<S>).post(platform_keys_service_upsert_platform_key::<S>))
        .route("/api/llm-proxy/platform-keys/:id", patch(platform_keys_service_update_platform_key::<S>).delete(platform_keys_service_delete_platform_key::<S>))
        .route("/api/llm-proxy/platform-keys/:providerType/verify", post(platform_keys_service_verify_platform_key::<S>))
}

#[async_trait]
pub trait TokensServiceHandler: Send + Sync + 'static {
    async fn create_token(&self, body: CreateTokenRequest) -> Result<CreateTokenResponse, ApiError>;
    async fn list_tokens(&self) -> Result<Vec<ProxyTokenSummary>, ApiError>;
    async fn get_token(&self, id: Uuid) -> Result<ProxyTokenSummary, ApiError>;
    async fn update_token(&self, id: Uuid, body: UpdateTokenRequest) -> Result<ProxyTokenSummary, ApiError>;
    async fn delete_token(&self, id: Uuid) -> Result<DeletedResponse, ApiError>;
    async fn rotate_token(&self, id: Uuid) -> Result<RotateTokenResponse, ApiError>;
}

async fn tokens_service_create_token<S: TokensServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<CreateTokenRequest>,
) -> Result<(StatusCode, Json<CreateTokenResponse>), ApiError> {
    let result = state.create_token(body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn tokens_service_list_tokens<S: TokensServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Vec<ProxyTokenSummary>>, ApiError> {
    let result = state.list_tokens().await?;
    Ok(Json(result))
}

async fn tokens_service_get_token<S: TokensServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProxyTokenSummary>, ApiError> {
    let result = state.get_token(id).await?;
    Ok(Json(result))
}

async fn tokens_service_update_token<S: TokensServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTokenRequest>,
) -> Result<Json<ProxyTokenSummary>, ApiError> {
    let result = state.update_token(id, body).await?;
    Ok(Json(result))
}

async fn tokens_service_delete_token<S: TokensServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let result = state.delete_token(id).await?;
    Ok(Json(result))
}

async fn tokens_service_rotate_token<S: TokensServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<RotateTokenResponse>, ApiError> {
    let result = state.rotate_token(id).await?;
    Ok(Json(result))
}

pub fn tokens_service_routes<S: TokensServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/api/llm-proxy/tokens", post(tokens_service_create_token::<S>).get(tokens_service_list_tokens::<S>))
        .route("/api/llm-proxy/tokens/:id", get(tokens_service_get_token::<S>).patch(tokens_service_update_token::<S>).delete(tokens_service_delete_token::<S>))
        .route("/api/llm-proxy/tokens/:id/rotate", post(tokens_service_rotate_token::<S>))
}

#[async_trait]
pub trait ProvidersServiceHandler: Send + Sync + 'static {
    async fn list_providers(&self) -> Result<ListProvidersResponse, ApiError>;
}

async fn providers_service_list_providers<S: ProvidersServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<ListProvidersResponse>, ApiError> {
    let result = state.list_providers().await?;
    Ok(Json(result))
}

pub fn providers_service_routes<S: ProvidersServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/api/llm-proxy/providers", get(providers_service_list_providers::<S>))
}

#[async_trait]
pub trait UsageServiceHandler: Send + Sync + 'static {
    async fn query_usage(&self, query: UsageServiceQueryUsageQuery) -> Result<UsageResponse, ApiError>;
    async fn usage_summary(&self, query: UsageServiceUsageSummaryQuery) -> Result<std::collections::HashMap<String, serde_json::Value>, ApiError>;
    async fn export_usage(&self, query: UsageServiceExportUsageQuery) -> Result<String, ApiError>;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageServiceQueryUsageQuery {
    pub proxy_token_id: Option<Uuid>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageServiceUsageSummaryQuery {
    pub proxy_token_id: Option<Uuid>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageServiceExportUsageQuery {
    pub proxy_token_id: Option<Uuid>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub format: Option<String>,
}

async fn usage_service_query_usage<S: UsageServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<UsageServiceQueryUsageQuery>,
) -> Result<Json<UsageResponse>, ApiError> {
    let result = state.query_usage(query).await?;
    Ok(Json(result))
}

async fn usage_service_usage_summary<S: UsageServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<UsageServiceUsageSummaryQuery>,
) -> Result<Json<std::collections::HashMap<String, serde_json::Value>>, ApiError> {
    let result = state.usage_summary(query).await?;
    Ok(Json(result))
}

async fn usage_service_export_usage<S: UsageServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<UsageServiceExportUsageQuery>,
) -> Result<Json<String>, ApiError> {
    let result = state.export_usage(query).await?;
    Ok(Json(result))
}

pub fn usage_service_routes<S: UsageServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/api/llm-proxy/usage", get(usage_service_query_usage::<S>))
        .route("/api/llm-proxy/usage/summary", get(usage_service_usage_summary::<S>))
        .route("/api/llm-proxy/usage/export", get(usage_service_export_usage::<S>))
}

#[async_trait]
pub trait ProxyServiceHandler: Send + Sync + 'static {
    async fn chat_completions(&self, body: std::collections::HashMap<String, serde_json::Value>) -> Result<std::collections::HashMap<String, serde_json::Value>, ApiError>;
    async fn completions(&self, body: std::collections::HashMap<String, serde_json::Value>) -> Result<std::collections::HashMap<String, serde_json::Value>, ApiError>;
    async fn embeddings(&self, body: std::collections::HashMap<String, serde_json::Value>) -> Result<std::collections::HashMap<String, serde_json::Value>, ApiError>;
    async fn messages(&self, body: std::collections::HashMap<String, serde_json::Value>) -> Result<std::collections::HashMap<String, serde_json::Value>, ApiError>;
    async fn list_models(&self) -> Result<ModelsResponse, ApiError>;
}

async fn proxy_service_chat_completions<S: ProxyServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<Json<std::collections::HashMap<String, serde_json::Value>>, ApiError> {
    let result = state.chat_completions(body).await?;
    Ok(Json(result))
}

async fn proxy_service_completions<S: ProxyServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<Json<std::collections::HashMap<String, serde_json::Value>>, ApiError> {
    let result = state.completions(body).await?;
    Ok(Json(result))
}

async fn proxy_service_embeddings<S: ProxyServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<Json<std::collections::HashMap<String, serde_json::Value>>, ApiError> {
    let result = state.embeddings(body).await?;
    Ok(Json(result))
}

async fn proxy_service_messages<S: ProxyServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<Json<std::collections::HashMap<String, serde_json::Value>>, ApiError> {
    let result = state.messages(body).await?;
    Ok(Json(result))
}

async fn proxy_service_list_models<S: ProxyServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<ModelsResponse>, ApiError> {
    let result = state.list_models().await?;
    Ok(Json(result))
}

pub fn proxy_service_routes<S: ProxyServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/v1/chat/completions", post(proxy_service_chat_completions::<S>))
        .route("/v1/completions", post(proxy_service_completions::<S>))
        .route("/v1/embeddings", post(proxy_service_embeddings::<S>))
        .route("/v1/messages", post(proxy_service_messages::<S>))
        .route("/v1/models", get(proxy_service_list_models::<S>))
}

pub fn create_router<S: KeysServiceHandler + PlatformKeysServiceHandler + TokensServiceHandler + ProvidersServiceHandler + UsageServiceHandler + ProxyServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .merge(keys_service_routes())
        .merge(platform_keys_service_routes())
        .merge(tokens_service_routes())
        .merge(providers_service_routes())
        .merge(usage_service_routes())
        .merge(proxy_service_routes())
}
