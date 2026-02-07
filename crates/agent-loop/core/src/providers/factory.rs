//! Provider factory for creating LLM providers

use crate::error::{AgentError, Result};
use crate::llm::{LlmProvider, MockLlmProvider};
use crate::types::Message;
use std::env;
use std::sync::Arc;

use super::{AnthropicProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider};

/// Provider configuration
#[derive(Debug, Clone)]
pub enum ProviderConfig {
    /// Anthropic Claude provider
    Anthropic { api_key: String, model: String },

    /// OpenAI GPT provider
    OpenAi {
        api_key: String,
        model: String,
        base_url: Option<String>,
    },

    /// OpenRouter provider
    OpenRouter {
        api_key: String,
        model: String,
        site_name: Option<String>,
    },

    /// Ollama local provider
    Ollama { host: Option<String>, model: String },

    /// Mock provider for testing
    Mock { responses: Vec<Message> },
}

impl ProviderConfig {
    /// Create from environment variables
    pub fn from_env(provider: &str, model: &str) -> Result<Self> {
        match provider.to_lowercase().as_str() {
            "anthropic" => Ok(Self::Anthropic {
                api_key: api_key_from_env("ANTHROPIC_API_KEY")?,
                model: model.to_string(),
            }),
            "openai" => Ok(Self::OpenAi {
                api_key: api_key_from_env("OPENAI_API_KEY")?,
                model: model.to_string(),
                base_url: env::var("OPENAI_BASE_URL").ok(),
            }),
            "openrouter" => Ok(Self::OpenRouter {
                api_key: api_key_from_env("OPENROUTER_API_KEY")?,
                model: model.to_string(),
                site_name: env::var("OPENROUTER_SITE_NAME").ok(),
            }),
            "ollama" => Ok(Self::Ollama {
                host: env::var("OLLAMA_HOST").ok(),
                model: model.to_string(),
            }),
            _ => Err(AgentError::ProviderConfig(format!(
                "Unknown provider: {}",
                provider
            ))),
        }
    }
}

/// Get API key from environment variable
fn api_key_from_env(var_name: &str) -> Result<String> {
    env::var(var_name).map_err(|_| AgentError::ApiKeyMissing(var_name.to_string()))
}

/// Create a provider from configuration
pub fn create_provider(config: ProviderConfig) -> Result<Arc<dyn LlmProvider>> {
    match config {
        ProviderConfig::Anthropic { api_key, model } => {
            Ok(Arc::new(AnthropicProvider::new(api_key, model)?))
        }
        ProviderConfig::OpenAi {
            api_key,
            model,
            base_url,
        } => Ok(Arc::new(OpenAiProvider::new(api_key, model, base_url)?)),
        ProviderConfig::OpenRouter {
            api_key,
            model,
            site_name,
        } => Ok(Arc::new(OpenRouterProvider::new(
            api_key, model, site_name,
        )?)),
        ProviderConfig::Ollama { host, model } => Ok(Arc::new(OllamaProvider::new(host, model)?)),
        ProviderConfig::Mock { responses } => {
            Ok(Arc::new(MockLlmProvider::with_responses(responses)))
        }
    }
}
