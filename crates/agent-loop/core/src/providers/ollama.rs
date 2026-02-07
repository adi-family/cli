//! Ollama local provider implementation
//!
//! TODO: Complete implementation - API calls need to match lib-client-ollama interface

use async_trait::async_trait;

use crate::error::{AgentError, Result};
use crate::llm::{LlmConfig, LlmProvider, LlmResponse};
use crate::tool::ToolSchema;
use crate::types::Message;

pub struct OllamaProvider {
    _host: Option<String>,
    _model: String,
}

impl OllamaProvider {
    pub fn new(host: Option<String>, model: String) -> Result<Self> {
        Ok(Self {
            _host: host,
            _model: model,
        })
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolSchema],
        _config: &LlmConfig,
    ) -> Result<LlmResponse> {
        Err(AgentError::OllamaError(
            "Ollama provider not yet fully implemented. See providers/ollama.rs TODO".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "ollama"
    }

    fn supports_tools(&self) -> bool {
        false // Ollama tool support varies by model
    }
}
