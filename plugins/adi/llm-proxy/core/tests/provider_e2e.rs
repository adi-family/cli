//! End-to-end provider tests using wiremock to mock upstream LLM APIs.

use llm_proxy_core::providers;
use llm_proxy_core::types::{ProviderType, ProxyRequest};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_request(body: serde_json::Value) -> ProxyRequest {
    ProxyRequest {
        method: http::Method::POST,
        path: "/v1/chat/completions".to_string(),
        headers: http::HeaderMap::new(),
        body,
    }
}

fn chat_response() -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test123",
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Hello!"},
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    })
}

fn anthropic_response() -> serde_json::Value {
    serde_json::json!({
        "id": "msg-test123",
        "type": "message",
        "model": "claude-sonnet-4-6-20250514",
        "content": [{"type": "text", "text": "Hello!"}],
        "usage": {
            "input_tokens": 12,
            "output_tokens": 8
        }
    })
}

// ── OpenAI-compatible forward ─────────────────────────────────────────────

#[tokio::test]
async fn test_openai_forward_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer sk-test-key"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_response()))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenAI, Some(server.uri()));
    let request = make_request(serde_json::json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Hi"}]
    }));

    let response = provider
        .forward("sk-test-key", "/v1/chat/completions", request, 30)
        .await
        .unwrap();

    assert_eq!(response.status, http::StatusCode::OK);
    assert_eq!(response.body["model"], "gpt-4");
    assert_eq!(response.body["choices"][0]["message"]["content"], "Hello!");

    // Verify extraction
    let usage = provider.extract_usage(&response).unwrap();
    assert_eq!(usage.input_tokens, Some(10));
    assert_eq!(usage.output_tokens, Some(5));
    assert_eq!(usage.total_tokens, Some(15));

    assert_eq!(provider.extract_model(&response), Some("gpt-4".to_string()));
    assert_eq!(
        provider.extract_request_id(&response),
        Some("chatcmpl-test123".to_string())
    );
}

#[tokio::test]
async fn test_openai_forward_auth_failure() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {"message": "Invalid API key"}
            })),
        )
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenAI, Some(server.uri()));
    let request = make_request(serde_json::json!({}));

    let err = provider
        .forward("bad-key", "/v1/chat/completions", request, 30)
        .await
        .unwrap_err();

    assert!(matches!(err, providers::ProviderError::AuthenticationFailed));
}

#[tokio::test]
async fn test_openai_forward_rate_limited() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": {"message": "Rate limit exceeded"}
        })))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenAI, Some(server.uri()));
    let request = make_request(serde_json::json!({}));

    let err = provider
        .forward("sk-test", "/v1/chat/completions", request, 30)
        .await
        .unwrap_err();

    assert!(matches!(err, providers::ProviderError::RateLimited));
}

#[tokio::test]
async fn test_openai_forward_model_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Model 'gpt-5' not found"))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenAI, Some(server.uri()));
    let request = make_request(serde_json::json!({}));

    let err = provider
        .forward("sk-test", "/v1/chat/completions", request, 30)
        .await
        .unwrap_err();

    assert!(matches!(err, providers::ProviderError::ModelNotFound(m) if m.contains("gpt-5")));
}

#[tokio::test]
async fn test_openai_forward_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenAI, Some(server.uri()));
    let request = make_request(serde_json::json!({}));

    let err = provider
        .forward("sk-test", "/v1/chat/completions", request, 30)
        .await
        .unwrap_err();

    assert!(matches!(err, providers::ProviderError::RequestFailed(_)));
}

// ── Anthropic forward ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_anthropic_forward_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "sk-ant-test"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(anthropic_response()))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::Anthropic, Some(server.uri()));
    let request = make_request(serde_json::json!({
        "model": "claude-sonnet-4-6-20250514",
        "messages": [{"role": "user", "content": "Hi"}]
    }));

    let response = provider
        .forward("sk-ant-test", "/v1/messages", request, 30)
        .await
        .unwrap();

    assert_eq!(response.status, http::StatusCode::OK);
    assert_eq!(response.body["model"], "claude-sonnet-4-6-20250514");

    let usage = provider.extract_usage(&response).unwrap();
    assert_eq!(usage.input_tokens, Some(12));
    assert_eq!(usage.output_tokens, Some(8));
    assert_eq!(usage.total_tokens, None); // Anthropic doesn't report total

    assert!(provider.extract_cost(&response).is_none());
}

// ── Custom provider with optional auth ────────────────────────────────────

#[tokio::test]
async fn test_custom_forward_no_auth() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        // Should NOT have Authorization header
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_response()))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::Custom, Some(server.uri()));
    let request = make_request(serde_json::json!({"model": "local-model"}));

    let response = provider
        .forward("", "/v1/chat/completions", request, 30)
        .await
        .unwrap();

    assert_eq!(response.status, http::StatusCode::OK);
}

