pub mod api;
pub mod overview;
pub mod tasks;
pub mod users;

use axum::{routing::get, Router};
use sqlx::PgPool;

pub fn create_router(pool: PgPool) -> Router {
    Router::new()
        // Overview
        .route("/api/analytics/overview", get(overview::get_overview))
        // Users
        .route("/api/analytics/users/daily", get(users::get_daily_active_users))
        .route("/api/analytics/users/weekly", get(users::get_weekly_active_users))
        // Tasks
        .route("/api/analytics/tasks/daily", get(tasks::get_task_stats_daily))
        .route("/api/analytics/tasks/overview", get(tasks::get_task_stats_overview))
        // API Performance
        .route("/api/analytics/api/latency", get(api::get_endpoint_latency))
        .route("/api/analytics/api/slowest", get(api::get_slowest_endpoints))
        .with_state(pool)
}
