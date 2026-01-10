//! OpenRouter provider implementation
//!
//! TODO: Complete implementation - API calls need to match lib-client-openrouter interface

use async_trait::async_trait;

use crate::error::{AgentError, Result};
use crate::llm::{LlmConfig, LlmProvider, LlmResponse};
use crate::tool::ToolSchema;
use crate::types::Message;

pub struct OpenRouterProvider {
    _api_key: String,
    _model: String,
    _site_name: Option<String>,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, model: String, site_name: Option<String>) -> Result<Self> {
        Ok(Self {
            _api_key: api_key,
            _model: model,
            _site_name: site_name,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolSchema],
        _config: &LlmConfig,
    ) -> Result<LlmResponse> {
        Err(AgentError::OpenRouterError(
            "OpenRouter provider not yet fully implemented. See providers/openrouter.rs TODO".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "openrouter"
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
