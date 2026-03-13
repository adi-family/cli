//! Database operations for platform provider keys.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::types::{PlatformProviderKey, ProviderType};

/// Get a platform provider key by type.
pub async fn get_platform_key(
    pool: &PgPool,
    provider_type: ProviderType,
) -> ApiResult<PlatformProviderKey> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_type, api_key_encrypted, base_url, is_active, created_at, updated_at
        FROM platform_provider_keys
        WHERE provider_type = $1 AND is_active = true
        "#,
    )
    .bind(provider_type.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ApiError::NotFound(format!("Platform key for {} not configured", provider_type))
    })?;

    Ok(row_to_platform_key(&row))
}

/// List all platform provider keys.
pub async fn list_platform_keys(pool: &PgPool) -> ApiResult<Vec<PlatformProviderKey>> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider_type, api_key_encrypted, base_url, is_active, created_at, updated_at
        FROM platform_provider_keys
        ORDER BY provider_type
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_platform_key).collect())
}

/// Create or update a platform provider key.
pub async fn upsert_platform_key(
    pool: &PgPool,
    provider_type: ProviderType,
    api_key_encrypted: &str,
    base_url: Option<&str>,
) -> ApiResult<PlatformProviderKey> {
    let row = sqlx::query(
        r#"
        INSERT INTO platform_provider_keys (provider_type, api_key_encrypted, base_url)
        VALUES ($1, $2, $3)
        ON CONFLICT (provider_type) DO UPDATE SET
            api_key_encrypted = EXCLUDED.api_key_encrypted,
            base_url = EXCLUDED.base_url,
            updated_at = NOW()
        RETURNING id, provider_type, api_key_encrypted, base_url, is_active, created_at, updated_at
        "#,
    )
    .bind(provider_type.to_string())
    .bind(api_key_encrypted)
    .bind(base_url)
    .fetch_one(pool)
    .await?;

    Ok(row_to_platform_key(&row))
}

/// Update platform key active status.
pub async fn set_platform_key_active(
    pool: &PgPool,
    id: Uuid,
    is_active: bool,
) -> ApiResult<PlatformProviderKey> {
    let row = sqlx::query(
        r#"
        UPDATE platform_provider_keys
        SET is_active = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, provider_type, api_key_encrypted, base_url, is_active, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(is_active)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("Platform key not found".to_string()))?;

    Ok(row_to_platform_key(&row))
}

/// Delete a platform provider key.
pub async fn delete_platform_key(pool: &PgPool, id: Uuid) -> ApiResult<()> {
    let result = sqlx::query("DELETE FROM platform_provider_keys WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Platform key not found".to_string()));
    }

    Ok(())
}

fn row_to_platform_key(row: &sqlx::postgres::PgRow) -> PlatformProviderKey {
    let provider_str: String = row.get("provider_type");
    PlatformProviderKey {
        id: row.get("id"),
        provider_type: provider_str.parse().unwrap_or(ProviderType::Custom),
        api_key_encrypted: row.get("api_key_encrypted"),
        base_url: row.get("base_url"),
        is_active: row.get("is_active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
