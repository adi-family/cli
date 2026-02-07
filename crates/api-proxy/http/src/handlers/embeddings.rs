//! Embeddings handler.

use axum::{extract::State, Json};

use crate::{handlers::common::*, middleware::ProxyAuth, state::AppState};
use api_proxy_core::{ApiError, ApiResult, ProxyRequest, RequestStatus};

/// Handle POST /v1/embeddings
pub async fn embeddings(
    State(state): State<AppState>,
    auth: ProxyAuth,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    // Extract model from request
    let requested_model = body
        .get("model")
        .and_then(|m| m.as_str())
        .map(|s| s.to_string());

    // Check if model is allowed (before moving token)
    if let Some(ref model) = requested_model {
        if !auth.is_model_allowed(model) {
            return Err(ApiError::Forbidden(format!(
                "Model '{}' is not allowed",
                model
            )));
        }
    }

    let token = auth.token;

    // Create proxy context
    let ctx = ProxyContext::new(
        &state,
        token.clone(),
        "/v1/embeddings",
        false,
        requested_model.clone(),
    )
    .await?;

    // Transform request
    let transformed_body =
        transform_request(&state, &token, "POST", "/v1/embeddings", body.clone())?;

    // Build proxy request
    let proxy_request = ProxyRequest {
        method: http::Method::POST,
        path: "/v1/embeddings".to_string(),
        headers: http::HeaderMap::new(),
        body: transformed_body,
    };

    let timeout = state.config.upstream_timeout_secs;

    // Forward request
    let result = ctx
        .provider
        .forward(ctx.api_key(), &ctx.endpoint, proxy_request, timeout)
        .await;

    match result {
        Ok(response) => {
            let usage = ctx.provider.extract_usage(&response);
            let upstream_id = ctx.provider.extract_request_id(&response);
            let actual_model = ctx.provider.extract_model(&response);

            // Transform response
            let transformed = transform_response(
                &state,
                &ctx.token,
                response.status.as_u16(),
                response.body.clone(),
                &usage,
            )?;

            // Log result
            ctx.log_result(
                &state,
                RequestStatus::Success,
                Some(response.status.as_u16() as i16),
                usage,
                None, // Embeddings don't have cost
                upstream_id,
                actual_model,
                None,
                None,
                None,
                Some(&body),
                Some(&transformed),
            )
            .await
            .ok();

            Ok(Json(transformed))
        }
        Err(e) => {
            ctx.log_result(
                &state,
                RequestStatus::UpstreamError,
                Some(502),
                None,
                None,
                None,
                None,
                None,
                Some("upstream_error"),
                Some(&e.to_string()),
                Some(&body),
                None,
            )
            .await
            .ok();

            Err(ApiError::UpstreamError(e.to_string()))
        }
    }
}
