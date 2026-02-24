use analytics_client::TimeRangeParams;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use sqlx::PgPool;

pub async fn get_task_stats_daily(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<analytics_client::TaskStats>>, StatusCode> {
    analytics_client::get_task_stats_daily(&pool, params.start_date, params.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch task stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn get_task_stats_overview(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<analytics_client::TaskStatsOverview>, StatusCode> {
    analytics_client::get_task_stats_overview(&pool, params.start_date, params.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch task stats overview: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
