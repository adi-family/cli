//! Data types for the Anthropic API.

use serde::{Deserialize, Serialize};

/// Message role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Content block in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content.
    Text { text: String },
    /// Tool use request.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result.
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

/// Message content (can be string or blocks).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content.
    Text(String),
    /// Block-based content.
    Blocks(Vec<ContentBlock>),
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message role (user or assistant).
    pub role: Role,
    /// Message content.
    pub content: MessageContent,
}

impl Message {
    /// Create a user message with text content.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(text.into()),
        }
    }

    /// Create an assistant message with text content.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(text.into()),
        }
    }

    /// Create a user message with content blocks.
    pub fn user_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Blocks(blocks),
        }
    }

    /// Create an assistant message with content blocks.
    pub fn assistant_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Blocks(blocks),
        }
    }
}

/// Tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON schema for input parameters.
    pub input_schema: serde_json::Value,
}

impl Tool {
    /// Create a new tool definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Request to create a message.
#[derive(Debug, Clone, Serialize)]
pub struct CreateMessageRequest {
    /// Model to use.
    pub model: String,
    /// Messages in the conversation.
    pub messages: Vec<Message>,
    /// Maximum tokens to generate.
    pub max_tokens: usize,
    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Temperature for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Available tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl CreateMessageRequest {
    /// Create a new message request.
    pub fn new(model: impl Into<String>, messages: Vec<Message>, max_tokens: usize) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens,
            system: None,
            temperature: None,
            top_p: None,
            stop_sequences: None,
            tools: None,
            stream: None,
        }
    }

    /// Set the system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top-p sampling.
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set stop sequences.
    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(sequences);
        self
    }

    /// Set available tools.
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    /// Input tokens.
    pub input_tokens: usize,
    /// Output tokens.
    pub output_tokens: usize,
}

/// Response from creating a message.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMessageResponse {
    /// Response ID.
    pub id: String,
    /// Response type (always "message").
    #[serde(rename = "type")]
    pub response_type: String,
    /// Role (always "assistant").
    pub role: Role,
    /// Content blocks.
    pub content: Vec<ContentBlock>,
    /// Model used.
    pub model: String,
    /// Stop reason.
    pub stop_reason: Option<String>,
    /// Stop sequence that triggered stop.
    pub stop_sequence: Option<String>,
    /// Token usage.
    pub usage: Usage,
}

impl CreateMessageResponse {
    /// Extract text content from the response.
    pub fn text(&self) -> Option<String> {
        for block in &self.content {
            if let ContentBlock::Text { text } = block {
                return Some(text.clone());
            }
        }
        None
    }

    /// Extract tool use blocks from the response.
    pub fn tool_uses(&self) -> Vec<(&str, &str, &serde_json::Value)> {
        self.content
            .iter()
            .filter_map(|block| {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    Some((id.as_str(), name.as_str(), input))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if the response contains tool calls.
    pub fn has_tool_use(&self) -> bool {
        self.content
            .iter()
            .any(|block| matches!(block, ContentBlock::ToolUse { .. }))
    }
}

/// Model information.
#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    /// Model ID.
    pub id: String,
    /// Display name.
    pub display_name: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Error response from the API.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    /// Error type.
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error details.
    pub error: ErrorDetail,
}

/// Error detail.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorDetail {
    /// Error type.
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error message.
    pub message: String,
}
