use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ===== Query Parameters =====

#[derive(Debug, Deserialize)]
pub struct TimeRangeParams {
    #[serde(default = "default_start_date")]
    pub start_date: DateTime<Utc>,

    #[serde(default = "default_end_date")]
    pub end_date: DateTime<Utc>,
}

fn default_start_date() -> DateTime<Utc> {
    Utc::now() - chrono::Duration::days(30)
}

fn default_end_date() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: i64,

    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

// ===== Response Models =====

#[derive(Debug, Serialize, FromRow)]
pub struct DailyActiveUsers {
    pub day: DateTime<Utc>,
    pub active_users: i64,
    pub total_events: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TaskStats {
    pub day: DateTime<Utc>,
    pub created: i64,
    pub started: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub avg_duration_ms: Option<f64>,
    pub p95_duration_ms: Option<f64>,
    pub success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct TaskStatsOverview {
    pub total_created: i64,
    pub total_completed: i64,
    pub total_failed: i64,
    pub total_cancelled: i64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct EndpointLatency {
    pub hour: DateTime<Utc>,
    pub service: String,
    pub endpoint: String,
    pub method: String,
    pub request_count: i64,
    pub avg_duration_ms: f64,
    pub p50_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
    pub error_4xx_count: i64,
    pub error_5xx_count: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct IntegrationHealth {
    pub day: DateTime<Utc>,
    pub provider: String,
    pub connections: i64,
    pub disconnections: i64,
    pub uses: i64,
    pub errors: i64,
    pub unique_users: i64,
    pub error_rate: f64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AuthStats {
    pub day: DateTime<Utc>,
    pub login_attempts: i64,
    pub successful_logins: i64,
    pub failed_logins: i64,
    pub code_verifications: i64,
    pub token_refreshes: i64,
    pub unique_users_authenticated: i64,
    pub login_success_rate: f64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CocoonActivity {
    pub day: DateTime<Utc>,
    pub registrations: i64,
    pub connections: i64,
    pub disconnections: i64,
    pub claims: i64,
    pub avg_session_duration_seconds: Option<f64>,
    pub unique_cocoons: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ErrorSummary {
    pub hour: DateTime<Utc>,
    pub service: String,
    pub error_type: String,
    pub error_count: i64,
    pub affected_users: i64,
    pub sample_error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OverviewStats {
    pub total_users: i64,
    pub active_users_today: i64,
    pub active_users_week: i64,
    pub active_users_month: i64,
    pub total_tasks: i64,
    pub tasks_today: i64,
    pub task_success_rate: f64,
    pub total_cocoons: i64,
    pub active_cocoons: i64,
    pub total_integrations: i64,
}

// ===== Internal Row Types for Queries =====

#[derive(FromRow)]
pub(crate) struct EndpointLatencyRow {
    pub hour: DateTime<Utc>,
    pub service: String,
    pub endpoint: Option<String>,
    pub method: Option<String>,
    pub request_count: i64,
    pub avg_duration_ms: Option<f64>,
    pub p50_duration_ms: Option<f64>,
    pub p95_duration_ms: Option<f64>,
    pub p99_duration_ms: Option<f64>,
    pub error_4xx_count: Option<i64>,
    pub error_5xx_count: Option<i64>,
}

#[derive(FromRow)]
pub(crate) struct TaskStatsRow {
    pub day: chrono::NaiveDate,
    pub created: i64,
    pub started: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub avg_duration_ms: Option<f64>,
    pub p95_duration_ms: Option<f64>,
}

#[derive(FromRow)]
pub(crate) struct TaskStatsOverviewRow {
    pub total_created: Option<i64>,
    pub total_completed: Option<i64>,
    pub total_failed: Option<i64>,
    pub total_cancelled: Option<i64>,
    pub avg_duration_ms: Option<f64>,
}

#[derive(FromRow)]
pub(crate) struct OverviewTaskStatsRow {
    pub total_tasks: Option<i64>,
    pub tasks_today: Option<i64>,
    pub total_completed: Option<i64>,
    pub total_failed: Option<i64>,
}
