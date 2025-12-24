//! OpenAI API provider implementation.

use adi_agent_loop_core::error::{AgentError, Result};
use adi_agent_loop_core::llm::{LlmConfig, LlmProvider, LlmResponse, TokenUsage};
use adi_agent_loop_core::tool::ToolSchema;
use adi_agent_loop_core::types::{Message, ToolCall};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI provider using the Chat Completions API.
pub struct OpenAiProvider {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
    organization: Option<String>,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: reqwest::Client::new(),
            base_url: OPENAI_API_URL.to_string(),
            organization: None,
        }
    }

    /// Set a custom base URL (for Azure OpenAI or proxies).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set organization ID.
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<ApiMessage> {
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg {
                Message::System { content, .. } => {
                    api_messages.push(ApiMessage {
                        role: "system".to_string(),
                        content: Some(content.clone()),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                Message::User { content, .. } => {
                    api_messages.push(ApiMessage {
                        role: "user".to_string(),
                        content: Some(content.clone()),
                        tool_calls: None,
                        tool_call_id: None,
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
                                id: c.id.clone(),
                                r#type: "function".to_string(),
                                function: ApiFunctionCall {
                                    name: c.name.clone(),
                                    arguments: serde_json::to_string(&c.arguments)
                                        .unwrap_or_default(),
                                },
                            })
                            .collect()
                    });

                    api_messages.push(ApiMessage {
                        role: "assistant".to_string(),
                        content: content.clone(),
                        tool_calls: api_tool_calls,
                        tool_call_id: None,
                    });
                }
                Message::Tool {
                    tool_call_id,
                    content,
                    ..
                } => {
                    api_messages.push(ApiMessage {
                        role: "tool".to_string(),
                        content: Some(content.clone()),
                        tool_calls: None,
                        tool_call_id: Some(tool_call_id.clone()),
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
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AgentError::LlmError("No choices in response".into()))?;

        let tool_calls = choice.message.tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|c| ToolCall {
                    id: c.id,
                    name: c.function.name,
                    arguments: serde_json::from_str(&c.function.arguments).unwrap_or_default(),
                })
                .collect()
        });

        let message = Message::Assistant {
            content: choice.message.content,
            tool_calls,
            timestamp: Some(chrono::Utc::now()),
        };

        let usage = response
            .usage
            .map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            })
            .unwrap_or_default();

        Ok(LlmResponse {
            message,
            usage,
            stop_reason: choice.finish_reason,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &LlmConfig,
    ) -> Result<LlmResponse> {
        let api_messages = self.convert_messages(messages);
        let api_tools = self.convert_tools(tools);

        let is_reasoning_model = config.model.starts_with("o1") || config.model.starts_with("o3");

        let request = ApiRequest {
            model: config.model.clone(),
            messages: api_messages,
            max_tokens: if is_reasoning_model {
                None
            } else {
                Some(config.max_tokens)
            },
            max_completion_tokens: if is_reasoning_model {
                Some(config.max_tokens)
            } else {
                None
            },
            temperature: if is_reasoning_model {
                None
            } else {
                Some(config.temperature)
            },
            top_p: if is_reasoning_model {
                None
            } else {
                config.top_p
            },
            stop: config.stop_sequences.clone(),
            tools: if api_tools.is_empty() {
                None
            } else {
                Some(api_tools)
            },
        };

        let mut req_builder = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(org) = &self.organization {
            req_builder = req_builder.header("OpenAI-Organization", org);
        }

        let response = req_builder
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
        "openai"
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn count_tokens(&self, text: &str) -> usize {
        // GPT-4 uses ~4 chars per token on average
        text.len() / 4
    }
}

// API Types

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ApiTool>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiToolCall {
    id: String,
    r#type: String,
    function: ApiFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiFunctionCall {
    name: String,
    arguments: String,
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
    choices: Vec<ApiChoice>,
    usage: Option<ApiUsage>,
}

#[derive(Debug, Deserialize)]
struct ApiChoice {
    message: ApiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_messages() {
        let provider = OpenAiProvider::new("test-key");
        let messages = vec![Message::system("You are helpful"), Message::user("Hello")];

        let api_messages = provider.convert_messages(&messages);

        assert_eq!(api_messages.len(), 2);
        assert_eq!(api_messages[0].role, "system");
        assert_eq!(api_messages[1].role, "user");
    }

    #[test]
    fn test_convert_tools() {
        let provider = OpenAiProvider::new("test-key");
        let tools = vec![ToolSchema::new("test", "A test tool")];

        let api_tools = provider.convert_tools(&tools);

        assert_eq!(api_tools.len(), 1);
        assert_eq!(api_tools[0].function.name, "test");
    }
}
