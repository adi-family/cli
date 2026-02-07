//! Usage query routes.

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{middleware::AuthUser, state::AppState};
use api_proxy_core::{db, ApiResult, ProxyUsageLog};

/// Query parameters for usage listing.
#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    pub proxy_token_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// Response for usage listing.
#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub logs: Vec<ProxyUsageLog>,
    pub total: i64,
}

/// Query usage logs.
pub async fn query_usage(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<UsageQuery>,
) -> ApiResult<Json<UsageResponse>> {
    let logs = db::usage::query_usage(
        state.db.pool(),
        user.id,
        query.proxy_token_id,
        query.from,
        query.to,
        query.limit.min(1000),
        query.offset,
    )
    .await?;

    let total = logs.len() as i64; // TODO: Add count query

    Ok(Json(UsageResponse { logs, total }))
}

/// Get usage summary.
pub async fn usage_summary(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<UsageQuery>,
) -> ApiResult<Json<db::usage::UsageSummary>> {
    let summary = db::usage::get_usage_summary(
        state.db.pool(),
        user.id,
        query.proxy_token_id,
        query.from,
        query.to,
    )
    .await?;

    Ok(Json(summary))
}

/// Query parameters for usage export.
#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub proxy_token_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// Export usage logs as CSV or JSON.
pub async fn export_usage(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ExportQuery>,
) -> ApiResult<(HeaderMap, String)> {
    let logs = db::usage::query_usage(
        state.db.pool(),
        user.id,
        query.proxy_token_id,
        query.from,
        query.to,
        10000, // Max export limit
        0,
    )
    .await?;

    let mut headers = HeaderMap::new();

    let body = match query.format.as_str() {
        "csv" => {
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/csv; charset=utf-8"),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"usage.csv\""),
            );
            export_csv(&logs)
        }
        _ => {
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"usage.json\""),
            );
            serde_json::to_string_pretty(&logs).unwrap_or_default()
        }
    };

    Ok((headers, body))
}

fn export_csv(logs: &[ProxyUsageLog]) -> String {
    let mut csv = String::new();

    // Header
    csv.push_str("id,created_at,proxy_token_id,request_id,provider_type,key_mode,");
    csv.push_str("requested_model,actual_model,endpoint,is_streaming,");
    csv.push_str("input_tokens,output_tokens,total_tokens,reported_cost_usd,");
    csv.push_str("latency_ms,ttft_ms,status,status_code,error_type,error_message\n");

    for log in logs {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            log.id,
            log.created_at,
            log.proxy_token_id,
            log.request_id,
            log.provider_type,
            log.key_mode,
            log.requested_model.as_deref().unwrap_or(""),
            log.actual_model.as_deref().unwrap_or(""),
            log.endpoint,
            log.is_streaming,
            log.input_tokens.map(|t| t.to_string()).unwrap_or_default(),
            log.output_tokens.map(|t| t.to_string()).unwrap_or_default(),
            log.total_tokens.map(|t| t.to_string()).unwrap_or_default(),
            log.reported_cost_usd
                .map(|c| c.to_string())
                .unwrap_or_default(),
            log.latency_ms.map(|t| t.to_string()).unwrap_or_default(),
            log.ttft_ms.map(|t| t.to_string()).unwrap_or_default(),
            log.status,
            log.status_code.map(|c| c.to_string()).unwrap_or_default(),
            escape_csv(log.error_type.as_deref().unwrap_or("")),
            escape_csv(log.error_message.as_deref().unwrap_or("")),
        ));
    }

    csv
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
