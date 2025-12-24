//! Cost-aware router that uses cheap models for simple tasks, expensive for complex.

use adi_agent_loop_core::error::Result;
use adi_agent_loop_core::llm::{LlmConfig, LlmProvider, LlmResponse};
use adi_agent_loop_core::tool::ToolSchema;
use adi_agent_loop_core::types::Message;
use async_trait::async_trait;
use std::sync::Arc;

/// Complexity estimator function type.
pub type ComplexityFn = Box<dyn Fn(&[Message], &[ToolSchema]) -> usize + Send + Sync>;

/// A router that selects between cheap and expensive models based on task complexity.
///
/// # Example
/// ```ignore
/// let router = CostAwareRouter::new(
///     Arc::new(haiku_provider),   // cheap
///     Arc::new(opus_provider),    // expensive
/// )
/// .with_threshold(500)  // Token threshold for "complex"
/// .with_complexity_fn(|msgs, tools| {
///     // Custom complexity calculation
///     msgs.iter().map(|m| m.estimated_tokens()).sum::<usize>()
///         + tools.len() * 100
/// });
/// ```
pub struct CostAwareRouter {
    cheap: Arc<dyn LlmProvider>,
    expensive: Arc<dyn LlmProvider>,
    threshold: usize,
    complexity_fn: Option<ComplexityFn>,
    tool_weight: usize,
    force_expensive_with_tools: bool,
}

impl CostAwareRouter {
    /// Create a new cost-aware router.
    ///
    /// - `cheap`: Provider for simple tasks (e.g., Haiku, GPT-4o-mini)
    /// - `expensive`: Provider for complex tasks (e.g., Opus, GPT-4)
    pub fn new(cheap: Arc<dyn LlmProvider>, expensive: Arc<dyn LlmProvider>) -> Self {
        Self {
            cheap,
            expensive,
            threshold: 1000,
            complexity_fn: None,
            tool_weight: 50,
            force_expensive_with_tools: false,
        }
    }

    /// Set the complexity threshold. Tasks above this use the expensive model.
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set a custom complexity estimation function.
    pub fn with_complexity_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&[Message], &[ToolSchema]) -> usize + Send + Sync + 'static,
    {
        self.complexity_fn = Some(Box::new(f));
        self
    }

    /// Set weight per tool (added to complexity score).
    pub fn with_tool_weight(mut self, weight: usize) -> Self {
        self.tool_weight = weight;
        self
    }

    /// Always use expensive model when tools are provided.
    pub fn force_expensive_with_tools(mut self) -> Self {
        self.force_expensive_with_tools = true;
        self
    }

    fn estimate_complexity(&self, messages: &[Message], tools: &[ToolSchema]) -> usize {
        if let Some(f) = &self.complexity_fn {
            return f(messages, tools);
        }

        // Default complexity estimation
        let message_tokens: usize = messages.iter().map(|m| m.estimated_tokens()).sum();
        let tool_complexity = tools.len() * self.tool_weight;

        // Bonus for multi-turn conversations
        let turn_bonus = if messages.len() > 4 { 200 } else { 0 };

        // Bonus for long messages (likely complex reasoning)
        let long_message_bonus = messages
            .iter()
            .filter(|m| m.estimated_tokens() > 500)
            .count()
            * 100;

        message_tokens + tool_complexity + turn_bonus + long_message_bonus
    }
}

#[async_trait]
impl LlmProvider for CostAwareRouter {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &LlmConfig,
    ) -> Result<LlmResponse> {
        // Force expensive if tools present and flag is set
        if self.force_expensive_with_tools && !tools.is_empty() {
            tracing::debug!(
                provider = %self.expensive.name(),
                reason = "tools_present",
                "Using expensive model"
            );
            return self.expensive.complete(messages, tools, config).await;
        }

        let complexity = self.estimate_complexity(messages, tools);
        let use_expensive = complexity > self.threshold;

        let provider = if use_expensive {
            &self.expensive
        } else {
            &self.cheap
        };

        tracing::debug!(
            provider = %provider.name(),
            complexity = complexity,
            threshold = self.threshold,
            use_expensive = use_expensive,
            "Cost-aware routing decision"
        );

        provider.complete(messages, tools, config).await
    }

    fn name(&self) -> &str {
        "cost_aware"
    }

    fn supports_tools(&self) -> bool {
        self.cheap.supports_tools() || self.expensive.supports_tools()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adi_agent_loop_core::llm::MockLlmProvider;

    #[tokio::test]
    async fn test_uses_cheap_for_simple() {
        let cheap = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Cheap response",
        )]));
        let expensive = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Expensive response",
        )]));

        let router = CostAwareRouter::new(cheap, expensive).with_threshold(1000);

        let messages = vec![Message::user("Hi")]; // Very simple

        let response = router
            .complete(&messages, &[], &LlmConfig::default())
            .await
            .unwrap();

        assert!(matches!(
            response.message,
            Message::Assistant { content: Some(ref c), .. } if c == "Cheap response"
        ));
    }

    #[tokio::test]
    async fn test_uses_expensive_for_complex() {
        let cheap = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Cheap response",
        )]));
        let expensive = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Expensive response",
        )]));

        let router = CostAwareRouter::new(cheap, expensive).with_threshold(10); // Very low threshold

        let messages = vec![Message::user(
            "This is a much longer message that should trigger the expensive model",
        )];

        let response = router
            .complete(&messages, &[], &LlmConfig::default())
            .await
            .unwrap();

        assert!(matches!(
            response.message,
            Message::Assistant { content: Some(ref c), .. } if c == "Expensive response"
        ));
    }

    #[tokio::test]
    async fn test_force_expensive_with_tools() {
        let cheap = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Cheap response",
        )]));
        let expensive = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Expensive response",
        )]));

        let router = CostAwareRouter::new(cheap, expensive)
            .with_threshold(10000) // High threshold
            .force_expensive_with_tools();

        let tools = vec![adi_agent_loop_core::tool::ToolSchema::new(
            "test",
            "A test tool",
        )];

        let response = router
            .complete(&[Message::user("Hi")], &tools, &LlmConfig::default())
            .await
            .unwrap();

        assert!(matches!(
            response.message,
            Message::Assistant { content: Some(ref c), .. } if c == "Expensive response"
        ));
    }
}
