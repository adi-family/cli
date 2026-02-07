//! Anthropic Messages API handler.

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use bytes::Bytes;
use futures::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use crate::{handlers::common::*, middleware::ProxyAuth, state::AppState};
use llm_proxy_core::{ApiError, ApiResult, ProviderType, ProxyRequest, RequestStatus};

/// Handle POST /v1/messages (Anthropic Messages API)
pub async fn messages(
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

    // Verify provider is Anthropic
    let provider_type = match token.key_mode {
        llm_proxy_core::KeyMode::Byok => {
            let key_id = token.upstream_key_id.ok_or_else(|| {
                ApiError::Internal("BYOK token missing upstream_key_id".to_string())
            })?;
            let key = llm_proxy_core::db::keys::get_upstream_key_by_id(state.db.pool(), key_id)
                .await?;
            key.provider_type
        }
        llm_proxy_core::KeyMode::Platform => token.platform_provider.ok_or_else(|| {
            ApiError::Internal("Platform token missing platform_provider".to_string())
        })?,
    };

    if provider_type != ProviderType::Anthropic {
        return Err(ApiError::BadRequest(
            "/v1/messages endpoint is only available for Anthropic provider".to_string(),
        ));
    }

    // Check if streaming
    let is_streaming = body
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    // Create proxy context
    let ctx = ProxyContext::new(
        &state,
        token.clone(),
        "/v1/messages",
        is_streaming,
        requested_model.clone(),
    )
    .await?;

    // Transform request
    let transformed_body = transform_request(&state, &token, "POST", "/v1/messages", body.clone())?;

    // Build proxy request
    let proxy_request = ProxyRequest {
        method: http::Method::POST,
        path: "/v1/messages".to_string(),
        headers: http::HeaderMap::new(),
        body: transformed_body.clone(),
    };

    if is_streaming {
        handle_streaming_messages(state, ctx, proxy_request, body).await
    } else {
        handle_non_streaming_messages(state, ctx, proxy_request, body).await
    }
}

async fn handle_non_streaming_messages(
    state: AppState,
    ctx: ProxyContext,
    request: ProxyRequest,
    original_body: serde_json::Value,
) -> Result<Response, ApiError> {
    let timeout = state.config.upstream_timeout_secs;

    let result = ctx
        .provider
        .forward(ctx.api_key(), &ctx.endpoint, request, timeout)
        .await;

    match result {
        Ok(response) => {
            let usage = ctx.provider.extract_usage(&response);
            let upstream_id = ctx.provider.extract_request_id(&response);
            let actual_model = ctx.provider.extract_model(&response);

            let transformed = transform_response(
                &state,
                &ctx.token,
                response.status.as_u16(),
                response.body.clone(),
                &usage,
            )?;

            ctx.log_result(
                &state,
                RequestStatus::Success,
                Some(response.status.as_u16() as i16),
                usage,
                None,
                upstream_id,
                actual_model,
                None,
                None,
                None,
                Some(&original_body),
                Some(&transformed),
            )
            .await
            .ok();

            Ok(Json(transformed).into_response())
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
                Some(&original_body),
                None,
            )
            .await
            .ok();

            Err(ApiError::UpstreamError(e.to_string()))
        }
    }
}

async fn handle_streaming_messages(
    state: AppState,
    ctx: ProxyContext,
    request: ProxyRequest,
    original_body: serde_json::Value,
) -> Result<Response, ApiError> {
    let timeout = state.config.upstream_timeout_secs;

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

            tokio::spawn(async move {
                let mut first_chunk = true;
                let mut ttft_ms: Option<i32> = None;
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

                            // Parse Anthropic SSE for usage
                            if let Ok(text) = std::str::from_utf8(&chunk) {
                                for line in text.lines() {
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if let Ok(json) =
                                            serde_json::from_str::<serde_json::Value>(data)
                                        {
                                            // Extract model
                                            if actual_model.is_none() {
                                                actual_model = json
                                                    .get("model")
                                                    .and_then(|m| m.as_str())
                                                    .map(|s| s.to_string());
                                            }

                                            // Extract usage from message_stop event
                                            if json.get("type").and_then(|t| t.as_str())
                                                == Some("message_delta")
                                            {
                                                if let Some(usage) = json.get("usage") {
                                                    usage_info =
                                                        Some(llm_proxy_core::UsageInfo {
                                                            input_tokens: usage
                                                                .get("input_tokens")
                                                                .and_then(|v| v.as_i64())
                                                                .map(|v| v as i32),
                                                            output_tokens: usage
                                                                .get("output_tokens")
                                                                .and_then(|v| v.as_i64())
                                                                .map(|v| v as i32),
                                                            total_tokens: None,
                                                        });
                                                }
                                            }
                                        }
                                    }
                                }
                            }

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

                let latency_ms = start_time.elapsed().as_millis() as i32;

                let _ = llm_proxy_core::db::usage::log_usage(
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
                    None,
                    None,
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
            });

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
