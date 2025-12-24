//! Fallback provider that tries providers in order until one succeeds.

use adi_agent_loop_core::error::{AgentError, Result};
use adi_agent_loop_core::llm::{LlmConfig, LlmProvider, LlmResponse};
use adi_agent_loop_core::tool::ToolSchema;
use adi_agent_loop_core::types::Message;
use async_trait::async_trait;
use std::sync::Arc;

/// A provider that tries multiple providers in order, falling back on failure.
///
/// # Example
/// ```ignore
/// let fallback = FallbackProvider::new(vec![
///     Arc::new(primary_provider),
///     Arc::new(backup_provider),
/// ]);
/// ```
pub struct FallbackProvider {
    providers: Vec<Arc<dyn LlmProvider>>,
    max_retries_per_provider: usize,
}

impl FallbackProvider {
    /// Create a new fallback provider with the given providers.
    /// Providers are tried in order.
    pub fn new(providers: Vec<Arc<dyn LlmProvider>>) -> Self {
        Self {
            providers,
            max_retries_per_provider: 1,
        }
    }

    /// Set the maximum retries per provider before moving to the next.
    pub fn with_retries(mut self, retries: usize) -> Self {
        self.max_retries_per_provider = retries;
        self
    }

    /// Add a provider to the fallback chain.
    pub fn add_provider(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.providers.push(provider);
        self
    }
}

#[async_trait]
impl LlmProvider for FallbackProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &LlmConfig,
    ) -> Result<LlmResponse> {
        if self.providers.is_empty() {
            return Err(AgentError::Internal("No providers configured".into()));
        }

        let mut last_error = None;

        for (idx, provider) in self.providers.iter().enumerate() {
            for attempt in 0..self.max_retries_per_provider {
                tracing::debug!(
                    provider = %provider.name(),
                    attempt = attempt + 1,
                    provider_idx = idx,
                    "Attempting request"
                );

                match provider.complete(messages, tools, config).await {
                    Ok(response) => {
                        if idx > 0 || attempt > 0 {
                            tracing::info!(
                                provider = %provider.name(),
                                fallback_idx = idx,
                                "Request succeeded after fallback"
                            );
                        }
                        return Ok(response);
                    }
                    Err(e) => {
                        tracing::warn!(
                            provider = %provider.name(),
                            error = %e,
                            attempt = attempt + 1,
                            "Provider failed"
                        );
                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AgentError::LlmError("All providers failed".into())))
    }

    fn name(&self) -> &str {
        "fallback"
    }

    fn supports_tools(&self) -> bool {
        self.providers.iter().any(|p| p.supports_tools())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adi_agent_loop_core::llm::MockLlmProvider;

    struct FailingProvider;

    #[async_trait]
    impl LlmProvider for FailingProvider {
        async fn complete(
            &self,
            _messages: &[Message],
            _tools: &[ToolSchema],
            _config: &LlmConfig,
        ) -> Result<LlmResponse> {
            Err(AgentError::LlmError("Always fails".into()))
        }

        fn name(&self) -> &str {
            "failing"
        }
    }

    #[tokio::test]
    async fn test_fallback_first_succeeds() {
        let primary = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Primary response",
        )]));
        let backup = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Backup response",
        )]));

        let fallback = FallbackProvider::new(vec![primary, backup]);

        let response = fallback
            .complete(&[], &[], &LlmConfig::default())
            .await
            .unwrap();

        assert!(matches!(
            response.message,
            Message::Assistant { content: Some(ref c), .. } if c == "Primary response"
        ));
    }

    #[tokio::test]
    async fn test_fallback_to_backup() {
        let primary: Arc<dyn LlmProvider> = Arc::new(FailingProvider);
        let backup = Arc::new(MockLlmProvider::with_responses(vec![Message::assistant(
            "Backup response",
        )]));

        let fallback = FallbackProvider::new(vec![primary, backup]);

        let response = fallback
            .complete(&[], &[], &LlmConfig::default())
            .await
            .unwrap();

        assert!(matches!(
            response.message,
            Message::Assistant { content: Some(ref c), .. } if c == "Backup response"
        ));
    }

    #[tokio::test]
    async fn test_all_fail() {
        let p1: Arc<dyn LlmProvider> = Arc::new(FailingProvider);
        let p2: Arc<dyn LlmProvider> = Arc::new(FailingProvider);

        let fallback = FallbackProvider::new(vec![p1, p2]);

        let result = fallback.complete(&[], &[], &LlmConfig::default()).await;

        assert!(result.is_err());
    }
}
