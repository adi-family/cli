pub mod agent;
pub mod context;
pub mod error;
pub mod hooks;
pub mod llm;
pub mod migrations;
pub mod permission;
pub mod providers;
pub mod quota;
pub mod storage;
pub mod tool;
pub mod tool_config;
pub mod types;

pub use agent::AgentLoop;
pub use context::ContextManager;
pub use error::{AgentError, Result};
pub use hooks::{Hook, HookEvent, HookManager};
pub use llm::{LlmConfig, LlmProvider, LlmResponse, MockLlmProvider, TokenUsage};
pub use permission::{
    ApprovalDecision, ApprovalHandler, AutoApprover, PermissionLevel, PermissionManager,
    PermissionRule,
};
pub use storage::{
    Session, SessionCounts, SessionId, SessionStatus, SessionStorage, SessionSummary,
    SqliteSessionStorage,
};
pub use tool::{FnTool, ToolCategory, ToolExecutor, ToolRegistry, ToolSchema};
pub use tool_config::{ToolConfig, ToolConfigSet};
pub use types::{AuditEntry, LoopConfig, LoopState, Message, ToolCall, ToolResult};
pub use quota::{QuotaCheckResult, QuotaConfig, QuotaManager, QuotaPeriod, QuotaStats, QuotaUsage};
pub use providers::{create_provider, ProviderConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct TestTool {
        schema: ToolSchema,
    }

    impl TestTool {
        fn new(name: &str, desc: &str) -> Self {
            Self {
                schema: ToolSchema::new(name, desc)
                    .with_parameters(serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    }))
                    .with_category(ToolCategory::ReadOnly),
            }
        }
    }

    #[async_trait::async_trait]
    impl ToolExecutor for TestTool {
        fn schema(&self) -> &ToolSchema {
            &self.schema
        }

        async fn execute(&self, _arguments: serde_json::Value) -> Result<String> {
            Ok("success".to_string())
        }
    }

    #[tokio::test]
    async fn test_full_agent_loop() {
        let provider = MockLlmProvider::with_responses(vec![Message::assistant(
            "Task completed successfully!",
        )]);

        let mut agent = AgentLoop::new(Arc::new(provider))
            .with_system_prompt("You are a helpful assistant.")
            .with_tool(Arc::new(TestTool::new("test", "A test tool")));

        let result = agent.run("Do something").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Task completed successfully!");
    }

    #[tokio::test]
    async fn test_agent_with_custom_config() {
        let provider = MockLlmProvider::with_responses(vec![Message::assistant("Done!")]);

        let config = LoopConfig {
            max_iterations: 10,
            max_tokens: 50_000,
            timeout_ms: 30_000,
            ..Default::default()
        };

        let mut agent = AgentLoop::new(Arc::new(provider)).with_loop_config(config);

        let result = agent.run("Test").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));

        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role(), "user");
    }

    #[test]
    fn test_tool_schema_validation() {
        let schema = ToolSchema::new("test", "Test tool").with_parameters(serde_json::json!({
            "type": "object",
            "properties": {
                "required_param": {"type": "string"}
            },
            "required": ["required_param"]
        }));

        let valid_args = serde_json::json!({"required_param": "value"});
        assert!(schema.validate_arguments(&valid_args).is_ok());

        let invalid_args = serde_json::json!({});
        assert!(schema.validate_arguments(&invalid_args).is_err());
    }

    #[test]
    fn test_permission_manager() {
        let manager = PermissionManager::with_defaults();

        let (level, _) = manager.check("read_file", &serde_json::json!({}), None);
        assert_eq!(level, PermissionLevel::Auto);

        let (level, _) = manager.check("write_file", &serde_json::json!({}), None);
        assert_eq!(level, PermissionLevel::Ask);
    }

    #[test]
    fn test_context_truncation() {
        let long_content = "x".repeat(100_000);
        let truncated = ContextManager::truncate_tool_result(&long_content, 1000);
        assert!(truncated.len() < long_content.len());
        assert!(truncated.contains("truncated"));
    }
}
