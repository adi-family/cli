//! Chat completions handler with streaming support.

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use futures::StreamExt;
use std::time::Instant;
use tokio_stream::wrappers::ReceiverStream;

use crate::{handlers::common::*, middleware::ProxyAuth, state::AppState};
use llm_proxy_core::{ApiError, ApiResult, ProxyRequest, RequestStatus};

/// Handle POST /v1/chat/completions
pub async fn chat_completions(
    State(state): State<AppState>,
    auth: ProxyAuth,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, ApiError> {
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

    // Check if streaming
    let is_streaming = body
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    // Create proxy context
    let ctx = ProxyContext::new(
        &state,
        token.clone(),
        "/v1/chat/completions",
        is_streaming,
        requested_model.clone(),
    )
    .await?;

    // Transform request
    let transformed_body =
        transform_request(&state, &token, "POST", "/v1/chat/completions", body.clone())?;

    // Build proxy request
    let proxy_request = ProxyRequest {
        method: http::Method::POST,
        path: "/v1/chat/completions".to_string(),
        headers: http::HeaderMap::new(),
        body: transformed_body.clone(),
    };

    if is_streaming {
        handle_streaming(state, ctx, proxy_request, body).await
    } else {
        handle_non_streaming(state, ctx, proxy_request, body).await
    }
}

async fn handle_non_streaming(
    state: AppState,
    ctx: ProxyContext,
    request: ProxyRequest,
    original_body: serde_json::Value,
) -> Result<Response, ApiError> {
    let timeout = state.config.upstream_timeout_secs;

    // Forward request
    let result = ctx
        .provider
        .forward(ctx.api_key(), &ctx.endpoint, request, timeout)
        .await;

    match result {
        Ok(response) => {
            let usage = ctx.provider.extract_usage(&response);
            let cost = ctx.provider.extract_cost(&response);
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
                cost,
                upstream_id,
                actual_model,
                None,
                None,
                None,
                Some(&original_body),
                Some(&transformed),
            )
            .await
            .ok(); // Don't fail on logging errors

            Ok(Json(transformed).into_response())
        }
        Err(e) => {
            let (status, error_type) = match &e {
                llm_proxy_core::providers::ProviderError::AuthenticationFailed => {
                    (StatusCode::UNAUTHORIZED, "authentication_failed")
                }
                llm_proxy_core::providers::ProviderError::RateLimited => {
                    (StatusCode::TOO_MANY_REQUESTS, "rate_limited")
                }
                llm_proxy_core::providers::ProviderError::Timeout => {
                    (StatusCode::GATEWAY_TIMEOUT, "timeout")
                }
                _ => (StatusCode::BAD_GATEWAY, "upstream_error"),
            };

            ctx.log_result(
                &state,
                RequestStatus::UpstreamError,
                Some(status.as_u16() as i16),
                None,
                None,
                None,
                None,
                None,
                Some(error_type),
                Some(&e.to_string()),
                Some(&original_body),
                None,
            )
            .await
            .ok();

            Err(ApiError::UpstreamError(e.to_string()))
        }
    }
}

async fn handle_streaming(
    state: AppState,
    ctx: ProxyContext,
    request: ProxyRequest,
    original_body: serde_json::Value,
) -> Result<Response, ApiError> {
    let timeout = state.config.upstream_timeout_secs;

    // Forward streaming request
    let stream_result = ctx
        .provider
        .forward_stream(ctx.api_key(), &ctx.endpoint, request, timeout)
        .await;

    match stream_result {
        Ok(upstream_stream) => {
            let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(32);

            let state_clone = state.clone();
            let token = ctx.token.clone();
            let request_id = ctx.request_id.clone();
            let endpoint = ctx.endpoint.clone();
            let provider_type = ctx.provider_config.provider_type;
            let key_mode = ctx.token.key_mode;
            let requested_model = ctx.requested_model.clone();
            let start_time = ctx.start_time;

            // Spawn task to process stream and extract usage
            tokio::spawn(async move {
                let mut first_chunk = true;
                let mut ttft_ms: Option<i32> = None;
                let mut collected_data = String::new();
                let mut usage_info: Option<llm_proxy_core::UsageInfo> = None;
                let mut actual_model: Option<String> = None;

                let mut stream = upstream_stream;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            if first_chunk {
                                ttft_ms = Some(start_time.elapsed().as_millis() as i32);
                                first_chunk = false;
                            }

                            // Collect data for parsing
                            if let Ok(text) = std::str::from_utf8(&chunk) {
                                collected_data.push_str(text);

                                // Try to extract usage from SSE data
                                for line in text.lines() {
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if data != "[DONE]" {
                                            if let Ok(json) =
                                                serde_json::from_str::<serde_json::Value>(data)
                                            {
                                                // Extract model on first chunk
                                                if actual_model.is_none() {
                                                    actual_model = json
                                                        .get("model")
                                                        .and_then(|m| m.as_str())
                                                        .map(|s| s.to_string());
                                                }

                                                // Extract usage from final chunk
                                                if let Some(usage) = json.get("usage") {
                                                    usage_info =
                                                        Some(llm_proxy_core::UsageInfo {
                                                            input_tokens: usage
                                                                .get("prompt_tokens")
                                                                .and_then(|v| v.as_i64())
                                                                .map(|v| v as i32),
                                                            output_tokens: usage
                                                                .get("completion_tokens")
                                                                .and_then(|v| v.as_i64())
                                                                .map(|v| v as i32),
                                                            total_tokens: usage
                                                                .get("total_tokens")
                                                                .and_then(|v| v.as_i64())
                                                                .map(|v| v as i32),
                                                        });
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Forward chunk to client
                            if tx.send(Ok(chunk)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Stream error: {}", e);
                            break;
                        }
                    }
                }

                // Log the completed stream
                let latency_ms = start_time.elapsed().as_millis() as i32;

                let log_result = llm_proxy_core::db::usage::log_usage(
                    state_clone.db.pool(),
                    token.id,
                    token.user_id,
                    &request_id,
                    None,
                    requested_model.as_deref(),
                    actual_model.as_deref(),
                    provider_type,
                    key_mode,
                    usage_info.as_ref().and_then(|u| u.input_tokens),
                    usage_info.as_ref().and_then(|u| u.output_tokens),
                    usage_info.as_ref().and_then(|u| u.total_tokens),
                    None, // cost
                    &endpoint,
                    true,
                    Some(latency_ms),
                    ttft_ms,
                    RequestStatus::Success,
                    Some(200),
                    None,
                    None,
                    if token.log_requests {
                        Some(&original_body)
                    } else {
                        None
                    },
                    None,
                )
                .await;

                if let Err(e) = log_result {
                    tracing::error!("Failed to log usage: {}", e);
                }
            });

            // Build SSE response
            let body = Body::from_stream(ReceiverStream::new(rx));

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/event-stream")
                .header(header::CACHE_CONTROL, "no-cache")
                .header(header::CONNECTION, "keep-alive")
                .body(body)
                .unwrap())
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
                Some("stream_error"),
                Some(&e.to_string()),
                Some(&original_body),
                None,
            )
            .await
            .ok();

            Err(ApiError::UpstreamError(e.to_string()))
        }
    }
}
