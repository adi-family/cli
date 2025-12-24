//! Generic router provider for custom routing logic.

use adi_agent_loop_core::error::{AgentError, Result};
use adi_agent_loop_core::llm::{LlmConfig, LlmProvider, LlmResponse};
use adi_agent_loop_core::tool::ToolSchema;
use adi_agent_loop_core::types::Message;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Router function type that selects a provider based on context.
pub type RouterFn = Box<dyn Fn(&[Message], &LlmConfig) -> String + Send + Sync>;

/// A router that directs requests to different providers based on custom logic.
///
/// # Example
/// ```ignore
/// let router = RouterProvider::new()
///     .add_provider("claude", claude_provider)
///     .add_provider("openai", openai_provider)
///     .with_router(|messages, config| {
///         if config.model.contains("claude") { "claude" }
///         else { "openai" }
///     });
/// ```
pub struct RouterProvider {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    router_fn: Option<RouterFn>,
    default_provider: Option<String>,
}

impl RouterProvider {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            router_fn: None,
            default_provider: None,
        }
    }

    /// Add a provider with a given name.
    pub fn add_provider(mut self, name: impl Into<String>, provider: Arc<dyn LlmProvider>) -> Self {
        let name = name.into();
        if self.default_provider.is_none() {
            self.default_provider = Some(name.clone());
        }
        self.providers.insert(name, provider);
        self
    }

    /// Set the routing function.
    pub fn with_router<F>(mut self, f: F) -> Self
    where
        F: Fn(&[Message], &LlmConfig) -> String + Send + Sync + 'static,
    {
        self.router_fn = Some(Box::new(f));
        self
    }

    /// Set the default provider (used when router returns unknown name).
    pub fn with_default(mut self, name: impl Into<String>) -> Self {
        self.default_provider = Some(name.into());
        self
    }

    /// Route by model name prefix (e.g., "claude-" -> "claude", "gpt-" -> "openai").
    pub fn with_model_prefix_router(self) -> Self {
        self.with_router(|_, config| {
            let model = config.model.to_lowercase();
            if model.starts_with("claude") {
                "claude".to_string()
            } else if model.starts_with("gpt") || model.starts_with("o1") || model.starts_with("o3")
            {
                "openai".to_string()
            } else if model.starts_with("llama")
                || model.starts_with("mistral")
                || model.starts_with("codellama")
                || model.starts_with("deepseek")
            {
                "ollama".to_string()
            } else {
                "default".to_string()
            }
        })
    }

    fn get_provider(&self, name: &str) -> Option<&Arc<dyn LlmProvider>> {
        self.providers.get(name).or_else(|| {
            self.default_provider
                .as_ref()
                .and_then(|d| self.providers.get(d))
        })
    }

    /// Get list of registered provider names.
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for RouterProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for RouterProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &LlmConfig,
    ) -> Result<LlmResponse> {
        let provider_name = match &self.router_fn {
            Some(f) => f(messages, config),
            None => self.default_provider.clone().ok_or_else(|| {
                AgentError::Internal("No router function or default provider".into())
            })?,
        };

        let provider = self.get_provider(&provider_name).ok_or_else(|| {
            AgentError::Internal(format!(
                "Provider '{}' not found. Available: {:?}",
                provider_name,
                self.provider_names()
            ))
        })?;

        tracing::debug!(provider = %provider_name, model = %config.model, "Routing request");

        provider.complete(messages, tools, config).await
    }

    fn name(&self) -> &str {
        "router"
    }

    fn supports_tools(&self) -> bool {
        // Assume at least one provider supports tools
        self.providers.values().any(|p| p.supports_tools())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adi_agent_loop_core::llm::MockLlmProvider;

    #[tokio::test]
    async fn test_router_with_model_prefix() {
        let claude = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "I am Claude",
        )]));
        let openai = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "I am GPT",
        )]));

        let router = RouterProvider::new()
            .add_provider("claude", claude)
            .add_provider("openai", openai)
            .with_model_prefix_router();

        // Test Claude routing
        let config = LlmConfig {
            model: "claude-sonnet-4-20250514".to_string(),
            ..Default::default()
        };
        let response = router.complete(&[], &[], &config).await.unwrap();
        assert!(
            matches!(response.message, Message::Assistant { content: Some(ref c), .. } if c == "I am Claude")
        );
    }

    #[tokio::test]
    async fn test_router_default_provider() {
        let provider = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Default response",
        )]));

        let router = RouterProvider::new()
            .add_provider("default", provider)
            .with_default("default");

        let response = router
            .complete(&[], &[], &LlmConfig::default())
            .await
            .unwrap();

        assert!(matches!(response.message, Message::Assistant { .. }));
    }
}
