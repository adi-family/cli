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
pub trait KnowledgebaseServiceHandler: Send + Sync + 'static {
    async fn get_status(&self) -> Result<StatusResponse, ApiError>;
    async fn add_node(&self, body: AddRequest) -> Result<Node, ApiError>;
    async fn get_node(&self, id: Uuid) -> Result<Node, ApiError>;
    async fn delete_node(&self, id: Uuid) -> Result<DeletedResponse, ApiError>;
    async fn approve_node(&self, id: Uuid) -> Result<ApprovedResponse, ApiError>;
    async fn query(&self, query: KnowledgebaseServiceQueryQuery) -> Result<Vec<SearchResult>, ApiError>;
    async fn subgraph(&self, query: KnowledgebaseServiceSubgraphQuery) -> Result<Subgraph, ApiError>;
    async fn get_conflicts(&self) -> Result<Vec<ConflictPair>, ApiError>;
    async fn get_orphans(&self) -> Result<Vec<Node>, ApiError>;
    async fn add_edge(&self, body: LinkRequest) -> Result<Edge, ApiError>;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgebaseServiceQueryQuery {
    pub q: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgebaseServiceSubgraphQuery {
    pub q: String,
    pub limit: Option<i32>,
}

async fn knowledgebase_service_get_status<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<StatusResponse>, ApiError> {
    let result = state.get_status().await?;
    Ok(Json(result))
}

async fn knowledgebase_service_add_node<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<AddRequest>,
) -> Result<(StatusCode, Json<Node>), ApiError> {
    let result = state.add_node(body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn knowledgebase_service_get_node<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Node>, ApiError> {
    let result = state.get_node(id).await?;
    Ok(Json(result))
}

async fn knowledgebase_service_delete_node<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let result = state.delete_node(id).await?;
    Ok(Json(result))
}

async fn knowledgebase_service_approve_node<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApprovedResponse>, ApiError> {
    let result = state.approve_node(id).await?;
    Ok(Json(result))
}

async fn knowledgebase_service_query<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<KnowledgebaseServiceQueryQuery>,
) -> Result<Json<Vec<SearchResult>>, ApiError> {
    let result = state.query(query).await?;
    Ok(Json(result))
}

async fn knowledgebase_service_subgraph<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<KnowledgebaseServiceSubgraphQuery>,
) -> Result<Json<Subgraph>, ApiError> {
    let result = state.subgraph(query).await?;
    Ok(Json(result))
}

async fn knowledgebase_service_get_conflicts<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Vec<ConflictPair>>, ApiError> {
    let result = state.get_conflicts().await?;
    Ok(Json(result))
}

async fn knowledgebase_service_get_orphans<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Vec<Node>>, ApiError> {
    let result = state.get_orphans().await?;
    Ok(Json(result))
}

async fn knowledgebase_service_add_edge<S: KnowledgebaseServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<LinkRequest>,
) -> Result<(StatusCode, Json<Edge>), ApiError> {
    let result = state.add_edge(body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

pub fn knowledgebase_service_routes<S: KnowledgebaseServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/status", get(knowledgebase_service_get_status::<S>))
        .route("/nodes", post(knowledgebase_service_add_node::<S>))
        .route("/nodes/:id", get(knowledgebase_service_get_node::<S>).delete(knowledgebase_service_delete_node::<S>))
        .route("/nodes/:id/approve", post(knowledgebase_service_approve_node::<S>))
        .route("/query", get(knowledgebase_service_query::<S>))
        .route("/subgraph", get(knowledgebase_service_subgraph::<S>))
        .route("/conflicts", get(knowledgebase_service_get_conflicts::<S>))
        .route("/orphans", get(knowledgebase_service_get_orphans::<S>))
        .route("/edges", post(knowledgebase_service_add_edge::<S>))
}

pub fn create_router<S: KnowledgebaseServiceHandler>() -> Router<Arc<S>> {
    knowledgebase_service_routes()
}
