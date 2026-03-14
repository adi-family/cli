use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::types::{PlatformAllowedModel, ProviderType};

pub async fn list_all_allowed_models(pool: &PgPool) -> ApiResult<Vec<PlatformAllowedModel>> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider_type, model_id, display_name, is_active, created_at
        FROM embed_platform_allowed_models
        WHERE is_active = true
        ORDER BY provider_type, model_id
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_allowed_model).collect())
}

pub async fn add_allowed_model(
    pool: &PgPool,
    provider_type: ProviderType,
    model_id: &str,
    display_name: Option<&str>,
) -> ApiResult<PlatformAllowedModel> {
    let row = sqlx::query(
        r#"
        INSERT INTO embed_platform_allowed_models (provider_type, model_id, display_name)
        VALUES ($1, $2, $3)
        ON CONFLICT (provider_type, model_id) DO UPDATE SET
            display_name = COALESCE(EXCLUDED.display_name, embed_platform_allowed_models.display_name),
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

pub async fn delete_allowed_model(pool: &PgPool, id: Uuid) -> ApiResult<()> {
    let result = sqlx::query("DELETE FROM embed_platform_allowed_models WHERE id = $1")
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
