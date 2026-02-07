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
pub trait TaskServiceHandler: Send + Sync + 'static {
    async fn list(&self, query: TaskServiceListQuery) -> Result<Vec<Task>, ApiError>;
    async fn create(&self, body: CreateTaskInput) -> Result<IdResponse, ApiError>;
    async fn get(&self, id: i64) -> Result<TaskWithDependencies, ApiError>;
    async fn update(&self, id: i64, body: UpdateTaskInput) -> Result<Task, ApiError>;
    async fn delete(&self, id: i64) -> Result<DeletedResponse, ApiError>;
    async fn update_status(&self, id: i64, body: UpdateStatusInput) -> Result<Task, ApiError>;
    async fn get_dependencies(&self, id: i64) -> Result<Vec<Task>, ApiError>;
    async fn add_dependency(&self, id: i64, body: AddDependencyInput) -> Result<DependencyResponse, ApiError>;
    async fn remove_dependency(&self, id: i64, dep_id: i64) -> Result<RemovedResponse, ApiError>;
    async fn get_dependents(&self, id: i64) -> Result<Vec<Task>, ApiError>;
    async fn search(&self, query: TaskServiceSearchQuery) -> Result<Vec<Task>, ApiError>;
    async fn get_ready(&self) -> Result<Vec<Task>, ApiError>;
    async fn get_blocked(&self) -> Result<Vec<Task>, ApiError>;
    async fn link_to_symbol(&self, id: i64, symbol_id: i64) -> Result<LinkResponse, ApiError>;
    async fn unlink_symbol(&self, id: i64) -> Result<UnlinkResponse, ApiError>;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskServiceListQuery {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskServiceSearchQuery {
    pub q: String,
    pub limit: Option<i32>,
}

async fn task_service_list<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<TaskServiceListQuery>,
) -> Result<Json<Vec<Task>>, ApiError> {
    let result = state.list(query).await?;
    Ok(Json(result))
}

async fn task_service_create<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Json(body): Json<CreateTaskInput>,
) -> Result<(StatusCode, Json<IdResponse>), ApiError> {
    let result = state.create(body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn task_service_get<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
) -> Result<Json<TaskWithDependencies>, ApiError> {
    let result = state.get(id).await?;
    Ok(Json(result))
}

async fn task_service_update<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateTaskInput>,
) -> Result<Json<Task>, ApiError> {
    let result = state.update(id, body).await?;
    Ok(Json(result))
}

async fn task_service_delete<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let result = state.delete(id).await?;
    Ok(Json(result))
}

async fn task_service_update_status<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateStatusInput>,
) -> Result<Json<Task>, ApiError> {
    let result = state.update_status(id, body).await?;
    Ok(Json(result))
}

async fn task_service_get_dependencies<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Task>>, ApiError> {
    let result = state.get_dependencies(id).await?;
    Ok(Json(result))
}

async fn task_service_add_dependency<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
    Json(body): Json<AddDependencyInput>,
) -> Result<(StatusCode, Json<DependencyResponse>), ApiError> {
    let result = state.add_dependency(id, body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

async fn task_service_remove_dependency<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path((id, dep_id)):  Path<(i64, i64)>,
) -> Result<Json<RemovedResponse>, ApiError> {
    let result = state.remove_dependency(id, dep_id).await?;
    Ok(Json(result))
}

async fn task_service_get_dependents<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Task>>, ApiError> {
    let result = state.get_dependents(id).await?;
    Ok(Json(result))
}

async fn task_service_search<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Query(query): Query<TaskServiceSearchQuery>,
) -> Result<Json<Vec<Task>>, ApiError> {
    let result = state.search(query).await?;
    Ok(Json(result))
}

async fn task_service_get_ready<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Vec<Task>>, ApiError> {
    let result = state.get_ready().await?;
    Ok(Json(result))
}

async fn task_service_get_blocked<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Vec<Task>>, ApiError> {
    let result = state.get_blocked().await?;
    Ok(Json(result))
}

async fn task_service_link_to_symbol<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path((id, symbol_id)):  Path<(i64, i64)>,
) -> Result<Json<LinkResponse>, ApiError> {
    let result = state.link_to_symbol(id, symbol_id).await?;
    Ok(Json(result))
}

async fn task_service_unlink_symbol<S: TaskServiceHandler>(
    State(state): State<Arc<S>>,
    Path(id): Path<i64>,
) -> Result<Json<UnlinkResponse>, ApiError> {
    let result = state.unlink_symbol(id).await?;
    Ok(Json(result))
}

pub fn task_service_routes<S: TaskServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/tasks", get(task_service_list::<S>).post(task_service_create::<S>))
        .route("/tasks/:id", get(task_service_get::<S>).put(task_service_update::<S>).delete(task_service_delete::<S>))
        .route("/tasks/:id/status", put(task_service_update_status::<S>))
        .route("/tasks/:id/dependencies", get(task_service_get_dependencies::<S>).post(task_service_add_dependency::<S>))
        .route("/tasks/:id/dependencies/:depId", delete(task_service_remove_dependency::<S>))
        .route("/tasks/:id/dependents", get(task_service_get_dependents::<S>))
        .route("/tasks/search", get(task_service_search::<S>))
        .route("/tasks/ready", get(task_service_get_ready::<S>))
        .route("/tasks/blocked", get(task_service_get_blocked::<S>))
        .route("/tasks/:id/link/:symbolId", put(task_service_link_to_symbol::<S>))
        .route("/tasks/:id/link", delete(task_service_unlink_symbol::<S>))
}

#[async_trait]
pub trait GraphServiceHandler: Send + Sync + 'static {
    async fn get_graph(&self) -> Result<Vec<GraphNode>, ApiError>;
    async fn detect_cycles(&self) -> Result<CyclesResponse, ApiError>;
}

async fn graph_service_get_graph<S: GraphServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<Vec<GraphNode>>, ApiError> {
    let result = state.get_graph().await?;
    Ok(Json(result))
}

async fn graph_service_detect_cycles<S: GraphServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<CyclesResponse>, ApiError> {
    let result = state.detect_cycles().await?;
    Ok(Json(result))
}

pub fn graph_service_routes<S: GraphServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/graph", get(graph_service_get_graph::<S>))
        .route("/graph/cycles", get(graph_service_detect_cycles::<S>))
}

#[async_trait]
pub trait StatusServiceHandler: Send + Sync + 'static {
    async fn get_status(&self) -> Result<TasksStatus, ApiError>;
}

async fn status_service_get_status<S: StatusServiceHandler>(
    State(state): State<Arc<S>>,
) -> Result<Json<TasksStatus>, ApiError> {
    let result = state.get_status().await?;
    Ok(Json(result))
}

pub fn status_service_routes<S: StatusServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .route("/status", get(status_service_get_status::<S>))
}

pub fn create_router<S: TaskServiceHandler + GraphServiceHandler + StatusServiceHandler>() -> Router<Arc<S>> {
    Router::new()
        .merge(task_service_routes())
        .merge(graph_service_routes())
        .merge(status_service_routes())
}
