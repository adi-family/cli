//! OpenAI GPT provider implementation
//!
//! TODO: Complete implementation - API calls need to match lib-client-openai interface

use async_trait::async_trait;

use crate::error::{AgentError, Result};
use crate::llm::{LlmConfig, LlmProvider, LlmResponse};
use crate::tool::ToolSchema;
use crate::types::Message;

pub struct OpenAiProvider {
    _api_key: String,
    _model: String,
    _base_url: Option<String>,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String, base_url: Option<String>) -> Result<Self> {
        Ok(Self {
            _api_key: api_key,
            _model: model,
            _base_url: base_url,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolSchema],
        _config: &LlmConfig,
    ) -> Result<LlmResponse> {
        Err(AgentError::OpenAiError(
            "OpenAI provider not yet fully implemented. See providers/openai.rs TODO".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
