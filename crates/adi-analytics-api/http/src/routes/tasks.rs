use adi_analytics_api_core::TimeRangeParams;
use axum::{extract::{Query, State}, http::StatusCode, Json};
use sqlx::PgPool;

pub async fn get_task_stats_daily(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<adi_analytics_api_core::TaskStats>>, StatusCode> {
    adi_analytics_api_core::get_task_stats_daily(&pool, params.start_date, params.end_date)
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
) -> Result<Json<adi_analytics_api_core::TaskStatsOverview>, StatusCode> {
    adi_analytics_api_core::get_task_stats_overview(&pool, params.start_date, params.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch task stats overview: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
