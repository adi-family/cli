//! Anthropic Claude provider implementation
//!
//! TODO: Complete implementation - API calls need to match lib-client-anthropic interface

use async_trait::async_trait;

use crate::error::{AgentError, Result};
use crate::llm::{LlmConfig, LlmProvider, LlmResponse, TokenUsage};
use crate::tool::ToolSchema;
use crate::types::Message;

pub struct AnthropicProvider {
    _api_key: String,
    _model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        Ok(Self {
            _api_key: api_key,
            _model: model,
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolSchema],
        _config: &LlmConfig,
    ) -> Result<LlmResponse> {
        Err(AgentError::AnthropicError(
            "Anthropic provider not yet fully implemented. See providers/anthropic.rs TODO"
                .to_string(),
        ))
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
