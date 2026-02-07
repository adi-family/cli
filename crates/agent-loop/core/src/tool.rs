use crate::error::{AgentError, Result};
use crate::types::{ToolCall, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<ToolCategory>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    ReadOnly,
    Write,
    Execute,
    External,
}

impl ToolSchema {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            category: None,
        }
    }

    pub fn with_parameters(mut self, parameters: serde_json::Value) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn with_category(mut self, category: ToolCategory) -> Self {
        self.category = Some(category);
        self
    }

    pub fn validate_arguments(&self, arguments: &serde_json::Value) -> Result<()> {
        if let Some(required) = self.parameters.get("required").and_then(|v| v.as_array()) {
            for req in required {
                if let Some(key) = req.as_str() {
                    if arguments.get(key).is_none() {
                        return Err(AgentError::InvalidArguments(format!(
                            "Missing required parameter: {}",
                            key
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    fn schema(&self) -> &ToolSchema;

    async fn execute(&self, arguments: serde_json::Value) -> Result<String>;

    fn name(&self) -> &str {
        &self.schema().name
    }

    fn description(&self) -> &str {
        &self.schema().description
    }

    fn category(&self) -> Option<ToolCategory> {
        self.schema().category
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ToolExecutor>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn ToolExecutor>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn ToolExecutor>> {
        self.tools.get(name)
    }

    pub fn list(&self) -> Vec<&ToolSchema> {
        self.tools.values().map(|t| t.schema()).collect()
    }

    pub fn schemas(&self) -> Vec<ToolSchema> {
        self.tools.values().map(|t| t.schema().clone()).collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    pub async fn execute(&self, tool_call: &ToolCall) -> ToolResult {
        let start = Instant::now();

        let Some(tool) = self.get(&tool_call.name) else {
            return ToolResult::error(
                &tool_call.id,
                format!("Tool not found: {}", tool_call.name),
                "TOOL_NOT_FOUND",
                start.elapsed().as_millis() as u64,
            );
        };

        if let Err(e) = tool.schema().validate_arguments(&tool_call.arguments) {
            return ToolResult::error(
                &tool_call.id,
                e.to_string(),
                "INVALID_ARGUMENTS",
                start.elapsed().as_millis() as u64,
            );
        }

        match tool.execute(tool_call.arguments.clone()).await {
            Ok(content) => {
                ToolResult::success(&tool_call.id, content, start.elapsed().as_millis() as u64)
            }
            Err(e) => ToolResult::error(
                &tool_call.id,
                e.to_string(),
                "EXECUTION_ERROR",
                start.elapsed().as_millis() as u64,
            ),
        }
    }

    pub async fn execute_parallel(&self, tool_calls: &[ToolCall]) -> Vec<ToolResult> {
        let futures: Vec<_> = tool_calls.iter().map(|call| self.execute(call)).collect();

        futures::future::join_all(futures).await
    }
}

pub struct FnTool<F>
where
    F: Fn(serde_json::Value) -> futures::future::BoxFuture<'static, Result<String>>
        + Send
        + Sync
        + 'static,
{
    schema: ToolSchema,
    func: F,
}

impl<F> FnTool<F>
where
    F: Fn(serde_json::Value) -> futures::future::BoxFuture<'static, Result<String>>
        + Send
        + Sync
        + 'static,
{
    pub fn new(schema: ToolSchema, func: F) -> Self {
        Self { schema, func }
    }
}

#[async_trait]
impl<F> ToolExecutor for FnTool<F>
where
    F: Fn(serde_json::Value) -> futures::future::BoxFuture<'static, Result<String>>
        + Send
        + Sync
        + 'static,
{
    fn schema(&self) -> &ToolSchema {
        &self.schema
    }

    async fn execute(&self, arguments: serde_json::Value) -> Result<String> {
        (self.func)(arguments).await
    }
}

#[macro_export]
macro_rules! tool {
    ($name:expr, $desc:expr, $params:expr, $handler:expr) => {{
        use std::sync::Arc;
        Arc::new($crate::tool::FnTool::new(
            $crate::tool::ToolSchema::new($name, $desc).with_parameters($params),
            |args| Box::pin(async move { $handler(args) }),
        ))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool {
        schema: ToolSchema,
    }

    impl EchoTool {
        fn new() -> Self {
            Self {
                schema: ToolSchema::new("echo", "Echoes input")
                    .with_parameters(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "message": {"type": "string"}
                        },
                        "required": ["message"]
                    }))
                    .with_category(ToolCategory::ReadOnly),
            }
        }
    }

    #[async_trait]
    impl ToolExecutor for EchoTool {
        fn schema(&self) -> &ToolSchema {
            &self.schema
        }

        async fn execute(&self, arguments: serde_json::Value) -> Result<String> {
            let msg = arguments
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("no message");
            Ok(msg.to_string())
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool::new()));

        assert!(registry.contains("echo"));
        assert!(!registry.contains("other"));

        let schemas = registry.schemas();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "echo");
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool::new()));

        let call = ToolCall::new("echo", serde_json::json!({"message": "hello"}));
        let result = registry.execute(&call).await;

        assert!(result.success);
        assert_eq!(result.content, "hello");
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let registry = ToolRegistry::new();
        let call = ToolCall::new("unknown", serde_json::json!({}));
        let result = registry.execute(&call).await;

        assert!(!result.success);
        assert_eq!(result.error_code, Some("TOOL_NOT_FOUND".to_string()));
    }

    #[tokio::test]
    async fn test_missing_required_parameter() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool::new()));

        let call = ToolCall::new("echo", serde_json::json!({}));
        let result = registry.execute(&call).await;

        assert!(!result.success);
        assert_eq!(result.error_code, Some("INVALID_ARGUMENTS".to_string()));
    }

    #[test]
    fn test_tool_category() {
        let tool = EchoTool::new();
        assert_eq!(tool.category(), Some(ToolCategory::ReadOnly));
    }
}
