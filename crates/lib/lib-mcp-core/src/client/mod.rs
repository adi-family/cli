//! MCP client implementation.
//!
//! This module provides a client for connecting to MCP servers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, error, info, warn};

use crate::jsonrpc::{Message, RequestId, Response};
use crate::messages::*;
use crate::protocol::*;
use crate::transport::Transport;
use crate::{Error, Result};

/// MCP client for communicating with servers.
pub struct McpClient<T> {
    transport: Arc<T>,
    next_id: AtomicI64,
    pending: Arc<Mutex<HashMap<RequestId, oneshot::Sender<Response>>>>,
    server_info: Arc<Mutex<Option<Implementation>>>,
    server_capabilities: Arc<Mutex<Option<ServerCapabilities>>>,
}

impl<T: Transport + 'static> McpClient<T> {
    /// Create a new client with the given transport.
    pub fn new(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
            next_id: AtomicI64::new(1),
            pending: Arc::new(Mutex::new(HashMap::new())),
            server_info: Arc::new(Mutex::new(None)),
            server_capabilities: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the client and initialize the connection.
    pub async fn connect(&self, client_info: Implementation) -> Result<InitializeResult> {
        // Start message receiver
        self.start_receiver();

        // Send initialize request
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info,
        };

        let result: InitializeResult = self.request(methods::INITIALIZE, params).await?;

        // Store server info
        *self.server_info.lock().await = Some(result.server_info.clone());
        *self.server_capabilities.lock().await = Some(result.capabilities.clone());

        // Send initialized notification
        self.notify(methods::INITIALIZED, ()).await?;

        info!(
            server = %result.server_info.name,
            version = %result.server_info.version,
            "Connected to MCP server"
        );

        Ok(result)
    }

    fn start_receiver(&self) {
        let transport = self.transport.clone();
        let pending = self.pending.clone();

        tokio::spawn(async move {
            loop {
                match transport.receive().await {
                    Ok(Some(message)) => {
                        if let Message::Response(response) = message {
                            if let Some(id) = &response.id {
                                let mut pending = pending.lock().await;
                                if let Some(tx) = pending.remove(id) {
                                    let _ = tx.send(response);
                                } else {
                                    warn!(id = %id, "Received response for unknown request");
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        debug!("Transport closed");
                        break;
                    }
                    Err(e) => {
                        error!(error = %e, "Error receiving message");
                        break;
                    }
                }
            }
        });
    }

    fn next_request_id(&self) -> RequestId {
        RequestId::Number(self.next_id.fetch_add(1, Ordering::SeqCst))
    }

    /// Send a request and wait for response.
    pub async fn request<P: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: P,
    ) -> Result<R> {
        let id = self.next_request_id();
        let params_value = serde_json::to_value(params)?;

        let message = Message::request(id.clone(), method, Some(params_value));

        // Set up response channel
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id.clone(), tx);
        }

        // Send request
        debug!(method = %method, id = %id, "Sending request");
        self.transport.send(message).await?;

        // Wait for response
        let response = rx.await.map_err(|_| Error::ChannelRecv)?;

        // Parse result
        response.parse_result()
    }

    /// Send a notification (no response expected).
    pub async fn notify<P: serde::Serialize>(&self, method: &str, params: P) -> Result<()> {
        let params_value = serde_json::to_value(params)?;
        let params = if params_value.is_null() {
            None
        } else {
            Some(params_value)
        };

        let message = Message::notification(method, params);
        debug!(method = %method, "Sending notification");
        self.transport.send(message).await
    }

    /// Get server info (after initialization).
    pub async fn server_info(&self) -> Option<Implementation> {
        self.server_info.lock().await.clone()
    }

    /// Get server capabilities (after initialization).
    pub async fn server_capabilities(&self) -> Option<ServerCapabilities> {
        self.server_capabilities.lock().await.clone()
    }

    // ==========================================================================
    // Tool methods
    // ==========================================================================

    /// List available tools.
    pub async fn list_tools(&self) -> Result<ListToolsResult> {
        self.list_tools_with_cursor(None).await
    }

    /// List available tools with pagination.
    pub async fn list_tools_with_cursor(&self, cursor: Option<Cursor>) -> Result<ListToolsResult> {
        self.request(methods::TOOLS_LIST, ListToolsParams { cursor })
            .await
    }

    /// Call a tool.
    pub async fn call_tool(
        &self,
        name: impl Into<String>,
        arguments: HashMap<String, serde_json::Value>,
    ) -> Result<CallToolResult> {
        self.request(
            methods::TOOLS_CALL,
            CallToolParams {
                name: name.into(),
                arguments,
            },
        )
        .await
    }

    // ==========================================================================
    // Resource methods
    // ==========================================================================

    /// List available resources.
    pub async fn list_resources(&self) -> Result<ListResourcesResult> {
        self.list_resources_with_cursor(None).await
    }

    /// List available resources with pagination.
    pub async fn list_resources_with_cursor(
        &self,
        cursor: Option<Cursor>,
    ) -> Result<ListResourcesResult> {
        self.request(methods::RESOURCES_LIST, ListResourcesParams { cursor })
            .await
    }

    /// Read a resource.
    pub async fn read_resource(&self, uri: impl Into<String>) -> Result<ReadResourceResult> {
        self.request(
            methods::RESOURCES_READ,
            ReadResourceParams { uri: uri.into() },
        )
        .await
    }

    /// List resource templates.
    pub async fn list_resource_templates(&self) -> Result<ListResourceTemplatesResult> {
        self.request(
            methods::RESOURCES_TEMPLATES_LIST,
            ListResourceTemplatesParams { cursor: None },
        )
        .await
    }

    // ==========================================================================
    // Prompt methods
    // ==========================================================================

    /// List available prompts.
    pub async fn list_prompts(&self) -> Result<ListPromptsResult> {
        self.list_prompts_with_cursor(None).await
    }

    /// List available prompts with pagination.
    pub async fn list_prompts_with_cursor(
        &self,
        cursor: Option<Cursor>,
    ) -> Result<ListPromptsResult> {
        self.request(methods::PROMPTS_LIST, ListPromptsParams { cursor })
            .await
    }

    /// Get a prompt with arguments.
    pub async fn get_prompt(
        &self,
        name: impl Into<String>,
        arguments: HashMap<String, String>,
    ) -> Result<GetPromptResult> {
        self.request(
            methods::PROMPTS_GET,
            GetPromptParams {
                name: name.into(),
                arguments,
            },
        )
        .await
    }

    // ==========================================================================
    // Utility methods
    // ==========================================================================

    /// Ping the server.
    pub async fn ping(&self) -> Result<()> {
        let _: serde_json::Value = self.request(methods::PING, ()).await?;
        Ok(())
    }

    /// Set the server's logging level.
    pub async fn set_log_level(&self, level: LogLevel) -> Result<()> {
        let _: EmptyResult = self
            .request(methods::LOGGING_SET_LEVEL, SetLevelParams { level })
            .await?;
        Ok(())
    }

    /// Close the client connection.
    pub async fn close(&self) -> Result<()> {
        self.transport.close().await
    }
}

/// Builder for creating MCP clients.
pub struct McpClientBuilder {
    client_name: String,
    client_version: String,
}

impl McpClientBuilder {
    /// Create a new client builder.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            client_name: name.into(),
            client_version: version.into(),
        }
    }

    /// Build and connect the client.
    pub async fn connect<T: Transport + 'static>(self, transport: T) -> Result<McpClient<T>> {
        let client = McpClient::new(transport);
        let client_info = Implementation::new(self.client_name, self.client_version);
        client.connect(client_info).await?;
        Ok(client)
    }
}
