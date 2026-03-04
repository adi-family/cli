//! Data types for the OpenRouter API.

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call ID (for tool role messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message with tool calls.
    pub fn assistant_with_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// Create a tool result message.
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call ID.
    pub id: String,
    /// Tool type (always "function").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function call details.
    pub function: FunctionCall,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: name.into(),
                arguments: arguments.into(),
            },
        }
    }
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name.
    pub name: String,
    /// JSON-encoded arguments.
    pub arguments: String,
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

/// Provider preferences for routing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderPreferences {
    /// Allow fallback to other providers if primary fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    /// Require specific providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    /// Data collection consent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_collection: Option<String>,
    /// Provider order preference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    /// Providers to ignore.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    /// Quantization preference (e.g., "int4", "int8", "fp6", "fp8", "fp16", "bf16").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<String>>,
}

/// Request to create a chat completion.
#[derive(Debug, Clone, Serialize)]
pub struct CreateChatCompletionRequest {
    /// Model to use (e.g., "openai/gpt-4o", "anthropic/claude-3.5-sonnet").
    pub model: String,
    /// Messages in the conversation.
    pub messages: Vec<Message>,
    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Temperature for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Available tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,
    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Provider routing preferences (OpenRouter-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderPreferences>,
    /// Model fallback list (OpenRouter-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
    /// Route to select model based on prompt (OpenRouter-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

impl CreateChatCompletionRequest {
    /// Create a new chat completion request.
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            tools: None,
            stream: None,
            n: None,
            presence_penalty: None,
            frequency_penalty: None,
            provider: None,
            models: None,
            route: None,
        }
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature.
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
    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Set available tools.
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set provider preferences.
    pub fn with_provider(mut self, provider: ProviderPreferences) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Set fallback models.
    pub fn with_fallback_models(mut self, models: Vec<String>) -> Self {
        self.models = Some(models);
        self
    }

    /// Set route (e.g., "fallback" for auto-routing).
    pub fn with_route(mut self, route: impl Into<String>) -> Self {
        self.route = Some(route.into());
        self
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    /// Prompt tokens.
    pub prompt_tokens: usize,
    /// Completion tokens.
    pub completion_tokens: usize,
    /// Total tokens.
    pub total_tokens: usize,
}

/// A completion choice.
#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    /// Choice index.
    pub index: usize,
    /// Generated message.
    pub message: Message,
    /// Finish reason.
    pub finish_reason: Option<String>,
}

/// Response from creating a chat completion.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChatCompletionResponse {
    /// Response ID.
    pub id: String,
    /// Object type.
    pub object: String,
    /// Creation timestamp.
    pub created: u64,
    /// Model used.
    pub model: String,
    /// Completion choices.
    pub choices: Vec<Choice>,
    /// Token usage.
    pub usage: Option<Usage>,
}

impl CreateChatCompletionResponse {
    /// Get the first choice's message content.
    pub fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.message.content.as_deref())
    }

    /// Get the first choice's tool calls.
    pub fn tool_calls(&self) -> Option<&Vec<ToolCall>> {
        self.choices
            .first()
            .and_then(|c| c.message.tool_calls.as_ref())
    }

    /// Check if the response contains tool calls.
    pub fn has_tool_calls(&self) -> bool {
        self.choices
            .first()
            .is_some_and(|c| c.message.tool_calls.is_some())
    }
}

/// Model pricing information.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelPricing {
    /// Price per prompt token (in USD).
    pub prompt: String,
    /// Price per completion token (in USD).
    pub completion: String,
    /// Price per image (if applicable).
    #[serde(default)]
    pub image: Option<String>,
    /// Price per request (if applicable).
    #[serde(default)]
    pub request: Option<String>,
}

/// Model information from OpenRouter.
#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    /// Model ID (e.g., "openai/gpt-4o").
    pub id: String,
    /// Display name.
    pub name: String,
    /// Model description.
    #[serde(default)]
    pub description: Option<String>,
    /// Context length in tokens.
    pub context_length: usize,
    /// Pricing information.
    pub pricing: ModelPricing,
    /// Top provider for this model.
    #[serde(default)]
    pub top_provider: Option<TopProvider>,
    /// Model architecture.
    #[serde(default)]
    pub architecture: Option<ModelArchitecture>,
}

/// Top provider details.
#[derive(Debug, Clone, Deserialize)]
pub struct TopProvider {
    /// Context length from this provider.
    #[serde(default)]
    pub context_length: Option<usize>,
    /// Max completion tokens.
    #[serde(default)]
    pub max_completion_tokens: Option<usize>,
    /// Whether provider is moderated.
    #[serde(default)]
    pub is_moderated: Option<bool>,
}

/// Model architecture details.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelArchitecture {
    /// Modality (e.g., "text->text", "text+image->text").
    #[serde(default)]
    pub modality: Option<String>,
    /// Tokenizer used.
    #[serde(default)]
    pub tokenizer: Option<String>,
    /// Instruction type.
    #[serde(default)]
    pub instruct_type: Option<String>,
}

/// List of models.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelList {
    /// Models.
    pub data: Vec<Model>,
}

/// Generation statistics (returned by /api/v1/generation).
#[derive(Debug, Clone, Deserialize)]
pub struct GenerationStats {
    /// Generation ID.
    pub id: String,
    /// Total cost in USD.
    #[serde(default)]
    pub total_cost: Option<f64>,
    /// Tokens used.
    #[serde(default)]
    pub tokens_prompt: Option<usize>,
    /// Tokens generated.
    #[serde(default)]
    pub tokens_completion: Option<usize>,
    /// Native tokens used.
    #[serde(default)]
    pub native_tokens_prompt: Option<usize>,
    /// Native tokens generated.
    #[serde(default)]
    pub native_tokens_completion: Option<usize>,
}

/// Credit balance response.
#[derive(Debug, Clone, Deserialize)]
pub struct CreditsResponse {
    /// Remaining credits in USD.
    #[serde(default)]
    pub data: Option<CreditsData>,
}

/// Credit balance data.
#[derive(Debug, Clone, Deserialize)]
pub struct CreditsData {
    /// Label (usually "default").
    #[serde(default)]
    pub label: Option<String>,
    /// Remaining balance in USD.
    #[serde(default)]
    pub balance: Option<f64>,
    /// Usage limit per interval.
    #[serde(default)]
    pub usage: Option<f64>,
    /// Rate limit interval in seconds.
    #[serde(default)]
    pub limit: Option<f64>,
    /// Whether rate limited.
    #[serde(default)]
    pub is_free_tier: Option<bool>,
}

/// Error response from the API.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    /// Error details.
    pub error: ErrorDetail,
}

/// Error detail.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorDetail {
    /// Error message.
    pub message: String,
    /// Error type.
    #[serde(rename = "type")]
    #[serde(default)]
    pub error_type: Option<String>,
    /// Error code.
    #[serde(default)]
    pub code: Option<i32>,
}
