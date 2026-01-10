use adi_analytics_api_core::TimeRangeParams;
use axum::{extract::{Query, State}, http::StatusCode, Json};
use sqlx::PgPool;

pub async fn get_daily_active_users(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<adi_analytics_api_core::DailyActiveUsers>>, StatusCode> {
    adi_analytics_api_core::get_daily_active_users(&pool, params.start_date, params.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch daily active users: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn get_weekly_active_users(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<adi_analytics_api_core::DailyActiveUsers>>, StatusCode> {
    adi_analytics_api_core::get_weekly_active_users(&pool, params.start_date, params.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch weekly active users: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
