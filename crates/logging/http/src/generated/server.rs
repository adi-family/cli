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
use chrono::{DateTime, Utc};
use serde::Deserialize;
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
pub trait LogServiceHandler: Send + Sync + 'static {
    async fn ingest(&self, body: Vec<LogEntry>) -> Result<IngestResponse, ApiError>;
    async fn query(&self, query: LogServiceQueryQuery) -> Result<LogQueryResult, ApiError>;
    async fn get_trace_logs(&self, trace_id: Uuid) -> Result<TraceLogsResult, ApiError>;
    async fn get_span_logs(&self, span_id: Uuid) -> Result<SpanLogsResult, ApiError>;
    async fn get_cocoon_logs(&self, cocoon_id: String, query: LogServiceGetCocoonLogsQuery) -> Result<CorrelationLogsResult, ApiError>;
    async fn get_user_logs(&self, user_id: String, query: LogServiceGetUserLogsQuery) -> Result<CorrelationLogsResult, ApiError>;
    async fn get_session_logs(&self, session_id: String, query: LogServiceGetSessionLogsQuery) -> Result<CorrelationLogsResult, ApiError>;
    async fn get_stats(&self) -> Result<StatsResponse, ApiError>;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogServiceQueryQuery {
    pub service: Option<String>,
    pub level: Option<String>,
    pub trace_id: Option<Uuid>,
    pub cocoon_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub hive_id: Option<String>,
    pub search: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogServiceGetCocoonLogsQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogServiceGetUserLogsQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogServiceGetSessionLogsQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

async fn log_service_ingest<S: LogServiceHandler>(
    State(state): State<S>,
    Json(body): Json<Vec<LogEntry>>,
) -> Result<Json<IngestResponse>, ApiError> {
    let result = state.ingest(body).await?;
    Ok(Json(result))
}

async fn log_service_query<S: LogServiceHandler>(
    State(state): State<S>,
    Query(query): Query<LogServiceQueryQuery>,
) -> Result<Json<LogQueryResult>, ApiError> {
    let result = state.query(query).await?;
    Ok(Json(result))
}

async fn log_service_get_trace_logs<S: LogServiceHandler>(
    State(state): State<S>,
    Path(trace_id): Path<Uuid>,
) -> Result<Json<TraceLogsResult>, ApiError> {
    let result = state.get_trace_logs(trace_id).await?;
    Ok(Json(result))
}

async fn log_service_get_span_logs<S: LogServiceHandler>(
    State(state): State<S>,
    Path(span_id): Path<Uuid>,
) -> Result<Json<SpanLogsResult>, ApiError> {
    let result = state.get_span_logs(span_id).await?;
    Ok(Json(result))
}

async fn log_service_get_cocoon_logs<S: LogServiceHandler>(
    State(state): State<S>,
    Path(cocoon_id): Path<String>,
    Query(query): Query<LogServiceGetCocoonLogsQuery>,
) -> Result<Json<CorrelationLogsResult>, ApiError> {
    let result = state.get_cocoon_logs(cocoon_id, query).await?;
    Ok(Json(result))
}

async fn log_service_get_user_logs<S: LogServiceHandler>(
    State(state): State<S>,
    Path(user_id): Path<String>,
    Query(query): Query<LogServiceGetUserLogsQuery>,
) -> Result<Json<CorrelationLogsResult>, ApiError> {
    let result = state.get_user_logs(user_id, query).await?;
    Ok(Json(result))
}

async fn log_service_get_session_logs<S: LogServiceHandler>(
    State(state): State<S>,
    Path(session_id): Path<String>,
    Query(query): Query<LogServiceGetSessionLogsQuery>,
) -> Result<Json<CorrelationLogsResult>, ApiError> {
    let result = state.get_session_logs(session_id, query).await?;
    Ok(Json(result))
}

async fn log_service_get_stats<S: LogServiceHandler>(
    State(state): State<S>,
) -> Result<Json<StatsResponse>, ApiError> {
    let result = state.get_stats().await?;
    Ok(Json(result))
}

pub fn log_service_routes<S: LogServiceHandler + Clone + 'static>() -> Router<S> {
    Router::new()
        .route("/logs/batch", post(log_service_ingest::<S>))
        .route("/logs", get(log_service_query::<S>))
        .route("/logs/trace/:traceId", get(log_service_get_trace_logs::<S>))
        .route("/logs/span/:spanId", get(log_service_get_span_logs::<S>))
        .route("/logs/cocoon/:cocoonId", get(log_service_get_cocoon_logs::<S>))
        .route("/logs/user/:userId", get(log_service_get_user_logs::<S>))
        .route("/logs/session/:sessionId", get(log_service_get_session_logs::<S>))
        .route("/logs/stats", get(log_service_get_stats::<S>))
}

pub fn create_router<S: LogServiceHandler + Clone + 'static>() -> Router<S> {
    log_service_routes()
}
