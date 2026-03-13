//! Database operations for upstream API keys (BYOK).

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::types::{ProviderType, UpstreamApiKey};

/// Create a new upstream API key.
pub async fn create_upstream_key(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    provider_type: ProviderType,
    api_key_encrypted: &str,
    base_url: Option<&str>,
) -> ApiResult<UpstreamApiKey> {
    let row = sqlx::query(
        r#"
        INSERT INTO upstream_api_keys (user_id, name, provider_type, api_key_encrypted, base_url)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, name, provider_type, api_key_encrypted, base_url, is_active, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(provider_type.to_string())
    .bind(api_key_encrypted)
    .bind(base_url)
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err) if db_err.constraint() == Some("upstream_api_keys_user_id_name_key") => {
            ApiError::Conflict(format!("Key with name '{}' already exists", name))
        }
        _ => ApiError::Database(e),
    })?;

    Ok(row_to_upstream_key(&row))
}

/// Get an upstream API key by ID.
pub async fn get_upstream_key(pool: &PgPool, id: Uuid, user_id: Uuid) -> ApiResult<UpstreamApiKey> {
    let row = sqlx::query(
        r#"
        SELECT id, user_id, name, provider_type, api_key_encrypted, base_url, is_active, created_at, updated_at
        FROM upstream_api_keys
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Upstream key not found".to_string()))?;

    Ok(row_to_upstream_key(&row))
}

/// Get an upstream API key by ID (internal use, no user check).
pub async fn get_upstream_key_by_id(pool: &PgPool, id: Uuid) -> ApiResult<UpstreamApiKey> {
    let row = sqlx::query(
        r#"
        SELECT id, user_id, name, provider_type, api_key_encrypted, base_url, is_active, created_at, updated_at
        FROM upstream_api_keys
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Upstream key not found".to_string()))?;

    Ok(row_to_upstream_key(&row))
}

/// List upstream API keys for a user.
pub async fn list_upstream_keys(pool: &PgPool, user_id: Uuid) -> ApiResult<Vec<UpstreamApiKey>> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, name, provider_type, api_key_encrypted, base_url, is_active, created_at, updated_at
        FROM upstream_api_keys
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_upstream_key).collect())
}

/// Update an upstream API key.
pub async fn update_upstream_key(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    name: Option<&str>,
    api_key_encrypted: Option<&str>,
    base_url: Option<Option<&str>>,
    is_active: Option<bool>,
) -> ApiResult<UpstreamApiKey> {
    let row = sqlx::query(
        r#"
        UPDATE upstream_api_keys
        SET 
            name = COALESCE($3, name),
            api_key_encrypted = COALESCE($4, api_key_encrypted),
            base_url = CASE WHEN $5::boolean THEN $6 ELSE base_url END,
            is_active = COALESCE($7, is_active),
            updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING id, user_id, name, provider_type, api_key_encrypted, base_url, is_active, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(api_key_encrypted)
    .bind(base_url.is_some())
    .bind(base_url.flatten())
    .bind(is_active)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Upstream key not found".to_string()))?;

    Ok(row_to_upstream_key(&row))
}

/// Delete an upstream API key.
pub async fn delete_upstream_key(pool: &PgPool, id: Uuid, user_id: Uuid) -> ApiResult<()> {
    let result = sqlx::query("DELETE FROM upstream_api_keys WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Upstream key not found".to_string()));
    }

    Ok(())
}

fn row_to_upstream_key(row: &sqlx::postgres::PgRow) -> UpstreamApiKey {
    let provider_str: String = row.get("provider_type");
    UpstreamApiKey {
        id: row.get("id"),
        user_id: row.get("user_id"),
        name: row.get("name"),
        provider_type: provider_str.parse().unwrap_or(ProviderType::Custom),
        api_key_encrypted: row.get("api_key_encrypted"),
        base_url: row.get("base_url"),
        is_active: row.get("is_active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
