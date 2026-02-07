//! Platform provider listing routes.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::{middleware::AuthUser, state::AppState};
use api_proxy_core::{db, ApiResult, ModelInfo, ProviderType};

/// Provider summary for listing.
#[derive(Debug, Serialize)]
pub struct ProviderSummary {
    pub provider_type: ProviderType,
    pub is_available: bool,
    pub allowed_models: Vec<AllowedModelInfo>,
}

/// Allowed model info.
#[derive(Debug, Serialize)]
pub struct AllowedModelInfo {
    pub model_id: String,
    pub display_name: Option<String>,
}

/// Response for listing providers.
#[derive(Debug, Serialize)]
pub struct ListProvidersResponse {
    pub providers: Vec<ProviderSummary>,
}

/// List available platform providers and their allowed models.
pub async fn list_providers(
    State(state): State<AppState>,
    _user: AuthUser,
) -> ApiResult<Json<ListProvidersResponse>> {
    // Get all platform keys
    let platform_keys = db::platform_keys::list_platform_keys(state.db.pool()).await?;

    // Get all allowed models
    let allowed_models = db::models::list_all_allowed_models(state.db.pool()).await?;

    // Build provider list
    let all_providers = vec![
        ProviderType::OpenAI,
        ProviderType::Anthropic,
        ProviderType::OpenRouter,
    ];

    let providers: Vec<ProviderSummary> = all_providers
        .into_iter()
        .map(|pt| {
            let is_available = platform_keys
                .iter()
                .any(|k| k.provider_type == pt && k.is_active);

            let models: Vec<AllowedModelInfo> = allowed_models
                .iter()
                .filter(|m| m.provider_type == pt)
                .map(|m| AllowedModelInfo {
                    model_id: m.model_id.clone(),
                    display_name: m.display_name.clone(),
                })
                .collect();

            ProviderSummary {
                provider_type: pt,
                is_available,
                allowed_models: models,
            }
        })
        .collect();

    Ok(Json(ListProvidersResponse { providers }))
}
