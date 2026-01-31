//! Model Context Protocol (MCP) service traits

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// MCP Tools
// ============================================================================

/// MCP tools service trait
///
/// Plugins implementing this trait can provide tools that LLMs can invoke
/// to perform actions or retrieve information.
#[async_trait]
pub trait McpTools: Plugin {
    /// List all MCP tools provided by this plugin
    async fn list_tools(&self) -> Vec<McpTool>;

    /// Call an MCP tool
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult>;
}

/// MCP tool metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name (e.g., "search_code")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// JSON Schema for tool arguments
    pub input_schema: Value,
}

/// MCP tool result
#[derive(Debug, Clone)]
pub struct McpToolResult {
    /// Tool output content
    pub content: String,

    /// Content type
    pub content_type: McpContentType,

    /// Whether this is an error result
    pub is_error: bool,
}

impl McpToolResult {
    /// Create a text result
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            content_type: McpContentType::Text,
            is_error: false,
        }
    }

    /// Create a JSON result
    pub fn json<T: Serialize>(data: &T) -> Result<Self> {
        Ok(Self {
            content: serde_json::to_string(data)?,
            content_type: McpContentType::Json,
            is_error: false,
        })
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: message.into(),
            content_type: McpContentType::Error,
            is_error: true,
        }
    }
}

/// MCP content type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpContentType {
    Text,
    Json,
    Error,
}

// ============================================================================
// MCP Resources
// ============================================================================

/// MCP resources service trait
///
/// Plugins implementing this trait can provide resources (files, data) that
/// LLMs can read to get context.
#[async_trait]
pub trait McpResources: Plugin {
    /// List all resources provided by this plugin
    async fn list_resources(&self) -> Vec<McpResource>;

    /// Read a resource by URI
    async fn read_resource(&self, uri: &str) -> Result<McpResourceContent>;
}

/// MCP resource metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Resource URI (e.g., "file:///project/src/main.rs")
    pub uri: String,

    /// Human-readable name
    pub name: String,

    /// Description of the resource
    pub description: String,

    /// MIME type (e.g., "text/x-rust")
    pub mime_type: String,
}

/// MCP resource content
#[derive(Debug, Clone)]
pub struct McpResourceContent {
    /// Resource URI
    pub uri: String,

    /// Resource content (binary or text)
    pub content: Vec<u8>,

    /// MIME type
    pub mime_type: String,
}

impl McpResourceContent {
    /// Create a text resource
    pub fn text(uri: impl Into<String>, content: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            content: content.into().into_bytes(),
            mime_type: mime_type.into(),
        }
    }

    /// Create a binary resource
    pub fn binary(uri: impl Into<String>, content: Vec<u8>, mime_type: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            content,
            mime_type: mime_type.into(),
        }
    }

    /// Get content as string (if text)
    pub fn as_text(&self) -> Result<String> {
        Ok(String::from_utf8(self.content.clone())?)
    }
}

// ============================================================================
// MCP Prompts
// ============================================================================

/// MCP prompts service trait
///
/// Plugins implementing this trait can provide prompt templates that LLMs
/// can use for specific tasks.
#[async_trait]
pub trait McpPrompts: Plugin {
    /// List all prompts provided by this plugin
    async fn list_prompts(&self) -> Vec<McpPrompt>;

    /// Get a prompt with arguments
    async fn get_prompt(&self, name: &str, arguments: Value) -> Result<Vec<McpPromptMessage>>;
}

/// MCP prompt metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    /// Prompt name (e.g., "code_review")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// JSON Schema for prompt arguments
    pub arguments_schema: Value,
}

/// MCP prompt message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessage {
    /// Message role
    pub role: McpPromptRole,

    /// Message content
    pub content: String,
}

impl McpPromptMessage {
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: McpPromptRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: McpPromptRole::Assistant,
            content: content.into(),
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: McpPromptRole::System,
            content: content.into(),
        }
    }
}

/// MCP prompt role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpPromptRole {
    User,
    Assistant,
    System,
}