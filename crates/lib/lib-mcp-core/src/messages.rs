//! MCP method names and request/response types.
//!
//! This module defines all the typed request and response structures
//! for each MCP method.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::protocol::*;

// =============================================================================
// Method Names
// =============================================================================

/// MCP method names.
pub mod methods {
    // Lifecycle
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "notifications/initialized";
    pub const PING: &str = "ping";

    // Tools
    pub const TOOLS_LIST: &str = "tools/list";
    pub const TOOLS_CALL: &str = "tools/call";

    // Resources
    pub const RESOURCES_LIST: &str = "resources/list";
    pub const RESOURCES_READ: &str = "resources/read";
    pub const RESOURCES_SUBSCRIBE: &str = "resources/subscribe";
    pub const RESOURCES_UNSUBSCRIBE: &str = "resources/unsubscribe";
    pub const RESOURCES_TEMPLATES_LIST: &str = "resources/templates/list";

    // Prompts
    pub const PROMPTS_LIST: &str = "prompts/list";
    pub const PROMPTS_GET: &str = "prompts/get";

    // Sampling
    pub const SAMPLING_CREATE_MESSAGE: &str = "sampling/createMessage";

    // Logging
    pub const LOGGING_SET_LEVEL: &str = "logging/setLevel";

    // Roots
    pub const ROOTS_LIST: &str = "roots/list";

    // Notifications
    pub const NOTIFICATION_CANCELLED: &str = "notifications/cancelled";
    pub const NOTIFICATION_PROGRESS: &str = "notifications/progress";
    pub const NOTIFICATION_MESSAGE: &str = "notifications/message";
    pub const NOTIFICATION_RESOURCES_UPDATED: &str = "notifications/resources/updated";
    pub const NOTIFICATION_RESOURCES_LIST_CHANGED: &str = "notifications/resources/list_changed";
    pub const NOTIFICATION_TOOLS_LIST_CHANGED: &str = "notifications/tools/list_changed";
    pub const NOTIFICATION_PROMPTS_LIST_CHANGED: &str = "notifications/prompts/list_changed";
    pub const NOTIFICATION_ROOTS_LIST_CHANGED: &str = "notifications/roots/list_changed";
}

// =============================================================================
// Initialize
// =============================================================================

/// Initialize request params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// Protocol version the client supports.
    pub protocol_version: String,

    /// Client capabilities.
    pub capabilities: ClientCapabilities,

    /// Client implementation info.
    pub client_info: Implementation,
}

/// Initialize response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Protocol version the server chose.
    pub protocol_version: String,

    /// Server capabilities.
    pub capabilities: ServerCapabilities,

    /// Server implementation info.
    pub server_info: Implementation,

    /// Optional instructions for using this server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

// =============================================================================
// Tools
// =============================================================================

/// List tools request params.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsParams {
    /// Pagination cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

/// List tools response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    /// Available tools.
    pub tools: Vec<Tool>,

    /// Next page cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
}

/// Call tool request params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolParams {
    /// Name of the tool to call.
    pub name: String,

    /// Arguments for the tool.
    #[serde(default)]
    pub arguments: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Resources
// =============================================================================

/// List resources request params.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesParams {
    /// Pagination cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

/// List resources response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesResult {
    /// Available resources.
    pub resources: Vec<Resource>,

    /// Next page cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
}

/// List resource templates request params.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesParams {
    /// Pagination cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

/// List resource templates response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesResult {
    /// Available resource templates.
    pub resource_templates: Vec<ResourceTemplate>,

    /// Next page cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
}

/// Read resource request params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceParams {
    /// URI of the resource to read.
    pub uri: String,
}

/// Read resource response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResult {
    /// Resource contents.
    pub contents: Vec<ResourceContents>,
}

/// Subscribe to resource request params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResourceParams {
    /// URI of the resource to subscribe to.
    pub uri: String,
}

/// Unsubscribe from resource request params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeResourceParams {
    /// URI of the resource to unsubscribe from.
    pub uri: String,
}

// =============================================================================
// Prompts
// =============================================================================

/// List prompts request params.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsParams {
    /// Pagination cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

/// List prompts response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsResult {
    /// Available prompts.
    pub prompts: Vec<Prompt>,

    /// Next page cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
}

/// Get prompt request params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptParams {
    /// Name of the prompt.
    pub name: String,

    /// Arguments for the prompt.
    #[serde(default)]
    pub arguments: HashMap<String, String>,
}

// =============================================================================
// Logging
// =============================================================================

/// Set logging level request params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLevelParams {
    /// Desired logging level.
    pub level: LogLevel,
}

// =============================================================================
// Roots
// =============================================================================

/// List roots response result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRootsResult {
    /// Available roots.
    pub roots: Vec<Root>,
}

// =============================================================================
// Notifications
// =============================================================================

/// Progress notification params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressParams {
    /// Progress token to identify the operation.
    pub progress_token: ProgressToken,

    /// Progress value (0.0 to 1.0).
    pub progress: f64,

    /// Optional total value (for absolute progress).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

/// Progress token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    Number(i64),
    String(String),
}

/// Cancelled notification params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelledParams {
    /// ID of the request that was cancelled.
    pub request_id: crate::jsonrpc::RequestId,

    /// Optional reason for cancellation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Resource updated notification params.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUpdatedParams {
    /// URI of the resource that was updated.
    pub uri: String,
}

// =============================================================================
// Empty responses
// =============================================================================

/// Empty result for methods that don't return data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmptyResult {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_params() {
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation::new("test-client", "1.0.0"),
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("clientInfo"));
    }

    #[test]
    fn test_call_tool_params() {
        let mut args = HashMap::new();
        args.insert("query".to_string(), serde_json::json!("test"));

        let params = CallToolParams {
            name: "search".to_string(),
            arguments: args,
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"name\":\"search\""));
        assert!(json.contains("\"query\":\"test\""));
    }
}
