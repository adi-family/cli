//! Log reader - queries logs from TimescaleDB.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Query parameters for log search.
#[derive(Debug, Default, Deserialize)]
pub struct LogQueryParams {
    pub service: Option<String>,
    pub level: Option<String>,
    pub trace_id: Option<Uuid>,
    pub cocoon_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub hive_id: Option<String>,
    pub search: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// A log record returned from queries.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LogRecord {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub service: String,
    pub hostname: Option<String>,
    pub environment: Option<String>,
    pub level: String,
    pub message: String,
    pub trace_id: Uuid,
    pub span_id: Uuid,
    pub parent_span_id: Option<Uuid>,
    pub cocoon_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub hive_id: Option<String>,
    pub fields: Option<serde_json::Value>,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub source: Option<String>,
    pub target: Option<String>,
}

/// Aggregate statistics for a time period.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LogStats {
    pub total_logs: i64,
    pub services_count: i64,
    pub traces_count: i64,
    pub error_count: i64,
    pub warn_count: i64,
}

/// Per-service statistics.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ServiceLogStats {
    pub service: String,
    pub log_count: i64,
    pub error_count: i64,
}

/// Pagination parameters for correlation queries.
#[derive(Debug, Default)]
pub struct PaginationParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn parse_level_filter(level: &str) -> Option<i16> {
    match level.to_lowercase().as_str() {
        "trace" => Some(0),
        "debug" => Some(1),
        "info" => Some(2),
        "notice" => Some(3),
        "warn" | "warning" => Some(4),
        "error" => Some(5),
        "fatal" | "critical" => Some(6),
        _ => None,
    }
}

/// Reads logs from the database.
#[derive(Clone)]
pub struct LogReader {
    pool: PgPool,
}

impl LogReader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Query logs with filters.
    pub async fn query(&self, params: &LogQueryParams) -> Result<Vec<LogRecord>, sqlx::Error> {
        let limit = params.limit.unwrap_or(100).min(1000);
        let offset = params.offset.unwrap_or(0);
        let level_filter = params.level.as_ref().and_then(|l| parse_level_filter(l));
        let from = params
            .from
            .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
        let to = params.to.unwrap_or_else(Utc::now);

        sqlx::query_as::<_, LogRecord>(
            r#"
            SELECT
                id, timestamp, service, hostname, environment,
                level_name as level, message,
                trace_id, span_id, parent_span_id,
                cocoon_id, user_id, session_id, hive_id,
                fields, error_kind, error_message, source, target
            FROM logs
            WHERE timestamp >= $1 AND timestamp <= $2
                AND ($3::varchar IS NULL OR service = $3)
                AND ($4::smallint IS NULL OR level >= $4)
                AND ($5::uuid IS NULL OR trace_id = $5)
                AND ($6::varchar IS NULL OR cocoon_id = $6)
                AND ($7::varchar IS NULL OR user_id = $7)
                AND ($8::varchar IS NULL OR session_id = $8)
                AND ($9::varchar IS NULL OR hive_id = $9)
                AND ($10::text IS NULL OR message ILIKE '%' || $10 || '%')
            ORDER BY timestamp DESC
            LIMIT $11 OFFSET $12
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(&params.service)
        .bind(level_filter)
        .bind(params.trace_id)
        .bind(&params.cocoon_id)
        .bind(&params.user_id)
        .bind(&params.session_id)
        .bind(&params.hive_id)
        .bind(&params.search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Get all logs for a trace.
    pub async fn trace_logs(&self, trace_id: Uuid) -> Result<Vec<LogRecord>, sqlx::Error> {
        sqlx::query_as::<_, LogRecord>(
            r#"
            SELECT
                id, timestamp, service, hostname, environment,
                level_name as level, message,
                trace_id, span_id, parent_span_id,
                cocoon_id, user_id, session_id, hive_id,
                fields, error_kind, error_message, source, target
            FROM logs
            WHERE trace_id = $1
            ORDER BY timestamp ASC
            LIMIT 1000
            "#,
        )
        .bind(trace_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get logs for a span.
    pub async fn span_logs(&self, span_id: Uuid) -> Result<Vec<LogRecord>, sqlx::Error> {
        sqlx::query_as::<_, LogRecord>(
            r#"
            SELECT
                id, timestamp, service, hostname, environment,
                level_name as level, message,
                trace_id, span_id, parent_span_id,
                cocoon_id, user_id, session_id, hive_id,
                fields, error_kind, error_message, source, target
            FROM logs
            WHERE span_id = $1
            ORDER BY timestamp ASC
            LIMIT 1000
            "#,
        )
        .bind(span_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get logs by correlation ID (cocoon, user, or session).
    pub async fn correlation_logs(
        &self,
        field: &str,
        value: &str,
        page: &PaginationParams,
    ) -> Result<Vec<LogRecord>, sqlx::Error> {
        let limit = page.limit.unwrap_or(100).min(1000);
        let offset = page.offset.unwrap_or(0);
        let from = page
            .from
            .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
        let to = page.to.unwrap_or_else(Utc::now);

        let query = format!(
            r#"
            SELECT
                id, timestamp, service, hostname, environment,
                level_name as level, message,
                trace_id, span_id, parent_span_id,
                cocoon_id, user_id, session_id, hive_id,
                fields, error_kind, error_message, source, target
            FROM logs
            WHERE {} = $1
                AND timestamp >= $2 AND timestamp <= $3
            ORDER BY timestamp DESC
            LIMIT $4 OFFSET $5
            "#,
            field
        );

        sqlx::query_as::<_, LogRecord>(&query)
            .bind(value)
            .bind(from)
            .bind(to)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
    }

    /// Get aggregate statistics for the last 24 hours.
    pub async fn stats(&self) -> Result<LogStats, sqlx::Error> {
        sqlx::query_as::<_, LogStats>(
            r#"
            SELECT
                COUNT(*) as total_logs,
                COUNT(DISTINCT service) as services_count,
                COUNT(DISTINCT trace_id) as traces_count,
                COUNT(*) FILTER (WHERE level >= 5) as error_count,
                COUNT(*) FILTER (WHERE level >= 4 AND level < 5) as warn_count
            FROM logs
            WHERE timestamp >= NOW() - INTERVAL '24 hours'
            "#,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Get per-service statistics for the last 24 hours.
    pub async fn service_stats(&self) -> Result<Vec<ServiceLogStats>, sqlx::Error> {
        sqlx::query_as::<_, ServiceLogStats>(
            r#"
            SELECT
                service,
                COUNT(*) as log_count,
                COUNT(*) FILTER (WHERE level >= 5) as error_count
            FROM logs
            WHERE timestamp >= NOW() - INTERVAL '24 hours'
            GROUP BY service
            ORDER BY log_count DESC
            LIMIT 20
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }
}
