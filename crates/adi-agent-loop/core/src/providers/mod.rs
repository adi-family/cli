//! LLM provider implementations

use crate::error::{AgentError, Result};
use crate::llm::LlmProvider;
use std::sync::Arc;

pub mod anthropic;
pub mod factory;
pub mod openai;
pub mod openrouter;
pub mod ollama;

pub use anthropic::AnthropicProvider;
pub use factory::{create_provider, ProviderConfig};
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
pub use ollama::OllamaProvider;
