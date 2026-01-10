use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::models::*;

#[derive(Debug, thiserror::Error)]
pub enum AnalyticsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, AnalyticsError>;

// ===== User Queries =====

pub async fn get_daily_active_users(
    pool: &PgPool,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<DailyActiveUsers>> {
    let rows = sqlx::query_as::<_, DailyActiveUsers>(
        r#"
        SELECT day, active_users, total_events
        FROM analytics_daily_active_users
        WHERE day >= $1 AND day <= $2
        ORDER BY day DESC
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_weekly_active_users(
    pool: &PgPool,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<DailyActiveUsers>> {
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
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

// ===== Task Queries =====

pub async fn get_task_stats_daily(
    pool: &PgPool,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<TaskStats>> {
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
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await?;

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

    Ok(stats)
}

pub async fn get_task_stats_overview(
    pool: &PgPool,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<TaskStatsOverview> {
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
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await?;

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

    Ok(TaskStatsOverview {
        total_created,
        total_completed,
        total_failed,
        total_cancelled,
        success_rate,
        avg_duration_ms: row.avg_duration_ms.unwrap_or(0.0),
    })
}

// ===== API Performance Queries =====

pub async fn get_endpoint_latency(
    pool: &PgPool,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<EndpointLatency>> {
    let rows = sqlx::query_as::<_, EndpointLatencyRow>(
        r#"
        SELECT
            hour,
            service,
            endpoint,
            method,
            request_count,
            avg_duration_ms,
            p50_duration_ms,
            p95_duration_ms,
            p99_duration_ms,
            error_4xx_count,
            error_5xx_count
        FROM analytics_api_latency_hourly
        WHERE hour >= $1 AND hour <= $2
        ORDER BY hour DESC, p99_duration_ms DESC
        LIMIT 1000
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await?;

    let latencies = rows
        .into_iter()
        .map(|row| EndpointLatency {
            hour: row.hour,
            service: row.service,
            endpoint: row.endpoint.unwrap_or_default(),
            method: row.method.unwrap_or_default(),
            request_count: row.request_count,
            avg_duration_ms: row.avg_duration_ms.unwrap_or(0.0),
            p50_duration_ms: row.p50_duration_ms.unwrap_or(0.0),
            p95_duration_ms: row.p95_duration_ms.unwrap_or(0.0),
            p99_duration_ms: row.p99_duration_ms.unwrap_or(0.0),
            error_4xx_count: row.error_4xx_count.unwrap_or(0),
            error_5xx_count: row.error_5xx_count.unwrap_or(0),
        })
        .collect();

    Ok(latencies)
}

pub async fn get_slowest_endpoints(pool: &PgPool) -> Result<Vec<EndpointLatency>> {
    let rows = sqlx::query_as::<_, EndpointLatencyRow>(
        r#"
        SELECT
            hour,
            service,
            endpoint,
            method,
            request_count,
            avg_duration_ms,
            p50_duration_ms,
            p95_duration_ms,
            p99_duration_ms,
            error_4xx_count,
            error_5xx_count
        FROM analytics_api_latency_hourly
        WHERE hour >= NOW() - INTERVAL '24 hours'
        ORDER BY p99_duration_ms DESC
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await?;

    let latencies = rows
        .into_iter()
        .map(|row| EndpointLatency {
            hour: row.hour,
            service: row.service,
            endpoint: row.endpoint.unwrap_or_default(),
            method: row.method.unwrap_or_default(),
            request_count: row.request_count,
            avg_duration_ms: row.avg_duration_ms.unwrap_or(0.0),
            p50_duration_ms: row.p50_duration_ms.unwrap_or(0.0),
            p95_duration_ms: row.p95_duration_ms.unwrap_or(0.0),
            p99_duration_ms: row.p99_duration_ms.unwrap_or(0.0),
            error_4xx_count: row.error_4xx_count.unwrap_or(0),
            error_5xx_count: row.error_5xx_count.unwrap_or(0),
        })
        .collect();

    Ok(latencies)
}

// ===== Overview Queries =====

pub async fn get_overview_stats(pool: &PgPool) -> Result<OverviewStats> {
    // Get unique users (all time)
    let total_users: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id)
        FROM analytics_events
        WHERE user_id IS NOT NULL
        "#,
    )
    .fetch_one(pool)
    .await?;

    // Active users (today, week, month)
    let active_users_today: i64 = sqlx::query_scalar(
        r#"
        SELECT active_users
        FROM analytics_daily_active_users
        WHERE day = CURRENT_DATE
        "#,
    )
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);

    let active_users_week: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id)
        FROM analytics_events
        WHERE timestamp >= NOW() - INTERVAL '7 days'
          AND user_id IS NOT NULL
        "#,
    )
    .fetch_one(pool)
    .await?;

    let active_users_month: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id)
        FROM analytics_events
        WHERE timestamp >= NOW() - INTERVAL '30 days'
          AND user_id IS NOT NULL
        "#,
    )
    .fetch_one(pool)
    .await?;

    // Task stats
    let task_row = sqlx::query_as::<_, OverviewTaskStatsRow>(
        r#"
        SELECT
            SUM(created)::BIGINT as total_tasks,
            SUM(created) FILTER (WHERE day = CURRENT_DATE)::BIGINT as tasks_today,
            SUM(completed)::BIGINT as total_completed,
            SUM(failed)::BIGINT as total_failed
        FROM analytics_task_stats_daily
        "#,
    )
    .fetch_one(pool)
    .await?;

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
        "#,
    )
    .fetch_one(pool)
    .await?;

    let active_cocoons: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT data->>'cocoon_id')
        FROM analytics_events
        WHERE event_type = 'cocoon_connected'
          AND timestamp >= NOW() - INTERVAL '24 hours'
        "#,
    )
    .fetch_one(pool)
    .await?;

    // Integration stats
    let total_integrations: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT data->>'integration_id')
        FROM analytics_events
        WHERE event_type = 'integration_connected'
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(OverviewStats {
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
    })
}
