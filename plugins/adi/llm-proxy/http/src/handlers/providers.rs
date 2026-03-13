use axum::Json;
use axum::extract::State;

use crate::AppState;
use crate::auth::AuthUser;
use llm_proxy_core::error::ApiResult;
use llm_proxy_core::db;
use llm_proxy_core::types::ProviderType;

pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let platform_keys = db::list_platform_keys(state.db.pool()).await?;
    let all_models = db::list_all_allowed_models(state.db.pool()).await?;

    let provider_types = [
        ProviderType::OpenAI,
        ProviderType::Anthropic,
        ProviderType::OpenRouter,
        ProviderType::Custom,
    ];

    let providers: Vec<serde_json::Value> = provider_types
        .iter()
        .map(|pt| {
            let is_available = platform_keys
                .iter()
                .any(|k| k.provider_type == *pt && k.is_active);
            let models: Vec<serde_json::Value> = all_models
                .iter()
                .filter(|m| m.provider_type == *pt)
                .map(|m| {
                    serde_json::json!({
                        "model_id": m.model_id,
                        "display_name": m.display_name,
                    })
                })
                .collect();
            serde_json::json!({
                "provider_type": pt,
                "is_available": is_available,
                "allowed_models": models,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "providers": providers })))
}
