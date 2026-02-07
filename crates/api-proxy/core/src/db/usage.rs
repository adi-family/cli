//! Database operations for usage logging.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::ApiResult;
use crate::types::{KeyMode, ProviderType, ProxyUsageLog, RequestStatus};

/// Log a proxy request.
#[allow(clippy::too_many_arguments)]
pub async fn log_usage(
    pool: &PgPool,
    proxy_token_id: Uuid,
    user_id: Uuid,
    request_id: &str,
    upstream_request_id: Option<&str>,
    requested_model: Option<&str>,
    actual_model: Option<&str>,
    provider_type: ProviderType,
    key_mode: KeyMode,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    total_tokens: Option<i32>,
    reported_cost_usd: Option<Decimal>,
    endpoint: &str,
    is_streaming: bool,
    latency_ms: Option<i32>,
    ttft_ms: Option<i32>,
    status: RequestStatus,
    status_code: Option<i16>,
    error_type: Option<&str>,
    error_message: Option<&str>,
    request_body: Option<&serde_json::Value>,
    response_body: Option<&serde_json::Value>,
) -> ApiResult<ProxyUsageLog> {
    let row = sqlx::query(
        r#"
        INSERT INTO proxy_usage_log (
            proxy_token_id, user_id, request_id, upstream_request_id,
            requested_model, actual_model, provider_type, key_mode,
            input_tokens, output_tokens, total_tokens, reported_cost_usd,
            endpoint, is_streaming, latency_ms, ttft_ms,
            status, status_code, error_type, error_message,
            request_body, response_body
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22
        )
        RETURNING *
        "#,
    )
    .bind(proxy_token_id)
    .bind(user_id)
    .bind(request_id)
    .bind(upstream_request_id)
    .bind(requested_model)
    .bind(actual_model)
    .bind(provider_type.to_string())
    .bind(key_mode.to_string())
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(total_tokens)
    .bind(reported_cost_usd)
    .bind(endpoint)
    .bind(is_streaming)
    .bind(latency_ms)
    .bind(ttft_ms)
    .bind(status.to_string())
    .bind(status_code)
    .bind(error_type)
    .bind(error_message)
    .bind(request_body)
    .bind(response_body)
    .fetch_one(pool)
    .await?;

    Ok(row_to_usage_log(&row))
}

/// Query usage logs for a user.
pub async fn query_usage(
    pool: &PgPool,
    user_id: Uuid,
    proxy_token_id: Option<Uuid>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: i64,
    offset: i64,
) -> ApiResult<Vec<ProxyUsageLog>> {
    let rows = sqlx::query(
        r#"
        SELECT *
        FROM proxy_usage_log
        WHERE user_id = $1
            AND ($2::uuid IS NULL OR proxy_token_id = $2)
            AND ($3::timestamptz IS NULL OR created_at >= $3)
            AND ($4::timestamptz IS NULL OR created_at <= $4)
        ORDER BY created_at DESC
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(user_id)
    .bind(proxy_token_id)
    .bind(from)
    .bind(to)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_usage_log).collect())
}

/// Get usage summary for a user.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UsageSummary {
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: Option<Decimal>,
    pub success_count: i64,
    pub error_count: i64,
}

pub async fn get_usage_summary(
    pool: &PgPool,
    user_id: Uuid,
    proxy_token_id: Option<Uuid>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> ApiResult<UsageSummary> {
    let row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_requests,
            COALESCE(SUM(input_tokens), 0)::bigint as total_input_tokens,
            COALESCE(SUM(output_tokens), 0)::bigint as total_output_tokens,
            SUM(reported_cost_usd) as total_cost_usd,
            COUNT(*) FILTER (WHERE status = 'success') as success_count,
            COUNT(*) FILTER (WHERE status != 'success') as error_count
        FROM proxy_usage_log
        WHERE user_id = $1
            AND ($2::uuid IS NULL OR proxy_token_id = $2)
            AND ($3::timestamptz IS NULL OR created_at >= $3)
            AND ($4::timestamptz IS NULL OR created_at <= $4)
        "#,
    )
    .bind(user_id)
    .bind(proxy_token_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    Ok(UsageSummary {
        total_requests: row.get("total_requests"),
        total_input_tokens: row.get("total_input_tokens"),
        total_output_tokens: row.get("total_output_tokens"),
        total_cost_usd: row.get("total_cost_usd"),
        success_count: row.get("success_count"),
        error_count: row.get("error_count"),
    })
}

fn row_to_usage_log(row: &sqlx::postgres::PgRow) -> ProxyUsageLog {
    let provider_str: String = row.get("provider_type");
    let key_mode_str: String = row.get("key_mode");
    let status_str: String = row.get("status");

    ProxyUsageLog {
        id: row.get("id"),
        proxy_token_id: row.get("proxy_token_id"),
        user_id: row.get("user_id"),
        request_id: row.get("request_id"),
        upstream_request_id: row.get("upstream_request_id"),
        requested_model: row.get("requested_model"),
        actual_model: row.get("actual_model"),
        provider_type: provider_str.parse().unwrap_or(ProviderType::Custom),
        key_mode: match key_mode_str.as_str() {
            "byok" => KeyMode::Byok,
            _ => KeyMode::Platform,
        },
        input_tokens: row.get("input_tokens"),
        output_tokens: row.get("output_tokens"),
        total_tokens: row.get("total_tokens"),
        reported_cost_usd: row.get("reported_cost_usd"),
        endpoint: row.get("endpoint"),
        is_streaming: row.get("is_streaming"),
        latency_ms: row.get("latency_ms"),
        ttft_ms: row.get("ttft_ms"),
        status: match status_str.as_str() {
            "success" => RequestStatus::Success,
            "error" => RequestStatus::Error,
            _ => RequestStatus::UpstreamError,
        },
        status_code: row.get("status_code"),
        error_type: row.get("error_type"),
        error_message: row.get("error_message"),
        request_body: row.get("request_body"),
        response_body: row.get("response_body"),
        created_at: row.get("created_at"),
    }
}
