pub mod api;
pub mod overview;
pub mod tasks;
pub mod users;

use axum::{Json, Router, response::IntoResponse, routing::get};
use sqlx::PgPool;

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

pub fn create_router(pool: PgPool) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Overview
        .route("/overview", get(overview::get_overview))
        // Users
        .route("/users/daily", get(users::get_daily_active_users))
        .route("/users/weekly", get(users::get_weekly_active_users))
        // Tasks
        .route("/tasks/daily", get(tasks::get_task_stats_daily))
        .route("/tasks/overview", get(tasks::get_task_stats_overview))
        // API Performance
        .route("/api/latency", get(api::get_endpoint_latency))
        .route("/api/slowest", get(api::get_slowest_endpoints))
        .with_state(pool)
}
