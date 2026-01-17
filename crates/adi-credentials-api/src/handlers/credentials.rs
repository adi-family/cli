use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    AppState,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    models::{
        CreateCredential, Credential, CredentialAccessLog, CredentialRow, CredentialWithData,
        ListCredentialsQuery, UpdateCredential,
    },
};

/// List all credentials for the authenticated user
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListCredentialsQuery>,
) -> ApiResult<Json<Vec<Credential>>> {
    let rows = match (query.credential_type, query.provider) {
        (None, None) => {
            sqlx::query_as::<_, CredentialRow>(
                "SELECT * FROM credentials WHERE user_id = $1 ORDER BY created_at DESC",
            )
            .bind(user.id)
            .fetch_all(state.db.pool())
            .await?
        }
        (Some(cred_type), None) => {
            sqlx::query_as::<_, CredentialRow>(
                "SELECT * FROM credentials WHERE user_id = $1 AND credential_type = $2 ORDER BY created_at DESC",
            )
            .bind(user.id)
            .bind(cred_type)
            .fetch_all(state.db.pool())
            .await?
        }
        (None, Some(provider)) => {
            sqlx::query_as::<_, CredentialRow>(
                "SELECT * FROM credentials WHERE user_id = $1 AND provider = $2 ORDER BY created_at DESC",
            )
            .bind(user.id)
            .bind(provider)
            .fetch_all(state.db.pool())
            .await?
        }
        (Some(cred_type), Some(provider)) => {
            sqlx::query_as::<_, CredentialRow>(
                "SELECT * FROM credentials WHERE user_id = $1 AND credential_type = $2 AND provider = $3 ORDER BY created_at DESC",
            )
            .bind(user.id)
            .bind(cred_type)
            .bind(provider)
            .fetch_all(state.db.pool())
            .await?
        }
    };

    let credentials: Vec<Credential> = rows.into_iter().map(Into::into).collect();
    Ok(Json(credentials))
}

/// Get a credential by ID (without decrypted data)
pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Credential>> {
    let row = sqlx::query_as::<_, CredentialRow>(
        "SELECT * FROM credentials WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(row.into()))
}

/// Get a credential with decrypted data
pub async fn get_with_data(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CredentialWithData>> {
    let row = sqlx::query_as::<_, CredentialRow>(
        "SELECT * FROM credentials WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    // Decrypt the data
    let decrypted = state.secrets.decrypt(&row.encrypted_data)?;
    let data: serde_json::Value = serde_json::from_str(&decrypted)
        .map_err(|e| ApiError::Internal(format!("Failed to parse credential data: {}", e)))?;

    // Update last_used_at
    sqlx::query("UPDATE credentials SET last_used_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(state.db.pool())
        .await?;

    // Log access
    log_access(&state, id, user.id, "read", None).await?;

    let credential: Credential = row.into();
    Ok(Json(CredentialWithData { credential, data }))
}

/// Create a new credential
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateCredential>,
) -> ApiResult<Json<Credential>> {
    // Validate name
    if input.name.trim().is_empty() {
        return Err(ApiError::BadRequest("Name cannot be empty".to_string()));
    }

    // Encrypt the credential data
    let data_json = serde_json::to_string(&input.data)
        .map_err(|e| ApiError::BadRequest(format!("Invalid data: {}", e)))?;
    let encrypted_data = state.secrets.encrypt(&data_json)?;

    let row = sqlx::query_as::<_, CredentialRow>(
        r#"
        INSERT INTO credentials (user_id, name, description, credential_type, encrypted_data, metadata, provider, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#
    )
    .bind(user.id)
    .bind(input.name.trim())
    .bind(&input.description)
    .bind(&input.credential_type)
    .bind(&encrypted_data)
    .bind(input.metadata.unwrap_or(serde_json::json!({})))
    .bind(&input.provider)
    .bind(&input.expires_at)
    .fetch_one(state.db.pool())
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("credentials_user_id_name_key") {
                return ApiError::Conflict(format!("Credential with name '{}' already exists", input.name));
            }
        }
        ApiError::Database(e)
    })?;

    // Log creation
    log_access(&state, row.id, user.id, "create", None).await?;

    Ok(Json(row.into()))
}

/// Update a credential
pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateCredential>,
) -> ApiResult<Json<Credential>> {
    // First check if credential exists and belongs to user
    let existing = sqlx::query_as::<_, CredentialRow>(
        "SELECT * FROM credentials WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    // Build update
    let name = input.name.unwrap_or(existing.name);
    let description = input.description.or(existing.description);
    let metadata = input.metadata.unwrap_or(existing.metadata);
    let provider = input.provider.or(existing.provider);
    let expires_at = input.expires_at.or(existing.expires_at);

    // Handle data update (re-encrypt if provided)
    let encrypted_data = if let Some(new_data) = input.data {
        let data_json = serde_json::to_string(&new_data)
            .map_err(|e| ApiError::BadRequest(format!("Invalid data: {}", e)))?;
        state.secrets.encrypt(&data_json)?
    } else {
        existing.encrypted_data
    };

    let row = sqlx::query_as::<_, CredentialRow>(
        r#"
        UPDATE credentials
        SET name = $1, description = $2, encrypted_data = $3, metadata = $4, 
            provider = $5, expires_at = $6, updated_at = NOW()
        WHERE id = $7 AND user_id = $8
        RETURNING *
        "#,
    )
    .bind(&name)
    .bind(&description)
    .bind(&encrypted_data)
    .bind(&metadata)
    .bind(&provider)
    .bind(&expires_at)
    .bind(id)
    .bind(user.id)
    .fetch_one(state.db.pool())
    .await?;

    // Log update
    log_access(&state, id, user.id, "update", None).await?;

    Ok(Json(row.into()))
}

/// Delete a credential
pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Log deletion before deleting (so we have the credential_id)
    log_access(&state, id, user.id, "delete", None).await.ok();

    let result = sqlx::query("DELETE FROM credentials WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(state.db.pool())
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Get access logs for a credential
pub async fn get_access_logs(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<CredentialAccessLog>>> {
    // First verify the credential belongs to the user
    let _credential = sqlx::query_as::<_, CredentialRow>(
        "SELECT * FROM credentials WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    let logs = sqlx::query_as::<_, CredentialAccessLog>(
        r#"
        SELECT id, credential_id, user_id, action, 
               host(ip_address)::text as ip_address, user_agent, details, created_at
        FROM credential_access_log 
        WHERE credential_id = $1 
        ORDER BY created_at DESC 
        LIMIT 100
        "#,
    )
    .bind(id)
    .fetch_all(state.db.pool())
    .await?;

    Ok(Json(logs))
}

/// Helper to log credential access
async fn log_access(
    state: &AppState,
    credential_id: Uuid,
    user_id: Uuid,
    action: &str,
    details: Option<serde_json::Value>,
) -> ApiResult<()> {
    sqlx::query(
        r#"
        INSERT INTO credential_access_log (credential_id, user_id, action, details)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(credential_id)
    .bind(user_id)
    .bind(action)
    .bind(details)
    .execute(state.db.pool())
    .await?;

    Ok(())
}

/// Verify a credential is valid (check expiration)
pub async fn verify(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_as::<_, CredentialRow>(
        "SELECT * FROM credentials WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(state.db.pool())
    .await?
    .ok_or(ApiError::NotFound)?;

    let is_expired = row.expires_at.map(|exp| exp < Utc::now()).unwrap_or(false);

    Ok(Json(serde_json::json!({
        "valid": !is_expired,
        "is_expired": is_expired,
        "expires_at": row.expires_at,
    })))
}
