//! Auto-generated server handlers from TypeSpec.
//! DO NOT EDIT.
//!
//! Implement the handler traits and use the generated router.

#![allow(unused_imports, dead_code)]

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
pub trait AuthServiceHandler: Send + Sync + 'static {
    async fn request_code(&self, body: RequestCodeInput) -> Result<MessageResponse, ApiError>;
    async fn verify_code(&self, body: VerifyCodeInput) -> Result<AuthToken, ApiError>;
    async fn verify_totp(&self, body: VerifyTotpInput) -> Result<AuthToken, ApiError>;
    async fn get_current_user(&self) -> Result<UserInfo, ApiError>;
    async fn setup_totp(&self) -> Result<TotpSetup, ApiError>;
    async fn enable_totp(&self, body: EnableTotpInput) -> Result<MessageResponse, ApiError>;
    async fn disable_totp(&self) -> Result<MessageResponse, ApiError>;
}

async fn auth_service_request_code<S: AuthServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<RequestCodeInput>,
) -> Result<Json<MessageResponse>, ApiError> {
    let result = state.request_code(body).await?;
    Ok(Json(result))
}

async fn auth_service_verify_code<S: AuthServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<VerifyCodeInput>,
) -> Result<Json<AuthToken>, ApiError> {
    let result = state.verify_code(body).await?;
    Ok(Json(result))
}

async fn auth_service_verify_totp<S: AuthServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<VerifyTotpInput>,
) -> Result<Json<AuthToken>, ApiError> {
    let result = state.verify_totp(body).await?;
    Ok(Json(result))
}

async fn auth_service_get_current_user<S: AuthServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<UserInfo>, ApiError> {
    let result = state.get_current_user().await?;
    Ok(Json(result))
}

async fn auth_service_setup_totp<S: AuthServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<TotpSetup>, ApiError> {
    let result = state.setup_totp().await?;
    Ok(Json(result))
}

async fn auth_service_enable_totp<S: AuthServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<EnableTotpInput>,
) -> Result<Json<MessageResponse>, ApiError> {
    let result = state.enable_totp(body).await?;
    Ok(Json(result))
}

async fn auth_service_disable_totp<S: AuthServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<MessageResponse>, ApiError> {
    let result = state.disable_totp().await?;
    Ok(Json(result))
}

pub fn auth_service_routes<S: AuthServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/auth/request-code", post(auth_service_request_code::<S>))
        .route("/auth/verify", post(auth_service_verify_code::<S>))
        .route("/auth/verify-totp", post(auth_service_verify_totp::<S>))
        .route("/auth/me", get(auth_service_get_current_user::<S>))
        .route("/auth/totp/setup", post(auth_service_setup_totp::<S>))
        .route("/auth/totp/enable", post(auth_service_enable_totp::<S>))
        .route("/auth/totp/disable", post(auth_service_disable_totp::<S>))
}

pub fn create_router<S: AuthServiceHandler>() -> Router<Arc<S>> {
    auth_service_routes()
}
