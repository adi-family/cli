//! MCP protocol types following the Model Context Protocol specification.
//!
//! This module defines all the core types used in MCP communication:
//! - Tools: Functions that can be called by LLMs
//! - Resources: Data sources that can be read
//! - Prompts: Template prompts with arguments
//! - Sampling: LLM completion requests

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP protocol version.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Supported protocol versions.
pub const SUPPORTED_VERSIONS: &[&str] = &["2024-11-05"];

// =============================================================================
// Server/Client Info
// =============================================================================

/// Information about an MCP implementation (server or client).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    /// Name of the implementation.
    pub name: String,
    /// Version of the implementation.
    pub version: String,
}

impl Implementation {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

// =============================================================================
// Capabilities
// =============================================================================

/// Server capabilities advertised during initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Tool-related capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,

    /// Resource-related capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,

    /// Prompt-related capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,

    /// Logging capability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,

    /// Experimental capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
}

/// Client capabilities sent during initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// Sampling capability (client can handle createMessage requests).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,

    /// Roots capability (client can provide filesystem roots).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,

    /// Experimental capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
}

/// Tools capability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Whether the server supports listing tools that have changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resources capability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    /// Whether the server supports subscribing to resource changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,

    /// Whether the server supports listing resources that have changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Prompts capability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapability {
    /// Whether the server supports listing prompts that have changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Logging capability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoggingCapability {}

/// Sampling capability (client-side).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SamplingCapability {}

/// Roots capability (client-side).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootsCapability {
    /// Whether the client supports notifying when roots change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

// =============================================================================
// Tools
// =============================================================================

/// Definition of a tool that can be called.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Unique name of the tool.
    pub name: String,

    /// Human-readable description of what the tool does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON Schema describing the tool's input parameters.
    pub input_schema: ToolInputSchema,
}

impl Tool {
    /// Create a new tool with required fields.
    pub fn new(name: impl Into<String>, input_schema: ToolInputSchema) -> Self {
        Self {
            name: name.into(),
            description: None,
            input_schema,
        }
    }

    /// Add a description to the tool.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// JSON Schema for tool input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    /// Must be "object".
    #[serde(rename = "type")]
    pub schema_type: String,

    /// Property definitions.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, serde_json::Value>,

    /// Required property names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

impl Default for ToolInputSchema {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: HashMap::new(),
            required: Vec::new(),
        }
    }
}

impl ToolInputSchema {
    /// Create a new empty schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a property to the schema.
    pub fn property(
        mut self,
        name: impl Into<String>,
        schema: serde_json::Value,
        required: bool,
    ) -> Self {
        let name = name.into();
        self.properties.insert(name.clone(), schema);
        if required {
            self.required.push(name);
        }
        self
    }

    /// Add a string property.
    pub fn string_property(
        self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.property(
            name,
            serde_json::json!({
                "type": "string",
                "description": description.into()
            }),
            required,
        )
    }

    /// Add an integer property.
    pub fn integer_property(
        self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.property(
            name,
            serde_json::json!({
                "type": "integer",
                "description": description.into()
            }),
            required,
        )
    }

    /// Add a boolean property.
    pub fn boolean_property(
        self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.property(
            name,
            serde_json::json!({
                "type": "boolean",
                "description": description.into()
            }),
            required,
        )
    }
}

/// Result of calling a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    /// Content returned by the tool.
    pub content: Vec<ToolContent>,

    /// Whether the tool call resulted in an error.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

impl CallToolResult {
    /// Create a successful text result.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(content)],
            is_error: false,
        }
    }

    /// Create a successful result with multiple content items.
    pub fn contents(content: Vec<ToolContent>) -> Self {
        Self {
            content,
            is_error: false,
        }
    }

    /// Create an error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::text(message)],
            is_error: true,
        }
    }
}

/// Content item in a tool result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ToolContent {
    /// Text content.
    #[serde(rename = "text")]
    Text { text: String },

    /// Image content (base64 encoded).
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },

    /// Embedded resource.
    #[serde(rename = "resource")]
    Resource { resource: ResourceContents },
}

impl ToolContent {
    /// Create text content.
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    /// Create image content.
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    /// Create embedded resource content.
    pub fn resource(resource: ResourceContents) -> Self {
        Self::Resource { resource }
    }
}

