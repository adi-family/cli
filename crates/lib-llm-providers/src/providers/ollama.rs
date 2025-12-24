//! Ollama local model provider implementation.

use adi_agent_loop_core::error::{AgentError, Result};
use adi_agent_loop_core::llm::{LlmConfig, LlmProvider, LlmResponse, TokenUsage};
use adi_agent_loop_core::tool::ToolSchema;
use adi_agent_loop_core::types::{Message, ToolCall};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const DEFAULT_OLLAMA_HOST: &str = "http://localhost:11434";

/// Ollama provider for local model inference.
pub struct OllamaProvider {
    host: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider with the default host (localhost:11434).
    pub fn new() -> Self {
        Self {
            host: DEFAULT_OLLAMA_HOST.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Set a custom host URL.
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<ApiMessage> {
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg {
                Message::System { content, .. } => {
                    api_messages.push(ApiMessage {
                        role: "system".to_string(),
                        content: content.clone(),
                        tool_calls: None,
                    });
                }
                Message::User { content, .. } => {
                    api_messages.push(ApiMessage {
                        role: "user".to_string(),
                        content: content.clone(),
                        tool_calls: None,
                    });
                }
                Message::Assistant {
                    content,
                    tool_calls,
                    ..
                } => {
                    let api_tool_calls = tool_calls.as_ref().map(|calls| {
                        calls
                            .iter()
                            .map(|c| ApiToolCall {
                                function: ApiFunctionCall {
                                    name: c.name.clone(),
                                    arguments: c.arguments.clone(),
                                },
                            })
                            .collect()
                    });

                    api_messages.push(ApiMessage {
                        role: "assistant".to_string(),
                        content: content.clone().unwrap_or_default(),
                        tool_calls: api_tool_calls,
                    });
                }
                Message::Tool {
                    tool_call_id,
                    content,
                    ..
                } => {
                    // Ollama uses a "tool" role for tool results
                    api_messages.push(ApiMessage {
                        role: "tool".to_string(),
                        content: format!("[{}] {}", tool_call_id, content),
                        tool_calls: None,
                    });
                }
            }
        }

        api_messages
    }

    fn convert_tools(&self, tools: &[ToolSchema]) -> Vec<ApiTool> {
        tools
            .iter()
            .map(|t| ApiTool {
                r#type: "function".to_string(),
                function: ApiFunction {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                },
            })
            .collect()
    }

    fn parse_response(&self, response: ApiResponse) -> Result<LlmResponse> {
        let tool_calls = response.message.tool_calls.map(|calls| {
            calls
                .into_iter()
                .enumerate()
                .map(|(i, c)| ToolCall {
                    id: format!("call_{}", i),
                    name: c.function.name,
                    arguments: c.function.arguments,
                })
                .collect()
        });

        let content = if response.message.content.is_empty() {
            None
        } else {
            Some(response.message.content)
        };

        let message = Message::Assistant {
            content,
            tool_calls,
            timestamp: Some(chrono::Utc::now()),
        };

        // Ollama doesn't always provide token counts
        let usage = TokenUsage {
            prompt_tokens: response.prompt_eval_count.unwrap_or(0),
            completion_tokens: response.eval_count.unwrap_or(0),
            total_tokens: response.prompt_eval_count.unwrap_or(0)
                + response.eval_count.unwrap_or(0),
        };

        Ok(LlmResponse {
            message,
            usage,
            stop_reason: Some(response.done_reason.unwrap_or_else(|| "stop".to_string())),
        })
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &LlmConfig,
    ) -> Result<LlmResponse> {
        let api_messages = self.convert_messages(messages);
        let api_tools = self.convert_tools(tools);

        let request = ApiRequest {
            model: config.model.clone(),
            messages: api_messages,
            stream: false,
            options: Some(ApiOptions {
                temperature: Some(config.temperature),
                top_p: config.top_p,
                num_predict: Some(config.max_tokens as i32),
                stop: config.stop_sequences.clone(),
            }),
            tools: if api_tools.is_empty() {
                None
            } else {
                Some(api_tools)
            },
        };

        let url = format!("{}/api/chat", self.host);

        let response = self
            .client
            .post(&url)
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
        "ollama"
    }

    fn supports_tools(&self) -> bool {
        // Most Ollama models support tools, but some don't
        true
    }

    fn count_tokens(&self, text: &str) -> usize {
        // Rough estimate
        text.len() / 4
    }
}

// API Types

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ApiOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
}

#[derive(Debug, Serialize)]
struct ApiOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiToolCall {
    function: ApiFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiFunctionCall {
    name: String,
    arguments: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ApiTool {
    r#type: String,
    function: ApiFunction,
}

#[derive(Debug, Serialize)]
struct ApiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    message: ApiMessage,
    done_reason: Option<String>,
    prompt_eval_count: Option<usize>,
    eval_count: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_messages() {
        let provider = OllamaProvider::new();
        let messages = vec![Message::system("You are helpful"), Message::user("Hello")];

        let api_messages = provider.convert_messages(&messages);

        assert_eq!(api_messages.len(), 2);
        assert_eq!(api_messages[0].role, "system");
        assert_eq!(api_messages[1].role, "user");
    }

    #[test]
    fn test_default_host() {
        let provider = OllamaProvider::new();
        assert_eq!(provider.host, "http://localhost:11434");
    }

    #[test]
    fn test_custom_host() {
        let provider = OllamaProvider::new().with_host("http://custom:8080");
        assert_eq!(provider.host, "http://custom:8080");
    }
}
