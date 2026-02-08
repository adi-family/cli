//! Auto-generated server handlers from TypeSpec.
//! DO NOT EDIT.
//!
//! Implement the trait to provide your business logic.

#![allow(unused_imports)]

use super::models::*;
use super::enums::*;
use async_trait::async_trait;
use axum::{extract::{Path, Query, State}, http::StatusCode, Json, Router};
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
pub trait CredentialsServiceHandler: Send + Sync + 'static {
    async fn list(&self, credential_type: Option<CredentialType>, provider: Option<String>) -> Result<Vec<Credential>, ApiError>;
    async fn create(&self, body: CreateCredential) -> Result<Credential, ApiError>;
    async fn get(&self, id: Uuid) -> Result<Credential, ApiError>;
    async fn update(&self, id: Uuid, body: UpdateCredential) -> Result<Credential, ApiError>;
    async fn delete(&self, id: Uuid) -> Result<DeleteResult, ApiError>;
    async fn get_with_data(&self, id: Uuid) -> Result<CredentialWithData, ApiError>;
    async fn get_access_logs(&self, id: Uuid) -> Result<Vec<CredentialAccessLog>, ApiError>;
    async fn verify(&self, id: Uuid) -> Result<VerifyResult, ApiError>;
}
