//! Builder for creating MCP servers with a fluent API.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::messages::*;
use crate::protocol::*;
use crate::server::{McpHandler, PromptDef, ResourceDef, ToolDef};
use crate::{Error, Result};

/// Builder for creating an MCP server.
pub struct McpServerBuilder {
    name: String,
    version: String,
    instructions: Option<String>,
    tools: HashMap<String, ToolDef>,
    resources: HashMap<String, ResourceDef>,
    prompts: HashMap<String, PromptDef>,
    capabilities: ServerCapabilities,
}

impl McpServerBuilder {
    /// Create a new server builder.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            instructions: None,
            tools: HashMap::new(),
            resources: HashMap::new(),
            prompts: HashMap::new(),
            capabilities: ServerCapabilities::default(),
        }
    }

    /// Set instructions for the client.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Add a tool to the server.
    pub fn tool<F, Fut>(mut self, tool: Tool, handler: F) -> Self
    where
        F: Fn(HashMap<String, serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<CallToolResult>> + Send + 'static,
    {
        let name = tool.name.clone();
        self.tools.insert(name, ToolDef::new(tool, handler));
        self.capabilities.tools = Some(ToolsCapability::default());
        self
    }

    /// Add a resource to the server.
    pub fn resource<F, Fut>(mut self, resource: Resource, handler: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<ResourceContents>> + Send + 'static,
    {
        let uri = resource.uri.clone();
        self.resources
            .insert(uri, ResourceDef::new(resource, handler));
        self.capabilities.resources = Some(ResourcesCapability::default());
        self
    }

    /// Add a prompt to the server.
    pub fn prompt<F, Fut>(mut self, prompt: Prompt, handler: F) -> Self
    where
        F: Fn(HashMap<String, String>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<GetPromptResult>> + Send + 'static,
    {
        let name = prompt.name.clone();
        self.prompts.insert(name, PromptDef::new(prompt, handler));
        self.capabilities.prompts = Some(PromptsCapability::default());
        self
    }

    /// Enable logging capability.
    pub fn with_logging(mut self) -> Self {
        self.capabilities.logging = Some(LoggingCapability {});
        self
    }

    /// Build the server handler.
    pub fn build(self) -> impl McpHandler {
        SimpleServer {
            info: Implementation::new(self.name, self.version),
            instructions: self.instructions,
            tools: Arc::new(self.tools),
            resources: Arc::new(self.resources),
            prompts: Arc::new(self.prompts),
            capabilities: self.capabilities,
        }
    }
}

/// A simple MCP server implementation built from the builder.
struct SimpleServer {
    info: Implementation,
    instructions: Option<String>,
    tools: Arc<HashMap<String, ToolDef>>,
    resources: Arc<HashMap<String, ResourceDef>>,
    prompts: Arc<HashMap<String, PromptDef>>,
    capabilities: ServerCapabilities,
}

#[async_trait]
impl McpHandler for SimpleServer {
    fn server_info(&self) -> Implementation {
        self.info.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        self.capabilities.clone()
    }

    fn instructions(&self) -> Option<String> {
        self.instructions.clone()
    }

    async fn list_tools(&self, _params: ListToolsParams) -> Result<ListToolsResult> {
        let tools: Vec<Tool> = self.tools.values().map(|t| t.tool.clone()).collect();
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult> {
        let tool = self
            .tools
            .get(&params.name)
            .ok_or_else(|| Error::ToolNotFound(params.name.clone()))?;

        (tool.handler)(params.arguments).await
    }

    async fn list_resources(&self, _params: ListResourcesParams) -> Result<ListResourcesResult> {
        let resources: Vec<Resource> = self
            .resources
            .values()
            .map(|r| r.resource.clone())
            .collect();
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(&self, params: ReadResourceParams) -> Result<ReadResourceResult> {
        let resource = self
            .resources
            .get(&params.uri)
            .ok_or_else(|| Error::ResourceNotFound(params.uri.clone()))?;

        let contents = (resource.handler)().await?;
        Ok(ReadResourceResult {
            contents: vec![contents],
        })
    }

    async fn list_prompts(&self, _params: ListPromptsParams) -> Result<ListPromptsResult> {
        let prompts: Vec<Prompt> = self.prompts.values().map(|p| p.prompt.clone()).collect();
        Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
        })
    }

    async fn get_prompt(&self, params: GetPromptParams) -> Result<GetPromptResult> {
        let prompt = self
            .prompts
            .get(&params.name)
            .ok_or_else(|| Error::PromptNotFound(params.name.clone()))?;

        (prompt.handler)(params.arguments).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_builder() {
        let server = McpServerBuilder::new("test-server", "1.0.0")
            .instructions("This is a test server")
            .tool(
                Tool::new(
                    "echo",
                    ToolInputSchema::new().string_property("message", "Message to echo", true),
                )
                .with_description("Echoes back the message"),
                |args| async move {
                    let msg = args
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("no message");
                    Ok(CallToolResult::text(msg))
                },
            )
            .build();

        // Test list tools
        let tools = server.list_tools(ListToolsParams::default()).await.unwrap();
        assert_eq!(tools.tools.len(), 1);
        assert_eq!(tools.tools[0].name, "echo");

        // Test call tool
        let mut args = HashMap::new();
        args.insert("message".to_string(), serde_json::json!("hello"));
        let result = server
            .call_tool(CallToolParams {
                name: "echo".to_string(),
                arguments: args,
            })
            .await
            .unwrap();
        assert!(!result.is_error);
    }
}
