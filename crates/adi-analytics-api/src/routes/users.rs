use crate::models::{DailyActiveUsers, TimeRangeParams};
use axum::{extract::{Query, State}, http::StatusCode, Json};
use sqlx::PgPool;

/// Get daily active users
pub async fn get_daily_active_users(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<DailyActiveUsers>>, StatusCode> {
    let rows = sqlx::query_as::<_, DailyActiveUsers>(
        r#"
        SELECT day, active_users, total_events
        FROM analytics_daily_active_users
        WHERE day >= $1 AND day <= $2
        ORDER BY day DESC
        "#,
    )
    .bind(params.start_date)
    .bind(params.end_date)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch daily active users: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(rows))
}

/// Get weekly active users
pub async fn get_weekly_active_users(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<DailyActiveUsers>>, StatusCode> {
    let rows = sqlx::query_as::<_, DailyActiveUsers>(
        r#"
        SELECT
            time_bucket('7 days', day) as day,
            SUM(active_users)::BIGINT as active_users,
            SUM(total_events)::BIGINT as total_events
        FROM analytics_daily_active_users
        WHERE day >= $1 AND day <= $2
        GROUP BY time_bucket('7 days', day)
        ORDER BY day DESC
        "#,
    )
    .bind(params.start_date)
    .bind(params.end_date)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch weekly active users: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(rows))
}
