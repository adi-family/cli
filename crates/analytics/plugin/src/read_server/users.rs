use analytics_client::TimeRangeParams;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use sqlx::PgPool;

pub async fn get_daily_active_users(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<analytics_client::DailyActiveUsers>>, StatusCode> {
    analytics_client::get_daily_active_users(&pool, params.start_date, params.end_date)
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
) -> Result<Json<Vec<analytics_client::DailyActiveUsers>>, StatusCode> {
    analytics_client::get_weekly_active_users(&pool, params.start_date, params.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to fetch weekly active users: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
