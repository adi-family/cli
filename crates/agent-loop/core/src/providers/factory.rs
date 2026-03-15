//! Provider factory for creating LLM providers

use crate::error::{AgentError, Result};
use crate::llm::{LlmProvider, MockLlmProvider};
use crate::types::Message;
use lib_env_parse::{env_vars, env_opt};
use std::sync::Arc;

use super::{
    AnthropicProvider, OllamaProvider, OpenAiProvider, OpenRouterProvider, SignalingConfig,
};

env_vars! {
    AnthropicApiKey => "ANTHROPIC_API_KEY",
    OpenaiApiKey => "OPENAI_API_KEY",
    OpenaiBaseUrl => "OPENAI_BASE_URL",
    OpenrouterApiKey => "OPENROUTER_API_KEY",
    OpenrouterSiteName => "OPENROUTER_SITE_NAME",
    OllamaHost => "OLLAMA_HOST",
    SignalingUrl => "SIGNALING_URL",
    SignalingAccessToken => "SIGNALING_ACCESS_TOKEN",
    SignalingProxyToken => "SIGNALING_PROXY_TOKEN",
    SignalingDeviceId => "SIGNALING_DEVICE_ID",
    SignalingPairingCode => "SIGNALING_PAIRING_CODE",
}

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

    /// Signaling-based provider (routes through KAKUN signaling server to LLM proxy)
    Signaling { config: SignalingConfig },

    /// Mock provider for testing
    Mock { responses: Vec<Message> },
}

impl ProviderConfig {
    /// Create from environment variables
    pub fn from_env(provider: &str, model: &str) -> Result<Self> {
        match provider.to_lowercase().as_str() {
            "anthropic" => Ok(Self::Anthropic {
                api_key: api_key_from_env(EnvVar::AnthropicApiKey.as_str())?,
                model: model.to_string(),
            }),
            "openai" => Ok(Self::OpenAi {
                api_key: api_key_from_env(EnvVar::OpenaiApiKey.as_str())?,
                model: model.to_string(),
                base_url: env_opt(EnvVar::OpenaiBaseUrl.as_str()),
            }),
            "openrouter" => Ok(Self::OpenRouter {
                api_key: api_key_from_env(EnvVar::OpenrouterApiKey.as_str())?,
                model: model.to_string(),
                site_name: env_opt(EnvVar::OpenrouterSiteName.as_str()),
            }),
            "ollama" => Ok(Self::Ollama {
                host: env_opt(EnvVar::OllamaHost.as_str()),
                model: model.to_string(),
            }),
            "signaling" => {
                let signaling_url = env_opt(EnvVar::SignalingUrl.as_str())
                    .ok_or_else(|| AgentError::ProviderConfig("SIGNALING_URL required".into()))?;
                let access_token = env_opt(EnvVar::SignalingAccessToken.as_str())
                    .ok_or_else(|| {
                        AgentError::ProviderConfig("SIGNALING_ACCESS_TOKEN required".into())
                    })?;
                let proxy_token = env_opt(EnvVar::SignalingProxyToken.as_str())
                    .ok_or_else(|| {
                        AgentError::ProviderConfig("SIGNALING_PROXY_TOKEN required".into())
                    })?;
                Ok(Self::Signaling {
                    config: SignalingConfig {
                        signaling_url,
                        access_token,
                        proxy_token,
                        device_id: env_opt(EnvVar::SignalingDeviceId.as_str()),
                        pairing_code: env_opt(EnvVar::SignalingPairingCode.as_str()),
                    },
                })
            }
            _ => Err(AgentError::ProviderConfig(format!(
                "Unknown provider: {}",
                provider
            ))),
        }
    }
}

/// Get API key from environment variable
fn api_key_from_env(var_name: &str) -> Result<String> {
    env_opt(var_name).ok_or_else(|| AgentError::ApiKeyMissing(var_name.to_string()))
}

/// Create a provider from configuration.
///
/// Async because the signaling provider needs to establish a WebSocket connection.
pub async fn create_provider(config: ProviderConfig) -> Result<Arc<dyn LlmProvider>> {
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
        ProviderConfig::Signaling { config } => {
            Ok(Arc::new(super::SignalingLlmProvider::connect(config).await?))
        }
        ProviderConfig::Mock { responses } => {
            Ok(Arc::new(MockLlmProvider::with_responses(responses)))
        }
    }
}
