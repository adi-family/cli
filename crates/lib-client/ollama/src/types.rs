//! Data types for the Ollama API.

use serde::{Deserialize, Serialize};

/// Message role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message role.
    pub role: Role,
    /// Message content.
    pub content: String,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl Message {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_calls: None,
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_calls: None,
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls: None,
        }
    }

    /// Create an assistant message with tool calls.
    pub fn assistant_with_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: String::new(),
            tool_calls: Some(tool_calls),
        }
    }

    /// Create a tool result message.
    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_calls: None,
        }
    }
}

/// Tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Function call details.
    pub function: FunctionCall,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            function: FunctionCall {
                name: name.into(),
                arguments,
            },
        }
    }
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name.
    pub name: String,
    /// Arguments as JSON.
    pub arguments: serde_json::Value,
}

/// Tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool type (always "function").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function definition.
    pub function: FunctionDefinition,
}

impl Tool {
    /// Create a new function tool.
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

/// Function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name.
    pub name: String,
    /// Function description.
    pub description: String,
    /// JSON schema for parameters.
    pub parameters: serde_json::Value,
}

/// Options for generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Options {
    /// Temperature for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Number of tokens to predict.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

impl Options {
    /// Create new options with temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top-p sampling.
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set number of tokens to predict.
    pub fn with_num_predict(mut self, num_predict: i32) -> Self {
        self.num_predict = Some(num_predict);
        self
    }

    /// Set stop sequences.
    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }
}

/// Request to generate a chat completion.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// Model name.
    pub model: String,
    /// Messages in the conversation.
    pub messages: Vec<Message>,
    /// Whether to stream the response.
    pub stream: bool,
    /// Generation options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Options>,
    /// Available tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Format (e.g., "json").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl ChatRequest {
    /// Create a new chat request.
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            stream: false,
            options: None,
            tools: None,
            format: None,
        }
    }

    /// Set generation options.
    pub fn with_options(mut self, options: Options) -> Self {
        self.options = Some(options);
        self
    }

    /// Set available tools.
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set output format.
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Enable streaming.
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}

/// Response from a chat completion.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    /// Model used.
    pub model: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Response message.
    pub message: Message,
    /// Whether generation is done.
    pub done: bool,
    /// Reason for stopping.
    pub done_reason: Option<String>,
    /// Total duration in nanoseconds.
    pub total_duration: Option<u64>,
    /// Load duration in nanoseconds.
    pub load_duration: Option<u64>,
    /// Prompt evaluation count.
    pub prompt_eval_count: Option<usize>,
    /// Prompt evaluation duration in nanoseconds.
    pub prompt_eval_duration: Option<u64>,
    /// Evaluation count.
    pub eval_count: Option<usize>,
    /// Evaluation duration in nanoseconds.
    pub eval_duration: Option<u64>,
}

impl ChatResponse {
    /// Get the response content.
    pub fn content(&self) -> &str {
        &self.message.content
    }

    /// Get tool calls if any.
    pub fn tool_calls(&self) -> Option<&Vec<ToolCall>> {
        self.message.tool_calls.as_ref()
    }

    /// Check if the response contains tool calls.
    pub fn has_tool_calls(&self) -> bool {
        self.message.tool_calls.is_some()
    }
}

/// Request to generate a completion (non-chat).
#[derive(Debug, Clone, Serialize)]
pub struct GenerateRequest {
    /// Model name.
    pub model: String,
    /// Prompt text.
    pub prompt: String,
    /// Whether to stream the response.
    pub stream: bool,
    /// Generation options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Options>,
    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Format (e.g., "json").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl GenerateRequest {
    /// Create a new generate request.
    pub fn new(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            stream: false,
            options: None,
            system: None,
            format: None,
        }
    }

    /// Set generation options.
    pub fn with_options(mut self, options: Options) -> Self {
        self.options = Some(options);
        self
    }

    /// Set system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }
}

/// Response from a generate request.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    /// Model used.
    pub model: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Generated response.
    pub response: String,
    /// Whether generation is done.
    pub done: bool,
    /// Total duration in nanoseconds.
    pub total_duration: Option<u64>,
    /// Prompt evaluation count.
    pub prompt_eval_count: Option<usize>,
    /// Evaluation count.
    pub eval_count: Option<usize>,
}

/// Model information.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    /// Model name.
    pub name: String,
    /// Model modification time.
    pub modified_at: String,
    /// Model size in bytes.
    pub size: u64,
    /// Model digest.
    pub digest: String,
    /// Model details.
    pub details: Option<ModelDetails>,
}

/// Model details.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelDetails {
    /// Format.
    pub format: Option<String>,
    /// Model family.
    pub family: Option<String>,
    /// Parameter size.
    pub parameter_size: Option<String>,
    /// Quantization level.
    pub quantization_level: Option<String>,
}

/// List of models.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelList {
    /// Models.
    pub models: Vec<ModelInfo>,
}

/// Error response from the API.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
}
