use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::types::{KeyMode, ProviderType, ProxyToken};

pub fn generate_token() -> (String, String, String) {
    let raw_token = format!("adi_ek_{}", Uuid::new_v4().to_string().replace("-", ""));
    let prefix = format!("{}...", &raw_token[..12]);
    let hash = hex::encode(Sha256::digest(raw_token.as_bytes()));
    (raw_token, prefix, hash)
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[allow(clippy::too_many_arguments)]
pub async fn create_proxy_token(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    token_hash: &str,
    token_prefix: &str,
    key_mode: KeyMode,
    upstream_key_id: Option<Uuid>,
    platform_provider: Option<ProviderType>,
    allowed_models: Option<&[String]>,
    blocked_models: Option<&[String]>,
    log_requests: bool,
    log_responses: bool,
    expires_at: Option<DateTime<Utc>>,
) -> ApiResult<ProxyToken> {
    let row = sqlx::query(
        r#"
        INSERT INTO embed_proxy_tokens (
            user_id, name, token_hash, token_prefix,
            key_mode, upstream_key_id, platform_provider,
            allowed_models, blocked_models,
            log_requests, log_responses, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(token_hash)
    .bind(token_prefix)
    .bind(key_mode.to_string())
    .bind(upstream_key_id)
    .bind(platform_provider.map(|p| p.to_string()))
    .bind(allowed_models)
    .bind(blocked_models)
    .bind(log_requests)
    .bind(log_responses)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err)
            if db_err.constraint() == Some("embed_proxy_tokens_user_id_name_key") =>
        {
            ApiError::Conflict(format!("Token with name '{}' already exists", name))
        }
        _ => ApiError::Database(e),
    })?;

    Ok(row_to_proxy_token(&row))
}

pub async fn get_proxy_token_by_hash(pool: &PgPool, token_hash: &str) -> ApiResult<ProxyToken> {
    let row = sqlx::query("SELECT * FROM embed_proxy_tokens WHERE token_hash = $1")
        .bind(token_hash)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    Ok(row_to_proxy_token(&row))
}

pub async fn get_proxy_token(pool: &PgPool, id: Uuid, user_id: Uuid) -> ApiResult<ProxyToken> {
    let row =
        sqlx::query("SELECT * FROM embed_proxy_tokens WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| ApiError::NotFound("Proxy token not found".to_string()))?;

    Ok(row_to_proxy_token(&row))
}

pub async fn list_proxy_tokens(pool: &PgPool, user_id: Uuid) -> ApiResult<Vec<ProxyToken>> {
    let rows = sqlx::query(
        "SELECT * FROM embed_proxy_tokens WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_proxy_token).collect())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_proxy_token(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    name: Option<&str>,
    allowed_models: Option<Option<&[String]>>,
    blocked_models: Option<Option<&[String]>>,
    log_requests: Option<bool>,
    log_responses: Option<bool>,
    is_active: Option<bool>,
    expires_at: Option<Option<DateTime<Utc>>>,
) -> ApiResult<ProxyToken> {
    let row = sqlx::query(
        r#"
        UPDATE embed_proxy_tokens
        SET
            name = COALESCE($3, name),
            allowed_models = CASE WHEN $4::boolean THEN $5 ELSE allowed_models END,
            blocked_models = CASE WHEN $6::boolean THEN $7 ELSE blocked_models END,
            log_requests = COALESCE($8, log_requests),
            log_responses = COALESCE($9, log_responses),
            is_active = COALESCE($10, is_active),
            expires_at = CASE WHEN $11::boolean THEN $12 ELSE expires_at END,
            updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(allowed_models.is_some())
    .bind(allowed_models.flatten())
    .bind(blocked_models.is_some())
    .bind(blocked_models.flatten())
    .bind(log_requests)
    .bind(log_responses)
    .bind(is_active)
    .bind(expires_at.is_some())
    .bind(expires_at.flatten())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Proxy token not found".to_string()))?;

    Ok(row_to_proxy_token(&row))
}

pub async fn rotate_proxy_token(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> ApiResult<(ProxyToken, String)> {
    let (raw_token, prefix, hash) = generate_token();

    let row = sqlx::query(
        r#"
        UPDATE embed_proxy_tokens
        SET token_hash = $3, token_prefix = $4, updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(&hash)
    .bind(&prefix)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Proxy token not found".to_string()))?;

    Ok((row_to_proxy_token(&row), raw_token))
}

pub async fn delete_proxy_token(pool: &PgPool, id: Uuid, user_id: Uuid) -> ApiResult<()> {
    let result =
        sqlx::query("DELETE FROM embed_proxy_tokens WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Proxy token not found".to_string()));
    }

    Ok(())
}

fn row_to_proxy_token(row: &sqlx::postgres::PgRow) -> ProxyToken {
    let key_mode_str: String = row.get("key_mode");
    let platform_provider_str: Option<String> = row.get("platform_provider");

    ProxyToken {
        id: row.get("id"),
        user_id: row.get("user_id"),
        name: row.get("name"),
        token_hash: row.get("token_hash"),
        token_prefix: row.get("token_prefix"),
        key_mode: match key_mode_str.as_str() {
            "byok" => KeyMode::Byok,
            _ => KeyMode::Platform,
        },
        upstream_key_id: row.get("upstream_key_id"),
        platform_provider: platform_provider_str.and_then(|s| s.parse().ok()),
        allowed_models: row.get("allowed_models"),
        blocked_models: row.get("blocked_models"),
        log_requests: row.get("log_requests"),
        log_responses: row.get("log_responses"),
        is_active: row.get("is_active"),
        expires_at: row.get("expires_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
