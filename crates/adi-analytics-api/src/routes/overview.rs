use crate::models::OverviewStats;
use axum::{extract::State, http::StatusCode, Json};
use sqlx::{PgPool, FromRow};

#[derive(FromRow)]
struct TaskStatsRow {
    total_tasks: Option<i64>,
    tasks_today: Option<i64>,
    total_completed: Option<i64>,
    total_failed: Option<i64>,
}

/// Get overview statistics (dashboard summary)
pub async fn get_overview(
    State(pool): State<PgPool>,
) -> Result<Json<OverviewStats>, StatusCode> {
    // Get unique users (all time)
    let total_users: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id)
        FROM analytics_events
        WHERE user_id IS NOT NULL
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch total users: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Active users (today, week, month)
    let active_users_today: i64 = sqlx::query_scalar(
        r#"
        SELECT active_users
        FROM analytics_daily_active_users
        WHERE day = CURRENT_DATE
        "#
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch active users today: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .unwrap_or(0);

    let active_users_week: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id)
        FROM analytics_events
        WHERE timestamp >= NOW() - INTERVAL '7 days'
          AND user_id IS NOT NULL
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch active users week: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let active_users_month: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id)
        FROM analytics_events
        WHERE timestamp >= NOW() - INTERVAL '30 days'
          AND user_id IS NOT NULL
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch active users month: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Task stats
    let task_row = sqlx::query_as::<_, TaskStatsRow>(
        r#"
        SELECT
            SUM(created)::BIGINT as total_tasks,
            SUM(created) FILTER (WHERE day = CURRENT_DATE)::BIGINT as tasks_today,
            SUM(completed)::BIGINT as total_completed,
            SUM(failed)::BIGINT as total_failed
        FROM analytics_task_stats_daily
        "#,
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch task stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total_tasks = task_row.total_tasks.unwrap_or(0);
    let tasks_today = task_row.tasks_today.unwrap_or(0);
    let total_completed = task_row.total_completed.unwrap_or(0);
    let total_failed = task_row.total_failed.unwrap_or(0);

    let total_finished = total_completed + total_failed;
    let task_success_rate = if total_finished > 0 {
        (total_completed as f64) / (total_finished as f64)
    } else {
        0.0
    };

    // Cocoon stats
    let total_cocoons: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT data->>'cocoon_id')
        FROM analytics_events
        WHERE event_type IN ('cocoon_registered', 'cocoon_connected')
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch total cocoons: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let active_cocoons: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT data->>'cocoon_id')
        FROM analytics_events
        WHERE event_type = 'cocoon_connected'
          AND timestamp >= NOW() - INTERVAL '24 hours'
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch active cocoons: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Integration stats
    let total_integrations: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT data->>'integration_id')
        FROM analytics_events
        WHERE event_type = 'integration_connected'
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch total integrations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(OverviewStats {
        total_users,
        active_users_today,
        active_users_week,
        active_users_month,
        total_tasks,
        tasks_today,
        task_success_rate,
        total_cocoons,
        active_cocoons,
        total_integrations,
    }))
}
