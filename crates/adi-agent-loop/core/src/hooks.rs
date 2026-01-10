use crate::error::Result;
use crate::types::{Message, ToolCall, ToolResult};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    PreToolCall,
    PostToolCall,
    OnError,
    OnMessage,
    OnIteration,
    OnComplete,
}

#[async_trait]
pub trait Hook: Send + Sync {
    fn event(&self) -> HookEvent;

    async fn on_pre_tool_call(&self, _tool_call: &ToolCall) -> Result<Option<ToolCall>> {
        Ok(None)
    }

    async fn on_post_tool_call(&self, _tool_call: &ToolCall, _result: &ToolResult) -> Result<()> {
        Ok(())
    }

    async fn on_error(&self, _error: &str, _tool_call: Option<&ToolCall>) -> Result<()> {
        Ok(())
    }

    async fn on_message(&self, _message: &Message) -> Result<()> {
        Ok(())
    }

    async fn on_iteration(&self, _iteration: usize) -> Result<()> {
        Ok(())
    }

    async fn on_complete(&self, _messages: &[Message]) -> Result<()> {
        Ok(())
    }
}

pub struct HookManager {
    hooks: Vec<Arc<dyn Hook>>,
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HookManager {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register(&mut self, hook: Arc<dyn Hook>) {
        self.hooks.push(hook);
    }

    pub async fn fire_pre_tool_call(&self, tool_call: &ToolCall) -> Result<Option<ToolCall>> {
        for hook in &self.hooks {
            if hook.event() == HookEvent::PreToolCall {
                if let Some(modified) = hook.on_pre_tool_call(tool_call).await? {
                    return Ok(Some(modified));
                }
            }
        }
        Ok(None)
    }

    pub async fn fire_post_tool_call(
        &self,
        tool_call: &ToolCall,
        result: &ToolResult,
    ) -> Result<()> {
        for hook in &self.hooks {
            if hook.event() == HookEvent::PostToolCall {
                hook.on_post_tool_call(tool_call, result).await?;
            }
        }
        Ok(())
    }

    pub async fn fire_error(&self, error: &str, tool_call: Option<&ToolCall>) -> Result<()> {
        for hook in &self.hooks {
            if hook.event() == HookEvent::OnError {
                hook.on_error(error, tool_call).await?;
            }
        }
        Ok(())
    }

    pub async fn fire_message(&self, message: &Message) -> Result<()> {
        for hook in &self.hooks {
            if hook.event() == HookEvent::OnMessage {
                hook.on_message(message).await?;
            }
        }
        Ok(())
    }

    pub async fn fire_iteration(&self, iteration: usize) -> Result<()> {
        for hook in &self.hooks {
            if hook.event() == HookEvent::OnIteration {
                hook.on_iteration(iteration).await?;
            }
        }
        Ok(())
    }

    pub async fn fire_complete(&self, messages: &[Message]) -> Result<()> {
        for hook in &self.hooks {
            if hook.event() == HookEvent::OnComplete {
                hook.on_complete(messages).await?;
            }
        }
        Ok(())
    }
}

pub struct LoggingHook {
    event: HookEvent,
}

impl LoggingHook {
    pub fn new(event: HookEvent) -> Self {
        Self { event }
    }
}

#[async_trait]
impl Hook for LoggingHook {
    fn event(&self) -> HookEvent {
        self.event
    }

    async fn on_pre_tool_call(&self, tool_call: &ToolCall) -> Result<Option<ToolCall>> {
        tracing::info!(tool = %tool_call.name, id = %tool_call.id, "Pre tool call");
        Ok(None)
    }

    async fn on_post_tool_call(&self, tool_call: &ToolCall, result: &ToolResult) -> Result<()> {
        tracing::info!(
            tool = %tool_call.name,
            success = result.success,
            duration_ms = result.duration_ms,
            "Post tool call"
        );
        Ok(())
    }

    async fn on_error(&self, error: &str, tool_call: Option<&ToolCall>) -> Result<()> {
        tracing::error!(
            error = %error,
            tool = tool_call.map(|t| t.name.as_str()),
            "Error occurred"
        );
        Ok(())
    }

    async fn on_message(&self, message: &Message) -> Result<()> {
        tracing::debug!(role = %message.role(), "New message");
        Ok(())
    }

    async fn on_iteration(&self, iteration: usize) -> Result<()> {
        tracing::debug!(iteration, "Starting iteration");
        Ok(())
    }

    async fn on_complete(&self, messages: &[Message]) -> Result<()> {
        tracing::info!(total_messages = messages.len(), "Agent loop complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingHook {
        event: HookEvent,
        count: AtomicUsize,
    }

    impl CountingHook {
        fn new(event: HookEvent) -> Self {
            Self {
                event,
                count: AtomicUsize::new(0),
            }
        }

        fn count(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Hook for CountingHook {
        fn event(&self) -> HookEvent {
            self.event
        }

        async fn on_iteration(&self, _iteration: usize) -> Result<()> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_hook_manager() {
        let mut manager = HookManager::new();
        let hook = Arc::new(CountingHook::new(HookEvent::OnIteration));
        manager.register(hook.clone());

        manager.fire_iteration(1).await.unwrap();
        manager.fire_iteration(2).await.unwrap();

        assert_eq!(hook.count(), 2);
    }
}