// =============================================================================
// Resources
// =============================================================================

/// Definition of a resource that can be read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// URI identifying the resource.
    pub uri: String,

    /// Human-readable name.
    pub name: String,

    /// Description of what the resource contains.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type of the resource content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl Resource {
    /// Create a new resource.
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
        }
    }

    /// Add a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the MIME type.
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }
}

/// Contents of a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContents {
    /// Resource URI.
    pub uri: String,

    /// MIME type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Text content (mutually exclusive with blob).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Binary content as base64 (mutually exclusive with text).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

impl ResourceContents {
    /// Create text resource contents.
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some("text/plain".to_string()),
            text: Some(text.into()),
            blob: None,
        }
    }

    /// Create binary resource contents.
    pub fn blob(
        uri: impl Into<String>,
        data: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
            text: None,
            blob: Some(data.into()),
        }
    }

    /// Set the MIME type.
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }
}

/// Resource template for dynamic resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplate {
    /// URI template (RFC 6570).
    pub uri_template: String,

    /// Human-readable name.
    pub name: String,

    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

// =============================================================================
// Prompts
// =============================================================================

/// Definition of a prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// Unique name of the prompt.
    pub name: String,

    /// Description of what the prompt does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Arguments the prompt accepts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<PromptArgument>,
}

impl Prompt {
    /// Create a new prompt.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            arguments: Vec::new(),
        }
    }

    /// Add a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add an argument.
    pub fn with_argument(mut self, argument: PromptArgument) -> Self {
        self.arguments.push(argument);
        self
    }
}

/// Argument definition for a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptArgument {
    /// Argument name.
    pub name: String,

    /// Description of the argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether the argument is required.
    #[serde(default)]
    pub required: bool,
}

impl PromptArgument {
    /// Create a new argument.
    pub fn new(name: impl Into<String>, required: bool) -> Self {
        Self {
            name: name.into(),
            description: None,
            required,
        }
    }

    /// Add a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Result of getting a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptResult {
    /// Optional description for this prompt invocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Messages that make up the prompt.
    pub messages: Vec<PromptMessage>,
}

/// A message in a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMessage {
    /// Role of the message sender.
    pub role: Role,

    /// Content of the message.
    pub content: MessageContent,
}

impl PromptMessage {
    /// Create a user message with text.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text { text: text.into() },
        }
    }

    /// Create an assistant message with text.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text { text: text.into() },
        }
    }
}

/// Role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Content of a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessageContent {
    /// Text content.
    #[serde(rename = "text")]
    Text { text: String },

    /// Image content.
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },

    /// Embedded resource.
    #[serde(rename = "resource")]
    Resource { resource: ResourceContents },
}

// =============================================================================
// Sampling (Client → Server requests for LLM completion)
// =============================================================================

/// Request to create a message (sampling request from server to client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageRequest {
    /// Messages to send to the LLM.
    pub messages: Vec<SamplingMessage>,

    /// Model preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,

    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Sampling temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Stop sequences.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop_sequences: Vec<String>,

    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Message in a sampling request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingMessage {
    pub role: Role,
    pub content: MessageContent,
}

/// Model preferences for sampling.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreferences {
    /// Hints for model selection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<ModelHint>,

    /// Cost priority (0-1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f32>,

    /// Speed priority (0-1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f32>,

    /// Intelligence priority (0-1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f32>,
}

/// Hint for model selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelHint {
    /// Model name pattern.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Result of creating a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageResult {
    /// Role of the response.
    pub role: Role,

    /// Content of the response.
    pub content: MessageContent,

    /// Model that generated the response.
    pub model: String,

    /// Reason for stopping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
}

/// Reason for stopping generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    EndTurn,
    StopSequence,
    MaxTokens,
}

// =============================================================================
// Logging
// =============================================================================

/// Log level for logging messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// Logging message notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMessage {
    /// Log level.
    pub level: LogLevel,

    /// Optional logger name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,

    /// Log message data.
    pub data: serde_json::Value,
}

// =============================================================================
// Roots
// =============================================================================

/// A root directory that the client has access to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    /// URI of the root.
    pub uri: String,

    /// Optional name for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

// =============================================================================
// Pagination
// =============================================================================

/// Cursor for paginated results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cursor(pub String);

impl From<String> for Cursor {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Cursor {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
