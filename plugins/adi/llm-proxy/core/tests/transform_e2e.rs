//! End-to-end transform tests: Rhai scripts applied to real request/response payloads.

use llm_proxy_core::transform::TransformEngine;

#[test]
fn test_full_request_transform_pipeline() {
    let engine = TransformEngine::new();
    let headers = http::HeaderMap::new();

    // Simulate a request that needs system prompt injection and model override
    let body = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Tell me a joke"}
        ],
        "temperature": 1.0
    });

    let script = r#"
        // Force model upgrade
        body.model = "gpt-4";

        // Inject system prompt
        body.messages.insert(0, #{
            role: "system",
            content: "You are a comedian. Keep responses under 100 words."
        });

        // Cap temperature
        if body.temperature > 0.9 {
            body.temperature = 0.9;
        }

        // Add custom metadata
        body.user = "proxy-user-" + uuid();
    "#;

    let result = engine
        .transform_request_body(script, "POST", "/v1/chat/completions", &headers, body)
        .unwrap();

    assert_eq!(result["model"], "gpt-4");
    assert_eq!(result["messages"][0]["role"], "system");
    assert_eq!(result["messages"][1]["role"], "user");
    assert_eq!(result["temperature"], 0.9);
    assert!(result["user"].as_str().unwrap().starts_with("proxy-user-"));
}

#[test]
fn test_full_response_transform_pipeline() {
    let engine = TransformEngine::new();
    let headers = http::HeaderMap::new();

    let body = serde_json::json!({
        "id": "chatcmpl-abc",
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Why don't scientists trust atoms? Because they make up everything!"},
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 25,
            "completion_tokens": 15,
            "total_tokens": 40
        }
    });

    let script = r#"
        // Add proxy metadata
        body.proxy = #{
            processed_at: timestamp(),
            input_tokens: input_tokens,
            output_tokens: output_tokens,
            status: status_code
        };

        // Remove internal fields
        body.remove("usage");
    "#;

    let result = engine
        .transform_response_body(script, 200, &headers, body, Some(25), Some(15))
        .unwrap();

    assert_eq!(result["proxy"]["input_tokens"], 25);
    assert_eq!(result["proxy"]["output_tokens"], 15);
    assert_eq!(result["proxy"]["status"], 200);
    assert!(result["proxy"]["processed_at"].as_i64().is_some());
    assert!(result.get("usage").is_none());
    // Original content preserved
    assert_eq!(result["choices"][0]["message"]["role"], "assistant");
}

#[test]
fn test_transform_with_model_routing() {
    let engine = TransformEngine::new();
    let headers = http::HeaderMap::new();

    let body = serde_json::json!({
        "model": "auto",
        "messages": [
            {"role": "user", "content": "Summarize this paper in 2 sentences."}
        ]
    });

    // Script that routes based on task
    let script = r#"
        // Simple model routing: if messages contain "summarize", use fast model
        let content = body.messages[body.messages.len() - 1].content;
        if content.to_lower().contains("summarize") {
            body.model = "gpt-3.5-turbo";
        } else {
            body.model = "gpt-4";
        }
    "#;

    let result = engine
        .transform_request_body(script, "POST", "/v1/chat/completions", &headers, body)
        .unwrap();

    assert_eq!(result["model"], "gpt-3.5-turbo");
}

#[test]
fn test_transform_header_injection() {
    let engine = TransformEngine::new();
    let compiled = engine
        .compile(
            r#"
        headers["x-custom-tag"] = "production";
        headers["x-request-source"] = "proxy";
    "#,
        )
        .unwrap();

    let mut headers = http::HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    let body = serde_json::json!({});

    let mut ctx = llm_proxy_core::transform::RequestContext::new("POST", "/v1/test", &headers, body);
    engine.transform_request(&compiled, &mut ctx).unwrap();

    let custom_tag = ctx.headers.get("x-custom-tag").unwrap();
    assert_eq!(custom_tag.clone().into_string().unwrap(), "production");
}

#[test]
fn test_transform_error_on_bad_script() {
    let engine = TransformEngine::new();
    let headers = http::HeaderMap::new();
    let body = serde_json::json!({});

    // Accessing undefined variable should error
    let result = engine.transform_request_body(
        r#"body.x = undefined_var;"#,
        "POST",
        "/test",
        &headers,
        body,
    );

    assert!(result.is_err());
}

#[test]
fn test_transform_preserves_nested_arrays() {
    let engine = TransformEngine::new();
    let headers = http::HeaderMap::new();

    let body = serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "What's in this image?"},
                    {"type": "image_url", "image_url": {"url": "https://example.com/img.png"}}
                ]
            }
        ]
    });

    let script = r#"
        // Noop — just verify complex structure survives round-trip
        body.processed = true;
    "#;

    let result = engine
        .transform_request_body(script, "POST", "/v1/chat/completions", &headers, body)
        .unwrap();

    assert_eq!(result["processed"], true);
    assert_eq!(result["messages"][0]["content"][0]["type"], "text");
    assert_eq!(result["messages"][0]["content"][1]["type"], "image_url");
}

#[test]
fn test_transform_crypto_token_generation() {
    let engine = TransformEngine::new();
    let headers = http::HeaderMap::new();
    let body = serde_json::json!({});

    let script = r#"
        body.request_id = uuid();
        body.timestamp = timestamp();
        body.timestamp_ms = timestamp_ms();
    "#;

    let result = engine
        .transform_request_body(script, "POST", "/test", &headers, body)
        .unwrap();

    // UUID is 36 chars with dashes
    let uuid = result["request_id"].as_str().unwrap();
    assert_eq!(uuid.len(), 36);
    assert!(uuid.contains('-'));

    // Timestamps should be reasonable
    let ts = result["timestamp"].as_i64().unwrap();
    assert!(ts > 1_700_000_000); // After 2023
    let ts_ms = result["timestamp_ms"].as_i64().unwrap();
    assert!(ts_ms > 1_700_000_000_000);
}
