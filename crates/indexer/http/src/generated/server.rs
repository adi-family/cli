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
pub trait IndexerServiceHandler: Send + Sync + 'static {
    async fn get_status(&self) -> Result<Status, ApiError>;
    async fn index_project(&self) -> Result<IndexProgress, ApiError>;
    async fn search(&self, query: IndexerServiceSearchQuery) -> Result<Vec<SearchResult>, ApiError>;
    async fn search_symbols(&self, query: IndexerServiceSearchSymbolsQuery) -> Result<Vec<SearchResult>, ApiError>;
    async fn get_symbol(&self, id: i64) -> Result<Symbol, ApiError>;
    async fn get_symbol_reachability(&self, id: i64) -> Result<ReachabilityResponse, ApiError>;
    async fn search_files(&self, query: IndexerServiceSearchFilesQuery) -> Result<Vec<File>, ApiError>;
    async fn get_file(&self, path: String) -> Result<FileInfo, ApiError>;
    async fn get_tree(&self) -> Result<Tree, ApiError>;
    async fn find_dead_code(&self, query: IndexerServiceFindDeadCodeQuery) -> Result<DeadCodeReport, ApiError>;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexerServiceSearchQuery {
    pub q: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexerServiceSearchSymbolsQuery {
    pub q: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexerServiceSearchFilesQuery {
    pub q: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexerServiceFindDeadCodeQuery {
    pub mode: Option<String>,
    pub exclude_tests: Option<bool>,
    pub exclude_traits: Option<bool>,
    pub exclude_ffi: Option<bool>,
}

async fn indexer_service_get_status<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Status>, ApiError> {
    let result = state.get_status().await?;
    Ok(Json(result))
}

async fn indexer_service_index_project<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<IndexProgress>, ApiError> {
    let result = state.index_project().await?;
    Ok(Json(result))
}

async fn indexer_service_search<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<IndexerServiceSearchQuery>,
) -> Result<Json<Vec<SearchResult>>, ApiError> {
    let result = state.search(query).await?;
    Ok(Json(result))
}

async fn indexer_service_search_symbols<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<IndexerServiceSearchSymbolsQuery>,
) -> Result<Json<Vec<SearchResult>>, ApiError> {
    let result = state.search_symbols(query).await?;
    Ok(Json(result))
}

async fn indexer_service_get_symbol<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
) -> Result<Json<Symbol>, ApiError> {
    let result = state.get_symbol(id).await?;
    Ok(Json(result))
}

async fn indexer_service_get_symbol_reachability<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
) -> Result<Json<ReachabilityResponse>, ApiError> {
    let result = state.get_symbol_reachability(id).await?;
    Ok(Json(result))
}

async fn indexer_service_search_files<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<IndexerServiceSearchFilesQuery>,
) -> Result<Json<Vec<File>>, ApiError> {
    let result = state.search_files(query).await?;
    Ok(Json(result))
}

async fn indexer_service_get_file<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
    Path(path): Path<String>,
) -> Result<Json<FileInfo>, ApiError> {
    let result = state.get_file(path).await?;
    Ok(Json(result))
}

async fn indexer_service_get_tree<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Tree>, ApiError> {
    let result = state.get_tree().await?;
    Ok(Json(result))
}

async fn indexer_service_find_dead_code<S: IndexerServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<IndexerServiceFindDeadCodeQuery>,
) -> Result<Json<DeadCodeReport>, ApiError> {
    let result = state.find_dead_code(query).await?;
    Ok(Json(result))
}

pub fn indexer_service_routes<S: IndexerServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/status", get(indexer_service_get_status::<S>))
        .route("/index", post(indexer_service_index_project::<S>))
        .route("/search", get(indexer_service_search::<S>))
        .route("/symbols", get(indexer_service_search_symbols::<S>))
        .route("/symbols/:id", get(indexer_service_get_symbol::<S>))
        .route("/symbols/:id/reachability", get(indexer_service_get_symbol_reachability::<S>))
        .route("/files", get(indexer_service_search_files::<S>))
        .route("/files/:path", get(indexer_service_get_file::<S>))
        .route("/tree", get(indexer_service_get_tree::<S>))
        .route("/dead-code", get(indexer_service_find_dead_code::<S>))
}

pub fn create_router<S: IndexerServiceHandler>() -> Router<Arc<S>> {
    indexer_service_routes()
}
