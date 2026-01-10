use crate::context::ContextManager;
use crate::error::{AgentError, Result};
use crate::hooks::HookManager;
use crate::llm::{LlmConfig, LlmProvider};
use crate::permission::{ApprovalDecision, ApprovalHandler, PermissionLevel, PermissionManager};
use crate::tool::{ToolExecutor, ToolRegistry};
use crate::types::{LoopConfig, LoopState, Message, ToolCall};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

pub struct AgentLoop {
    llm: Arc<dyn LlmProvider>,
    tools: ToolRegistry,
    permissions: PermissionManager,
    hooks: HookManager,
    context: ContextManager,
    loop_config: LoopConfig,
    llm_config: LlmConfig,
    system_prompt: Option<String>,
    interrupt_rx: Option<mpsc::Receiver<()>>,
}

impl AgentLoop {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        let loop_config = LoopConfig::default();
        Self {
            llm,
            tools: ToolRegistry::new(),
            permissions: PermissionManager::with_defaults(),
            hooks: HookManager::new(),
            context: ContextManager::new(loop_config.clone()),
            loop_config,
            llm_config: LlmConfig::default(),
            system_prompt: None,
            interrupt_rx: None,
        }
    }

    pub fn with_tool(mut self, tool: Arc<dyn ToolExecutor>) -> Self {
        self.tools.register(tool);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Arc<dyn ToolExecutor>>) -> Self {
        for tool in tools {
            self.tools.register(tool);
        }
        self
    }

    pub fn with_permissions(mut self, permissions: PermissionManager) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_hooks(mut self, hooks: HookManager) -> Self {
        self.hooks = hooks;
        self
    }

    pub fn with_loop_config(mut self, config: LoopConfig) -> Self {
        self.context = ContextManager::new(config.clone());
        self.loop_config = config;
        self
    }

    pub fn with_llm_config(mut self, config: LlmConfig) -> Self {
        self.llm_config = config;
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_interrupt_channel(mut self, rx: mpsc::Receiver<()>) -> Self {
        self.interrupt_rx = Some(rx);
        self
    }

    pub async fn run(&mut self, user_input: impl Into<String>) -> Result<String> {
        self.run_with_approval(&crate::permission::AutoApprover, user_input)
            .await
    }

    pub async fn run_with_approval<A: ApprovalHandler>(
        &mut self,
        approver: &A,
        user_input: impl Into<String>,
    ) -> Result<String> {
        let mut state = LoopState::default();

        if let Some(system_prompt) = &self.system_prompt {
            self.context.add_message(Message::system(system_prompt));
        }

        self.context.add_message(Message::user(user_input));

        loop {
            if state.iteration >= self.loop_config.max_iterations {
                return Err(AgentError::MaxIterationsReached(state.iteration));
            }

            if self.context.total_tokens() > self.loop_config.max_tokens {
                return Err(AgentError::TokenLimitExceeded {
                    used: self.context.total_tokens(),
                    limit: self.loop_config.max_tokens,
                });
            }

            if let Some(ref mut rx) = self.interrupt_rx {
                if rx.try_recv().is_ok() {
                    return Err(AgentError::UserAborted);
                }
            }

            self.hooks.fire_iteration(state.iteration).await?;
            state.iteration += 1;
            state.last_activity = Utc::now();

            self.context.compact_if_needed();

            let response = timeout(
                Duration::from_millis(self.loop_config.timeout_ms),
                self.llm.complete(
                    self.context.messages(),
                    &self.tools.schemas(),
                    &self.llm_config,
                ),
            )
            .await
            .map_err(|_| AgentError::Timeout(self.loop_config.timeout_ms))??;

            state.total_tokens += response.usage.total_tokens;

            self.context.add_message(response.message.clone());
            self.hooks.fire_message(&response.message).await?;

            match response.message {
                Message::Assistant {
                    content: Some(text),
                    tool_calls: None,
                    ..
                } => {
                    self.hooks.fire_complete(self.context.messages()).await?;
                    return Ok(text);
                }

                Message::Assistant {
                    tool_calls: Some(tool_calls),
                    ..
                } => {
                    let results = self
                        .execute_tool_calls(&tool_calls, approver, &mut state)
                        .await?;

                    for result in results {
                        let message = result.to_message();
                        self.context.add_message(message.clone());
                        self.hooks.fire_message(&message).await?;
                    }
                }

                _ => {
                    return Err(AgentError::Internal(
                        "Unexpected message type from LLM".to_string(),
                    ));
                }
            }
        }
    }

    async fn execute_tool_calls<A: ApprovalHandler>(
        &mut self,
        tool_calls: &[ToolCall],
        approver: &A,
        state: &mut LoopState,
    ) -> Result<Vec<crate::types::ToolResult>> {
        let mut results = Vec::new();

        let can_parallelize = self.can_parallelize_calls(tool_calls);
        let max_parallel = self.loop_config.max_parallel_tools.min(tool_calls.len());

        if can_parallelize && tool_calls.len() > 1 {
            let mut approved_calls = Vec::new();

            for tool_call in tool_calls {
                match self.check_and_approve_call(tool_call, approver).await? {
                    Some(call) => approved_calls.push(call),
                    None => {
                        results.push(crate::types::ToolResult::error(
                            &tool_call.id,
                            "Permission denied",
                            "PERMISSION_DENIED",
                            0,
                        ));
                    }
                }
            }

            for chunk in approved_calls.chunks(max_parallel) {
                let chunk_results = self.tools.execute_parallel(chunk).await;
                for result in chunk_results {
                    state.tool_calls_count += 1;
                    if !result.success {
                        state.errors_count += 1;
                    }
                    results.push(result);
                }
            }
        } else {
            for tool_call in tool_calls {
                let approved_call = match self.check_and_approve_call(tool_call, approver).await? {
                    Some(call) => call,
                    None => {
                        results.push(crate::types::ToolResult::error(
                            &tool_call.id,
                            "Permission denied",
                            "PERMISSION_DENIED",
                            0,
                        ));
                        continue;
                    }
                };

                self.hooks.fire_pre_tool_call(&approved_call).await?;

                let result = self.tools.execute(&approved_call).await;

                self.hooks
                    .fire_post_tool_call(&approved_call, &result)
                    .await?;

                state.tool_calls_count += 1;
                if !result.success {
                    state.errors_count += 1;
                    self.hooks
                        .fire_error(&result.content, Some(&approved_call))
                        .await?;
                }

                results.push(result);
            }
        }

        Ok(results)
    }

    async fn check_and_approve_call<A: ApprovalHandler>(
        &mut self,
        tool_call: &ToolCall,
        approver: &A,
    ) -> Result<Option<ToolCall>> {
        let tool = self.tools.get(&tool_call.name);
        let category = tool.and_then(|t| t.category());

        let (level, rule) = self
            .permissions
            .check(&tool_call.name, &tool_call.arguments, category);

        match level {
            PermissionLevel::Auto => {
                if let Some(modified) = self.hooks.fire_pre_tool_call(tool_call).await? {
                    Ok(Some(modified))
                } else {
                    Ok(Some(tool_call.clone()))
                }
            }

            PermissionLevel::Ask => {
                let decision = approver.request_approval(tool_call, rule).await?;

                match decision {
                    ApprovalDecision::Allow => Ok(Some(tool_call.clone())),
                    ApprovalDecision::AllowAll => {
                        let pattern = format!("tool:{}:*", tool_call.name);
                        self.permissions
                            .set_session_override(pattern, PermissionLevel::Auto);
                        Ok(Some(tool_call.clone()))
                    }
                    ApprovalDecision::Deny => Ok(None),
                    ApprovalDecision::Abort => Err(AgentError::UserAborted),
                }
            }

            PermissionLevel::Deny => {
                let pattern = rule.map(|r| r.pattern.clone()).unwrap_or_default();
                Err(AgentError::PermissionDenied {
                    tool: tool_call.name.clone(),
                    pattern,
                })
            }
        }
    }

    fn can_parallelize_calls(&self, tool_calls: &[ToolCall]) -> bool {
        if tool_calls.len() <= 1 {
            return false;
        }

        let all_read_only = tool_calls.iter().all(|call| {
            self.tools
                .get(&call.name)
                .and_then(|t| t.category())
                .map(|c| c == crate::tool::ToolCategory::ReadOnly)
                .unwrap_or(false)
        });

        all_read_only
    }

    pub fn context(&self) -> &ContextManager {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut ContextManager {
        &mut self.context
    }

    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    pub fn permissions(&self) -> &PermissionManager {
        &self.permissions
    }

    pub fn permissions_mut(&mut self) -> &mut PermissionManager {
        &mut self.permissions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmProvider;
    use crate::tool::{ToolCategory, ToolSchema};

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

    #[async_trait::async_trait]
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
    async fn test_agent_simple_response() {
        let provider =
            MockLlmProvider::with_responses(vec![Message::assistant("Hello, how can I help you?")]);

        let mut agent = AgentLoop::new(Arc::new(provider));

        let response = agent.run("Hi there!").await.unwrap();
        assert_eq!(response, "Hello, how can I help you?");
    }

    #[tokio::test]
    async fn test_agent_with_tool_call() {
        let tool_call = ToolCall::new("echo", serde_json::json!({"message": "test"}));
        let provider = MockLlmProvider::with_responses(vec![
            Message::assistant("The echo returned: test"),
            Message::assistant_with_tools(vec![tool_call]),
        ]);

        let mut agent = AgentLoop::new(Arc::new(provider)).with_tool(Arc::new(EchoTool::new()));

        let response = agent.run("Echo test please").await.unwrap();
        assert_eq!(response, "The echo returned: test");
    }

    #[tokio::test]
    async fn test_agent_max_iterations() {
        let provider = MockLlmProvider::with_responses(
            (0..100)
                .map(|_| {
                    Message::assistant_with_tools(vec![ToolCall::new(
                        "echo",
                        serde_json::json!({"message": "loop"}),
                    )])
                })
                .collect(),
        );

        let mut agent = AgentLoop::new(Arc::new(provider))
            .with_tool(Arc::new(EchoTool::new()))
            .with_loop_config(LoopConfig {
                max_iterations: 3,
                ..Default::default()
            });

        let result = agent.run("Loop forever").await;
        assert!(matches!(result, Err(AgentError::MaxIterationsReached(3))));
    }
}
