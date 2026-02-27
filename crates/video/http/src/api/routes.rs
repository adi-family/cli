use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;
use super::handlers;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/v1/render", post(handlers::create_render))
        .route("/v1/render/{id}/frame", post(handlers::upload_frame))
        .route("/v1/render/{id}/finish", post(handlers::finish_upload))
        .route("/v1/render/{id}", get(handlers::get_status))
        .route("/v1/render/{id}/download", get(handlers::download))
        .route("/v1/jobs", get(handlers::list_jobs))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
