//! Rhai transformation engine with security sandboxing.

use rhai::{Engine, Scope, AST};

use super::context::{RequestContext, ResponseContext};
use crate::error::ApiError;

/// Create a sandboxed Rhai engine with security limits.
fn create_sandboxed_engine() -> Engine {
    let mut engine = Engine::new();

    // Safety limits to prevent abuse
    engine.set_max_operations(10_000);
    engine.set_max_call_levels(16);
    engine.set_max_expr_depths(64, 64);
    engine.set_max_string_size(100_000); // 100KB strings
    engine.set_max_array_size(1_000);
    engine.set_max_map_size(100);
    engine.set_max_variables(50);
    engine.set_max_functions(20);
    engine.set_max_modules(0); // No module imports

    // Disable dangerous features
    engine.disable_symbol("eval");

    // Register custom functions
    engine.register_fn("timestamp", || chrono::Utc::now().timestamp());
    engine.register_fn("timestamp_ms", || chrono::Utc::now().timestamp_millis());
    engine.register_fn("uuid", || uuid::Uuid::new_v4().to_string());

    engine
}

/// Transformation engine for request/response modification.
///
/// This is a zero-sized type that creates a fresh Rhai engine per operation.
/// This design ensures thread-safety (Send + Sync) without synchronization overhead.
#[derive(Clone, Copy, Default)]
pub struct TransformEngine;

impl TransformEngine {
    /// Create a new transform engine.
    pub fn new() -> Self {
        Self
    }

    /// Compile a script for later execution.
    pub fn compile(&self, script: &str) -> Result<CompiledScript, ApiError> {
        let engine = create_sandboxed_engine();
        let ast = engine
            .compile(script)
            .map_err(|e| ApiError::TransformError(format!("Script compilation failed: {}", e)))?;

        Ok(CompiledScript { ast })
    }

    /// Transform a request using the given script.
    pub fn transform_request(
        &self,
        script: &CompiledScript,
        ctx: &mut RequestContext,
    ) -> Result<(), ApiError> {
        let engine = create_sandboxed_engine();
        let mut scope = Scope::new();

        // Add context variables
        scope.push("method", ctx.method.clone());
        scope.push("path", ctx.path.clone());
        scope.push("headers", ctx.headers.clone());
        scope.push("body", ctx.body.clone());
        scope.push("model", ctx.model.clone().unwrap_or_default());

        // Execute script
        engine
            .run_ast_with_scope(&mut scope, &script.ast)
            .map_err(|e| ApiError::TransformError(format!("Script execution failed: {}", e)))?;

        // Extract modified values
        if let Some(body) = scope.get_value("body") {
            ctx.body = body;
        }
        if let Some(headers) = scope.get_value("headers") {
            ctx.headers = headers;
        }
        if let Some(model) = scope.get_value::<String>("model") {
            if !model.is_empty() {
                ctx.model = Some(model);
            }
        }

        Ok(())
    }

    /// Transform a response using the given script.
    pub fn transform_response(
        &self,
        script: &CompiledScript,
        ctx: &mut ResponseContext,
    ) -> Result<(), ApiError> {
        let engine = create_sandboxed_engine();
        let mut scope = Scope::new();

        // Add context variables
        scope.push("status_code", ctx.status_code);
        scope.push("headers", ctx.headers.clone());
        scope.push("body", ctx.body.clone());
        scope.push("model", ctx.model.clone().unwrap_or_default());
        scope.push("input_tokens", ctx.input_tokens.unwrap_or(0));
        scope.push("output_tokens", ctx.output_tokens.unwrap_or(0));

        // Execute script
        engine
            .run_ast_with_scope(&mut scope, &script.ast)
            .map_err(|e| ApiError::TransformError(format!("Script execution failed: {}", e)))?;

        // Extract modified values
        if let Some(body) = scope.get_value("body") {
            ctx.body = body;
        }
        if let Some(headers) = scope.get_value("headers") {
            ctx.headers = headers;
        }

        Ok(())
    }

    /// Transform a request body directly with a script string.
    pub fn transform_request_body(
        &self,
        script: &str,
        method: &str,
        path: &str,
        headers: &http::HeaderMap,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, ApiError> {
        let compiled = self.compile(script)?;
        let mut ctx = RequestContext::new(method, path, headers, body);
        self.transform_request(&compiled, &mut ctx)?;
        ctx.to_json_body().map_err(ApiError::TransformError)
    }

    /// Transform a response body directly with a script string.
    pub fn transform_response_body(
        &self,
        script: &str,
        status_code: u16,
        headers: &http::HeaderMap,
        body: serde_json::Value,
        input_tokens: Option<i32>,
        output_tokens: Option<i32>,
    ) -> Result<serde_json::Value, ApiError> {
        let compiled = self.compile(script)?;
        let mut ctx = ResponseContext::new(status_code, headers, body, input_tokens, output_tokens);
        self.transform_response(&compiled, &mut ctx)?;
        ctx.to_json_body().map_err(ApiError::TransformError)
    }
}

/// A compiled Rhai script ready for execution.
pub struct CompiledScript {
    ast: AST,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_transform() {
        let engine = TransformEngine::new();

        let script = r#"
            // Inject system prompt
            if body.messages.len() == 0 || body.messages[0].role != "system" {
                body.messages.insert(0, #{
                    role: "system",
                    content: "You are a helpful assistant."
                });
            }
            
            // Override temperature
            body.temperature = 0.7;
        "#;

        let headers = http::HeaderMap::new();
        let body = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let result = engine
            .transform_request_body(script, "POST", "/v1/chat/completions", &headers, body)
            .unwrap();

        assert_eq!(result["temperature"], 0.7);
        assert_eq!(result["messages"][0]["role"], "system");
        assert_eq!(result["messages"][1]["role"], "user");
    }

    #[test]
    fn test_response_transform() {
        let engine = TransformEngine::new();

        let script = r#"
            // Add custom field
            body.proxy_metadata = #{
                processed: true,
                timestamp: timestamp()
            };
        "#;

        let headers = http::HeaderMap::new();
        let body = serde_json::json!({
            "id": "chatcmpl-123",
            "model": "gpt-4",
            "choices": []
        });

        let result = engine
            .transform_response_body(script, 200, &headers, body, Some(100), Some(50))
            .unwrap();

        assert!(result["proxy_metadata"]["processed"].as_bool().unwrap());
        assert!(result["proxy_metadata"]["timestamp"].as_i64().is_some());
    }

    #[test]
    fn test_script_limits() {
        let engine = TransformEngine::new();

        // This should fail due to operation limit
        let infinite_loop = r#"
            let i = 0;
            while true {
                i += 1;
            }
        "#;

        let headers = http::HeaderMap::new();
        let body = serde_json::json!({});

        let result = engine.transform_request_body(infinite_loop, "POST", "/test", &headers, body);

        assert!(result.is_err());
    }
}
