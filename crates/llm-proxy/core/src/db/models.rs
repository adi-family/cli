//! Database operations for platform allowed models.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::types::{PlatformAllowedModel, ProviderType};

/// List allowed models for a provider.
pub async fn list_allowed_models(
    pool: &PgPool,
    provider_type: ProviderType,
) -> ApiResult<Vec<PlatformAllowedModel>> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider_type, model_id, display_name, is_active, created_at
        FROM platform_allowed_models
        WHERE provider_type = $1 AND is_active = true
        ORDER BY model_id
        "#,
    )
    .bind(provider_type.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_allowed_model).collect())
}

/// List all allowed models.
pub async fn list_all_allowed_models(pool: &PgPool) -> ApiResult<Vec<PlatformAllowedModel>> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider_type, model_id, display_name, is_active, created_at
        FROM platform_allowed_models
        WHERE is_active = true
        ORDER BY provider_type, model_id
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_allowed_model).collect())
}

/// Add an allowed model.
pub async fn add_allowed_model(
    pool: &PgPool,
    provider_type: ProviderType,
    model_id: &str,
    display_name: Option<&str>,
) -> ApiResult<PlatformAllowedModel> {
    let row = sqlx::query(
        r#"
        INSERT INTO platform_allowed_models (provider_type, model_id, display_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (provider_type, model_id) DO UPDATE SET
            display_name = COALESCE(EXCLUDED.display_name, platform_allowed_models.display_name),
            is_active = true
        RETURNING id, provider_type, model_id, display_name, is_active, created_at
        "#,
    )
    .bind(provider_type.to_string())
    .bind(model_id)
    .bind(display_name)
    .fetch_one(pool)
    .await?;

    Ok(row_to_allowed_model(&row))
}

/// Remove an allowed model.
pub async fn remove_allowed_model(
    pool: &PgPool,
    provider_type: ProviderType,
    model_id: &str,
) -> ApiResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE platform_allowed_models
        SET is_active = false
        WHERE provider_type = $1 AND model_id = $2
        "#,
    )
    .bind(provider_type.to_string())
    .bind(model_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Model not found".to_string()));
    }

    Ok(())
}

/// Check if a model is allowed.
pub async fn is_model_allowed(
    pool: &PgPool,
    provider_type: ProviderType,
    model_id: &str,
) -> ApiResult<bool> {
    let row = sqlx::query(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM platform_allowed_models
            WHERE provider_type = $1 AND model_id = $2 AND is_active = true
        ) as exists
        "#,
    )
    .bind(provider_type.to_string())
    .bind(model_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<bool, _>("exists"))
}

/// Delete an allowed model permanently.
pub async fn delete_allowed_model(pool: &PgPool, id: Uuid) -> ApiResult<()> {
    let result = sqlx::query("DELETE FROM platform_allowed_models WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Model not found".to_string()));
    }

    Ok(())
}

fn row_to_allowed_model(row: &sqlx::postgres::PgRow) -> PlatformAllowedModel {
    let provider_str: String = row.get("provider_type");
    PlatformAllowedModel {
        id: row.get("id"),
        provider_type: provider_str.parse().unwrap_or(ProviderType::Custom),
        model_id: row.get("model_id"),
        display_name: row.get("display_name"),
        is_active: row.get("is_active"),
        created_at: row.get("created_at"),
    }
}
