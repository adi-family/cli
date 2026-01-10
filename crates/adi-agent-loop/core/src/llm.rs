use crate::error::Result;
use crate::tool::ToolSchema;
use crate::types::Message;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: 0.0,
            max_tokens: 8192,
            top_p: None,
            stop_sequences: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub message: Message,
    pub usage: TokenUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &LlmConfig,
    ) -> Result<LlmResponse>;

    fn name(&self) -> &str;

    fn supports_tools(&self) -> bool {
        true
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.len() / 4
    }
}

pub struct MockLlmProvider {
    responses: std::sync::Mutex<Vec<Message>>,
}

impl MockLlmProvider {
    pub fn new() -> Self {
        Self {
            responses: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn with_responses(responses: Vec<Message>) -> Self {
        Self {
            responses: std::sync::Mutex::new(responses),
        }
    }

    pub fn add_response(&self, message: Message) {
        self.responses.lock().unwrap().push(message);
    }
}

impl Default for MockLlmProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolSchema],
        _config: &LlmConfig,
    ) -> Result<LlmResponse> {
        let message = self
            .responses
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(|| Message::assistant("I don't know how to help with that."));

        Ok(LlmResponse {
            message,
            usage: TokenUsage::default(),
            stop_reason: Some("end_turn".to_string()),
        })
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolCall;

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockLlmProvider::with_responses(vec![Message::assistant("Hello!")]);

        let response = provider
            .complete(&[], &[], &LlmConfig::default())
            .await
            .unwrap();

        assert!(matches!(response.message, Message::Assistant { .. }));
    }

    #[tokio::test]
    async fn test_mock_provider_with_tools() {
        let tool_call = ToolCall::new("read_file", serde_json::json!({"path": "/test.txt"}));
        let provider =
            MockLlmProvider::with_responses(vec![Message::assistant_with_tools(vec![tool_call])]);

        let response = provider
            .complete(&[], &[], &LlmConfig::default())
            .await
            .unwrap();

        match response.message {
            Message::Assistant { tool_calls, .. } => {
                assert!(tool_calls.is_some());
            }
            _ => panic!("Expected assistant message with tools"),
        }
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.temperature, 0.0);
        assert_eq!(config.max_tokens, 8192);
    }
}
