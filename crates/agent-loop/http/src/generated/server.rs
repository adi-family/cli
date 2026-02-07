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
pub trait AgentLoopServiceHandler: Send + Sync + 'static {
    async fn get_status(&self) -> Result<StatusResponse, ApiError>;
    async fn run(&self, body: RunRequest) -> Result<RunResponse, ApiError>;
}

async fn agent_loop_service_get_status<S: AgentLoopServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<StatusResponse>, ApiError> {
    let result = state.get_status().await?;
    Ok(Json(result))
}

async fn agent_loop_service_run<S: AgentLoopServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<RunRequest>,
) -> Result<Json<RunResponse>, ApiError> {
    let result = state.run(body).await?;
    Ok(Json(result))
}

pub fn agent_loop_service_routes<S: AgentLoopServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/api/status", get(agent_loop_service_get_status::<S>))
        .route("/api/run", post(agent_loop_service_run::<S>))
}

pub fn create_router<S: AgentLoopServiceHandler>() -> Router<Arc<S>> {
    agent_loop_service_routes()
}
