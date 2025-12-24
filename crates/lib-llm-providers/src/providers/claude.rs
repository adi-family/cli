//! Anthropic Claude API provider implementation.

use adi_agent_loop_core::error::{AgentError, Result};
use adi_agent_loop_core::llm::{LlmConfig, LlmProvider, LlmResponse, TokenUsage};
use adi_agent_loop_core::tool::ToolSchema;
use adi_agent_loop_core::types::{Message, ToolCall};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Claude provider using the Anthropic Messages API.
pub struct ClaudeProvider {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
}

impl ClaudeProvider {
    /// Create a new Claude provider with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: reqwest::Client::new(),
            base_url: ANTHROPIC_API_URL.to_string(),
        }
    }

    /// Set a custom base URL (for proxies or testing).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn convert_messages(&self, messages: &[Message]) -> (Option<String>, Vec<ApiMessage>) {
        let mut system_prompt = None;
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg {
                Message::System { content, .. } => {
                    system_prompt = Some(content.clone());
                }
                Message::User { content, .. } => {
                    api_messages.push(ApiMessage {
                        role: "user".to_string(),
                        content: ApiContent::Text(content.clone()),
                    });
                }
                Message::Assistant {
                    content,
                    tool_calls,
                    ..
                } => {
                    let mut blocks = Vec::new();

                    if let Some(text) = content {
                        blocks.push(ContentBlock::Text { text: text.clone() });
                    }

                    if let Some(calls) = tool_calls {
                        for call in calls {
                            blocks.push(ContentBlock::ToolUse {
                                id: call.id.clone(),
                                name: call.name.clone(),
                                input: call.arguments.clone(),
                            });
                        }
                    }

                    if !blocks.is_empty() {
                        api_messages.push(ApiMessage {
                            role: "assistant".to_string(),
                            content: ApiContent::Blocks(blocks),
                        });
                    }
                }
                Message::Tool {
                    tool_call_id,
                    content,
                    is_error,
                    ..
                } => {
                    api_messages.push(ApiMessage {
                        role: "user".to_string(),
                        content: ApiContent::Blocks(vec![ContentBlock::ToolResult {
                            tool_use_id: tool_call_id.clone(),
                            content: content.clone(),
                            is_error: *is_error,
                        }]),
                    });
                }
            }
        }

        (system_prompt, api_messages)
    }

    fn convert_tools(&self, tools: &[ToolSchema]) -> Vec<ApiTool> {
        tools
            .iter()
            .map(|t| ApiTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.parameters.clone(),
            })
            .collect()
    }

    fn parse_response(&self, response: ApiResponse) -> Result<LlmResponse> {
        let mut text_content = None;
        let mut tool_calls = Vec::new();

        for block in response.content {
            match block {
                ContentBlock::Text { text } => {
                    text_content = Some(text);
                }
                ContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments: input,
                    });
                }
                _ => {}
            }
        }

        let message = if tool_calls.is_empty() {
            Message::Assistant {
                content: text_content,
                tool_calls: None,
                timestamp: Some(chrono::Utc::now()),
            }
        } else {
            Message::Assistant {
                content: text_content,
                tool_calls: Some(tool_calls),
                timestamp: Some(chrono::Utc::now()),
            }
        };

        Ok(LlmResponse {
            message,
            usage: TokenUsage {
                prompt_tokens: response.usage.input_tokens,
                completion_tokens: response.usage.output_tokens,
                total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            },
            stop_reason: Some(response.stop_reason.unwrap_or_default()),
        })
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &LlmConfig,
    ) -> Result<LlmResponse> {
        let (system, api_messages) = self.convert_messages(messages);
        let api_tools = self.convert_tools(tools);

        let mut request = ApiRequest {
            model: config.model.clone(),
            messages: api_messages,
            max_tokens: config.max_tokens,
            system,
            temperature: Some(config.temperature),
            top_p: config.top_p,
            stop_sequences: config.stop_sequences.clone(),
            tools: if api_tools.is_empty() {
                None
            } else {
                Some(api_tools)
            },
        };

        // Handle models that don't support temperature
        if config.model.contains("o1") || config.model.contains("o3") {
            request.temperature = None;
        }

        let response = self
            .client
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AgentError::LlmError(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| AgentError::LlmError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(AgentError::LlmError(format!(
                "API error ({}): {}",
                status, body
            )));
        }

        let api_response: ApiResponse = serde_json::from_str(&body).map_err(|e| {
            AgentError::LlmError(format!("Failed to parse response: {} - {}", e, body))
        })?;

        self.parse_response(api_response)
    }

    fn name(&self) -> &str {
        "claude"
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn count_tokens(&self, text: &str) -> usize {
        // Claude uses ~4 chars per token on average
        text.len() / 4
    }
}

// API Types

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    content: ApiContent,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ApiContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Serialize)]
struct ApiTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: ApiUsage,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    input_tokens: usize,
    output_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_messages() {
        let provider = ClaudeProvider::new("test-key");
        let messages = vec![Message::system("You are helpful"), Message::user("Hello")];

        let (system, api_messages) = provider.convert_messages(&messages);

        assert_eq!(system, Some("You are helpful".to_string()));
        assert_eq!(api_messages.len(), 1);
        assert_eq!(api_messages[0].role, "user");
    }

    #[test]
    fn test_convert_tools() {
        let provider = ClaudeProvider::new("test-key");
        let tools = vec![ToolSchema::new("test", "A test tool")];

        let api_tools = provider.convert_tools(&tools);

        assert_eq!(api_tools.len(), 1);
        assert_eq!(api_tools[0].name, "test");
    }
}
