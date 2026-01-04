use crate::models::{TaskStats, TaskStatsOverview, TimeRangeParams};
use axum::{extract::{Query, State}, http::StatusCode, Json};
use sqlx::{PgPool, FromRow};
use chrono::NaiveDate;

#[derive(FromRow)]
struct TaskStatsRow {
    day: NaiveDate,
    created: i64,
    started: i64,
    completed: i64,
    failed: i64,
    cancelled: i64,
    avg_duration_ms: Option<f64>,
    p95_duration_ms: Option<f64>,
}

#[derive(FromRow)]
struct TaskStatsOverviewRow {
    total_created: Option<i64>,
    total_completed: Option<i64>,
    total_failed: Option<i64>,
    total_cancelled: Option<i64>,
    avg_duration_ms: Option<f64>,
}

/// Get task statistics by day
pub async fn get_task_stats_daily(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<TaskStats>>, StatusCode> {
    let rows = sqlx::query_as::<_, TaskStatsRow>(
        r#"
        SELECT
            day,
            created,
            started,
            completed,
            failed,
            cancelled,
            avg_duration_ms,
            p95_duration_ms
        FROM analytics_task_stats_daily
        WHERE day >= $1 AND day <= $2
        ORDER BY day DESC
        "#,
    )
    .bind(params.start_date)
    .bind(params.end_date)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch task stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let stats = rows
        .into_iter()
        .map(|row| {
            let total_finished = row.completed + row.failed;
            let success_rate = if total_finished > 0 {
                (row.completed as f64) / (total_finished as f64)
            } else {
                0.0
            };

            TaskStats {
                day: row.day.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                created: row.created,
                started: row.started,
                completed: row.completed,
                failed: row.failed,
                cancelled: row.cancelled,
                avg_duration_ms: row.avg_duration_ms,
                p95_duration_ms: row.p95_duration_ms,
                success_rate,
            }
        })
        .collect();

    Ok(Json(stats))
}

/// Get task statistics overview (summary)
pub async fn get_task_stats_overview(
    Query(params): Query<TimeRangeParams>,
    State(pool): State<PgPool>,
) -> Result<Json<TaskStatsOverview>, StatusCode> {
    let row = sqlx::query_as::<_, TaskStatsOverviewRow>(
        r#"
        SELECT
            SUM(created)::BIGINT as total_created,
            SUM(completed)::BIGINT as total_completed,
            SUM(failed)::BIGINT as total_failed,
            SUM(cancelled)::BIGINT as total_cancelled,
            AVG(avg_duration_ms) as avg_duration_ms
        FROM analytics_task_stats_daily
        WHERE day >= $1 AND day <= $2
        "#,
    )
    .bind(params.start_date)
    .bind(params.end_date)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch task stats overview: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total_created = row.total_created.unwrap_or(0);
    let total_completed = row.total_completed.unwrap_or(0);
    let total_failed = row.total_failed.unwrap_or(0);
    let total_cancelled = row.total_cancelled.unwrap_or(0);

    let total_finished = total_completed + total_failed;
    let success_rate = if total_finished > 0 {
        (total_completed as f64) / (total_finished as f64)
    } else {
        0.0
    };

    Ok(Json(TaskStatsOverview {
        total_created,
        total_completed,
        total_failed,
        total_cancelled,
        success_rate,
        avg_duration_ms: row.avg_duration_ms.unwrap_or(0.0),
    }))
}