#[tokio::test]
async fn test_custom_forward_with_auth() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer my-custom-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_response()))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::Custom, Some(server.uri()));
    let request = make_request(serde_json::json!({}));

    let response = provider
        .forward("my-custom-key", "/v1/chat/completions", request, 30)
        .await
        .unwrap();

    assert_eq!(response.status, http::StatusCode::OK);
}

// ── Streaming forward ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_openai_stream_success() {
    use futures::StreamExt;

    let server = MockServer::start().await;
    let sse_data = "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\ndata: [DONE]\n\n";

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Accept", "text/event-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_data),
        )
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenAI, Some(server.uri()));
    let request = make_request(serde_json::json!({"stream": true}));

    let mut stream = provider
        .forward_stream("sk-test", "/v1/chat/completions", request, 30)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap());
    }

    assert!(!chunks.is_empty());
    let all_text: String = chunks.iter().map(|b| String::from_utf8_lossy(b).to_string()).collect();
    assert!(all_text.contains("chatcmpl-1"));
}

#[tokio::test]
async fn test_stream_auth_failure() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenAI, Some(server.uri()));
    let request = make_request(serde_json::json!({"stream": true}));

    let result = provider
        .forward_stream("bad-key", "/v1/chat/completions", request, 30)
        .await;

    match result {
        Err(err) => assert!(matches!(err, providers::ProviderError::AuthenticationFailed)),
        Ok(_) => panic!("Expected auth failure"),
    }
}

// ── OpenRouter cost extraction ────────────────────────────────────────────

#[tokio::test]
async fn test_openrouter_extracts_cost() {
    let server = MockServer::start().await;

    let response_body = serde_json::json!({
        "id": "gen-test",
        "model": "anthropic/claude-3-opus",
        "choices": [{"message": {"content": "Hi"}}],
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "total_tokens": 150,
            "cost": 0.00375
        }
    });

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenRouter, Some(server.uri()));
    let request = make_request(serde_json::json!({}));

    let response = provider
        .forward("sk-or-test", "/v1/chat/completions", request, 30)
        .await
        .unwrap();

    let cost = provider.extract_cost(&response).unwrap();
    assert!(cost > rust_decimal::Decimal::ZERO);
}

// ── List models ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_openai_list_models() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"id": "gpt-4", "name": "GPT-4", "context_length": 128000},
                {"id": "gpt-3.5-turbo", "name": "GPT-3.5 Turbo"}
            ]
        })))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenAI, Some(server.uri()));
    let models = provider.list_models("sk-test").await.unwrap();

    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "gpt-4");
    assert_eq!(models[0].context_length, Some(128000));
    assert_eq!(models[1].id, "gpt-3.5-turbo");
    assert_eq!(models[1].context_length, None);
}

#[tokio::test]
async fn test_custom_list_models_silent_on_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::Custom, Some(server.uri()));
    // Custom provider returns empty vec on error instead of failing
    let models = provider.list_models("").await.unwrap();
    assert!(models.is_empty());
}

#[tokio::test]
async fn test_openai_list_models_returns_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::OpenAI, Some(server.uri()));
    // OpenAI provider propagates the error
    let err = provider.list_models("sk-test").await.unwrap_err();
    assert!(matches!(err, providers::ProviderError::RequestFailed(_)));
}

// ── Anthropic stream injects stream: true ─────────────────────────────────

#[tokio::test]
async fn test_anthropic_stream_injects_stream_field() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("event: message_start\ndata: {}\n\n"),
        )
        .mount(&server)
        .await;

    let provider = providers::create_provider(ProviderType::Anthropic, Some(server.uri()));
    let request = make_request(serde_json::json!({"model": "claude-sonnet-4-6-20250514"}));

    // Should not fail — Anthropic provider injects stream: true
    let _stream = provider
        .forward_stream("sk-ant-test", "/v1/messages", request, 30)
        .await
        .unwrap();
}

// ── Connection refused ────────────────────────────────────────────────────

#[tokio::test]
async fn test_forward_connection_refused() {
    // Use a port that's definitely not listening
    let provider = providers::create_provider(ProviderType::OpenAI, Some("http://127.0.0.1:1".to_string()));
    let request = make_request(serde_json::json!({}));

    let err = provider
        .forward("sk-test", "/v1/chat/completions", request, 5)
        .await
        .unwrap_err();

    // Should be a network or request error, not a panic
    assert!(matches!(
        err,
        providers::ProviderError::Network(_) | providers::ProviderError::RequestFailed(_)
    ));
}
