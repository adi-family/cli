//! LLM Provider adapters for proxying requests.

pub mod anthropic;
pub mod custom;
pub mod openai;
pub mod traits;

pub use anthropic::AnthropicProvider;
pub use custom::CustomProvider;
pub use openai::OpenAIProvider;
pub use traits::{LlmProvider, ProviderError};

use crate::types::ProviderType;

/// Create a provider instance based on type.
pub fn create_provider(
    provider_type: ProviderType,
    base_url: Option<String>,
) -> Box<dyn LlmProvider> {
    match provider_type {
        ProviderType::OpenAI => Box::new(OpenAIProvider::new(base_url)),
        ProviderType::OpenRouter => Box::new(OpenAIProvider::openrouter(base_url)),
        ProviderType::Anthropic => Box::new(AnthropicProvider::new(base_url)),
        ProviderType::Custom => Box::new(CustomProvider::new(
            base_url.unwrap_or_else(|| "http://localhost:8080".to_string()),
        )),
    }
}
