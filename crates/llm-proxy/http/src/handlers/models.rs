//! Models listing handler.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::{middleware::ProxyAuth, state::AppState};
use llm_proxy_core::{db, ApiError, ApiResult, KeyMode, ModelInfo};

/// Response for models listing.
#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub object: &'static str,
    pub data: Vec<ModelData>,
}

#[derive(Debug, Serialize)]
pub struct ModelData {
    pub id: String,
    pub object: &'static str,
    pub owned_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<i32>,
}

/// Handle GET /v1/models
pub async fn list_models(
    State(state): State<AppState>,
    auth: ProxyAuth,
) -> ApiResult<Json<ModelsResponse>> {
    let models: Vec<ModelInfo> = match auth.token.key_mode {
        KeyMode::Byok => {
            // For BYOK, query the upstream provider
            let upstream_key_id = auth.token.upstream_key_id.ok_or_else(|| {
                ApiError::Internal("BYOK token missing upstream_key_id".to_string())
            })?;

            let key = db::keys::get_upstream_key_by_id(state.db.pool(), upstream_key_id).await?;
            let api_key = state.secrets.decrypt(&key.api_key_encrypted)?;

            let provider =
                llm_proxy_core::providers::create_provider(key.provider_type, key.base_url);

            provider
                .list_models(&api_key)
                .await
                .map_err(|e| ApiError::UpstreamError(e.to_string()))?
        }
        KeyMode::Platform => {
            // For Platform, return allowed models from database
            let provider_type = auth.token.platform_provider.ok_or_else(|| {
                ApiError::Internal("Platform token missing platform_provider".to_string())
            })?;

            let allowed = db::models::list_allowed_models(state.db.pool(), provider_type).await?;

            allowed
                .into_iter()
                .map(|m| ModelInfo {
                    id: m.model_id,
                    name: m.display_name,
                    description: None,
                    context_length: None,
                    provider: provider_type,
                })
                .collect()
        }
    };

    // Filter models based on allowed/blocked lists
    let filtered: Vec<ModelInfo> = models
        .into_iter()
        .filter(|m| auth.is_model_allowed(&m.id))
        .collect();

    let data: Vec<ModelData> = filtered
        .into_iter()
        .map(|m| ModelData {
            id: m.id,
            object: "model",
            owned_by: m.provider.to_string(),
            context_length: m.context_length,
        })
        .collect();

    Ok(Json(ModelsResponse {
        object: "list",
        data,
    }))
}
