//! LLM Provider adapters for proxying requests.

pub mod anthropic;
pub mod openai;
pub mod traits;

pub use anthropic::AnthropicProvider;
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
        ProviderType::Custom => Box::new(OpenAIProvider::custom(
            base_url.unwrap_or_else(|| "http://localhost:8080".to_string()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_provider_returns_correct_type() {
        let openai = create_provider(ProviderType::OpenAI, None);
        assert_eq!(openai.provider_type(), ProviderType::OpenAI);

        let anthropic = create_provider(ProviderType::Anthropic, None);
        assert_eq!(anthropic.provider_type(), ProviderType::Anthropic);

        let openrouter = create_provider(ProviderType::OpenRouter, None);
        assert_eq!(openrouter.provider_type(), ProviderType::OpenRouter);

        let custom = create_provider(ProviderType::Custom, None);
        assert_eq!(custom.provider_type(), ProviderType::Custom);
    }

    #[test]
    fn test_create_provider_custom_base_url() {
        let provider = create_provider(ProviderType::OpenAI, Some("https://my-proxy.com".into()));
        assert_eq!(provider.base_url(), "https://my-proxy.com");
    }

    #[test]
    fn test_create_provider_custom_default_url() {
        let provider = create_provider(ProviderType::Custom, None);
        assert_eq!(provider.base_url(), "http://localhost:8080");
    }
}
