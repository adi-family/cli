//! LLM provider implementations

use crate::error::{AgentError, Result};
use crate::llm::LlmProvider;
use std::sync::Arc;

pub mod anthropic;
pub mod factory;
pub mod ollama;
pub mod openai;
pub mod openrouter;

pub use anthropic::AnthropicProvider;
pub use factory::{create_provider, ProviderConfig};
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
