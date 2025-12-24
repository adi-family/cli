//! Load-balanced router that distributes requests across providers.

use adi_agent_loop_core::error::{AgentError, Result};
use adi_agent_loop_core::llm::{LlmConfig, LlmProvider, LlmResponse};
use adi_agent_loop_core::tool::ToolSchema;
use adi_agent_loop_core::types::Message;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Load balancing strategy.
#[derive(Debug, Clone, Copy, Default)]
pub enum Strategy {
    /// Round-robin distribution
    #[default]
    RoundRobin,
    /// Random selection
    Random,
    /// Weighted round-robin based on provider weights
    Weighted,
}

/// A provider with an optional weight for weighted load balancing.
pub struct WeightedProvider {
    pub provider: Arc<dyn LlmProvider>,
    pub weight: usize,
}

impl WeightedProvider {
    pub fn new(provider: Arc<dyn LlmProvider>, weight: usize) -> Self {
        Self { provider, weight }
    }
}

/// A router that distributes requests across multiple providers.
///
/// # Example
/// ```ignore
/// let router = LoadBalancedRouter::new()
///     .add_provider(provider1)
///     .add_provider(provider2)
///     .with_strategy(Strategy::RoundRobin);
/// ```
pub struct LoadBalancedRouter {
    providers: Vec<WeightedProvider>,
    strategy: Strategy,
    counter: AtomicUsize,
    total_weight: usize,
}

impl LoadBalancedRouter {
    /// Create a new load-balanced router.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            strategy: Strategy::RoundRobin,
            counter: AtomicUsize::new(0),
            total_weight: 0,
        }
    }

    /// Add a provider with default weight of 1.
    pub fn add_provider(self, provider: Arc<dyn LlmProvider>) -> Self {
        self.add_weighted_provider(provider, 1)
    }

    /// Add a provider with a specific weight.
    pub fn add_weighted_provider(mut self, provider: Arc<dyn LlmProvider>, weight: usize) -> Self {
        self.total_weight += weight;
        self.providers.push(WeightedProvider::new(provider, weight));
        self
    }

    /// Set the load balancing strategy.
    pub fn with_strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = strategy;
        self
    }

    fn select_provider(&self) -> Option<&Arc<dyn LlmProvider>> {
        if self.providers.is_empty() {
            return None;
        }

        match self.strategy {
            Strategy::RoundRobin => {
                let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.providers.len();
                Some(&self.providers[idx].provider)
            }
            Strategy::Random => {
                let idx = fastrand::usize(..self.providers.len());
                Some(&self.providers[idx].provider)
            }
            Strategy::Weighted => {
                if self.total_weight == 0 {
                    return self.providers.first().map(|p| &p.provider);
                }

                let counter = self.counter.fetch_add(1, Ordering::Relaxed);
                let target = counter % self.total_weight;

                let mut cumulative = 0;
                for wp in &self.providers {
                    cumulative += wp.weight;
                    if target < cumulative {
                        return Some(&wp.provider);
                    }
                }

                self.providers.last().map(|p| &p.provider)
            }
        }
    }

    /// Get the number of providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

impl Default for LoadBalancedRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for LoadBalancedRouter {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &LlmConfig,
    ) -> Result<LlmResponse> {
        let provider = self
            .select_provider()
            .ok_or_else(|| AgentError::Internal("No providers configured".into()))?;

        tracing::debug!(
            provider = %provider.name(),
            strategy = ?self.strategy,
            "Load balanced routing"
        );

        provider.complete(messages, tools, config).await
    }

    fn name(&self) -> &str {
        "load_balanced"
    }

    fn supports_tools(&self) -> bool {
        self.providers.iter().any(|p| p.provider.supports_tools())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adi_agent_loop_core::llm::MockLlmProvider;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_round_robin() {
        let p1 = Arc::new(MockLlmProvider::with_responses(
            (0..10).map(|_| Message::assistant("P1")).collect(),
        ));
        let p2 = Arc::new(MockLlmProvider::with_responses(
            (0..10).map(|_| Message::assistant("P2")).collect(),
        ));

        let router = LoadBalancedRouter::new()
            .add_provider(p1)
            .add_provider(p2)
            .with_strategy(Strategy::RoundRobin);

        let mut responses = Vec::new();
        for _ in 0..4 {
            let r = router
                .complete(&[], &[], &LlmConfig::default())
                .await
                .unwrap();
            if let Message::Assistant {
                content: Some(c), ..
            } = r.message
            {
                responses.push(c);
            }
        }

        // Should alternate between P1 and P2
        assert_eq!(responses, vec!["P1", "P2", "P1", "P2"]);
    }

    #[tokio::test]
    async fn test_weighted() {
        // P1 has weight 3, P2 has weight 1
        // So P1 should be selected 3x as often
        let p1 = Arc::new(MockLlmProvider::with_responses(
            (0..100).map(|_| Message::assistant("P1")).collect(),
        ));
        let p2 = Arc::new(MockLlmProvider::with_responses(
            (0..100).map(|_| Message::assistant("P2")).collect(),
        ));

        let router = LoadBalancedRouter::new()
            .add_weighted_provider(p1, 3)
            .add_weighted_provider(p2, 1)
            .with_strategy(Strategy::Weighted);

        let mut counts: HashMap<String, usize> = HashMap::new();
        for _ in 0..40 {
            let r = router
                .complete(&[], &[], &LlmConfig::default())
                .await
                .unwrap();
            if let Message::Assistant {
                content: Some(c), ..
            } = r.message
            {
                *counts.entry(c).or_default() += 1;
            }
        }

        // P1 should have ~30 hits, P2 ~10 (3:1 ratio)
        let p1_count = counts.get("P1").copied().unwrap_or(0);
        let p2_count = counts.get("P2").copied().unwrap_or(0);

        assert_eq!(p1_count, 30);
        assert_eq!(p2_count, 10);
    }

    #[tokio::test]
    async fn test_empty_router() {
        let router = LoadBalancedRouter::new();

        let result = router.complete(&[], &[], &LlmConfig::default()).await;

        assert!(result.is_err());
    }
}
