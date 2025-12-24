//! LLM provider implementations and routing for adi-agent-loop-core.
//!
//! This crate provides concrete implementations of the `LlmProvider` trait
//! for various LLM services, plus routing/orchestration patterns.

mod providers;
mod router;

pub use providers::claude::ClaudeProvider;
pub use providers::ollama::OllamaProvider;
pub use providers::openai::OpenAiProvider;

pub use router::configurable::RouterProvider;
pub use router::cost_aware::CostAwareRouter;
pub use router::fallback::FallbackProvider;
pub use router::load_balanced::LoadBalancedRouter;

pub use adi_agent_loop_core::llm::{LlmConfig, LlmProvider, LlmResponse, TokenUsage};
