//! Handler traits for MCP operations.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::messages::*;
use crate::protocol::*;
use crate::Result;

/// Handler for tool operations.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// List available tools.
    async fn list_tools(&self, params: ListToolsParams) -> Result<ListToolsResult>;

    /// Call a tool.
    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult>;
}

/// Handler for resource operations.
#[async_trait]
pub trait ResourceHandler: Send + Sync {
    /// List available resources.
    async fn list_resources(&self, params: ListResourcesParams) -> Result<ListResourcesResult>;

    /// Read a resource.
    async fn read_resource(&self, params: ReadResourceParams) -> Result<ReadResourceResult>;

    /// List resource templates (optional).
    async fn list_resource_templates(
        &self,
        _params: ListResourceTemplatesParams,
    ) -> Result<ListResourceTemplatesResult> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![],
            next_cursor: None,
        })
    }

    /// Subscribe to a resource (optional).
    async fn subscribe(&self, _params: SubscribeResourceParams) -> Result<EmptyResult> {
        Ok(EmptyResult {})
    }

    /// Unsubscribe from a resource (optional).
    async fn unsubscribe(&self, _params: UnsubscribeResourceParams) -> Result<EmptyResult> {
        Ok(EmptyResult {})
    }
}

/// Handler for prompt operations.
#[async_trait]
pub trait PromptHandler: Send + Sync {
    /// List available prompts.
    async fn list_prompts(&self, params: ListPromptsParams) -> Result<ListPromptsResult>;

    /// Get a prompt with arguments.
    async fn get_prompt(&self, params: GetPromptParams) -> Result<GetPromptResult>;
}

/// Combined handler for a complete MCP server.
#[async_trait]
pub trait McpHandler: Send + Sync {
    /// Get server info for initialization.
    fn server_info(&self) -> Implementation;

    /// Get server capabilities.
    fn capabilities(&self) -> ServerCapabilities;

    /// Optional instructions for the client.
    fn instructions(&self) -> Option<String> {
        None
    }

    /// Handle tool list request.
    async fn list_tools(&self, params: ListToolsParams) -> Result<ListToolsResult>;

    /// Handle tool call request.
    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult>;

    /// Handle resource list request.
    async fn list_resources(&self, params: ListResourcesParams) -> Result<ListResourcesResult>;

    /// Handle resource read request.
    async fn read_resource(&self, params: ReadResourceParams) -> Result<ReadResourceResult>;

    /// Handle resource templates list request.
    async fn list_resource_templates(
        &self,
        _params: ListResourceTemplatesParams,
    ) -> Result<ListResourceTemplatesResult> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![],
            next_cursor: None,
        })
    }

    /// Handle prompt list request.
    async fn list_prompts(&self, params: ListPromptsParams) -> Result<ListPromptsResult>;

    /// Handle get prompt request.
    async fn get_prompt(&self, params: GetPromptParams) -> Result<GetPromptResult>;

    /// Handle logging level change.
    async fn set_log_level(&self, _params: SetLevelParams) -> Result<EmptyResult> {
        Ok(EmptyResult {})
    }
}

/// A simple tool definition with its handler function.
pub struct ToolDef {
    /// Tool metadata.
    pub tool: Tool,
    /// Handler function.
    pub handler: Box<
        dyn Fn(
                HashMap<String, serde_json::Value>,
            ) -> futures::future::BoxFuture<'static, Result<CallToolResult>>
            + Send
            + Sync,
    >,
}

impl ToolDef {
    /// Create a new tool definition.
    pub fn new<F, Fut>(tool: Tool, handler: F) -> Self
    where
        F: Fn(HashMap<String, serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<CallToolResult>> + Send + 'static,
    {
        Self {
            tool,
            handler: Box::new(move |args| Box::pin(handler(args))),
        }
    }
}

/// A simple resource definition with its handler function.
pub struct ResourceDef {
    /// Resource metadata.
    pub resource: Resource,
    /// Handler function.
    pub handler: Box<
        dyn Fn() -> futures::future::BoxFuture<'static, Result<ResourceContents>> + Send + Sync,
    >,
}

impl ResourceDef {
    /// Create a new resource definition.
    pub fn new<F, Fut>(resource: Resource, handler: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ResourceContents>> + Send + 'static,
    {
        Self {
            resource,
            handler: Box::new(move || Box::pin(handler())),
        }
    }
}

/// A simple prompt definition with its handler function.
pub struct PromptDef {
    /// Prompt metadata.
    pub prompt: Prompt,
    /// Handler function.
    pub handler: Box<
        dyn Fn(
                HashMap<String, String>,
            ) -> futures::future::BoxFuture<'static, Result<GetPromptResult>>
            + Send
            + Sync,
    >,
}

impl PromptDef {
    /// Create a new prompt definition.
    pub fn new<F, Fut>(prompt: Prompt, handler: F) -> Self
    where
        F: Fn(HashMap<String, String>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<GetPromptResult>> + Send + 'static,
    {
        Self {
            prompt,
            handler: Box::new(move |args| Box::pin(handler(args))),
        }
    }
}
