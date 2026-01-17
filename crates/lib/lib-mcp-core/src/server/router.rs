//! Message router for MCP servers.
//!
//! Routes incoming JSON-RPC messages to the appropriate handlers.

use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::jsonrpc::{JsonRpcError, Message, Notification, Request, Response};
use crate::messages::*;
use crate::protocol::*;
use crate::server::McpHandler;
use crate::transport::Transport;
use crate::Result;

/// Routes MCP messages to handlers.
pub struct McpRouter<H> {
    handler: Arc<H>,
    initialized: bool,
    client_info: Option<Implementation>,
    client_capabilities: Option<ClientCapabilities>,
}

impl<H: McpHandler> McpRouter<H> {
    /// Create a new router with the given handler.
    pub fn new(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
            initialized: false,
            client_info: None,
            client_capabilities: None,
        }
    }

    /// Run the server with the given transport.
    pub async fn run<T: Transport>(&mut self, transport: T) -> Result<()> {
        info!("MCP server starting");

        loop {
            match transport.receive().await? {
                Some(message) => {
                    let response = self.handle_message(message).await;
                    if let Some(resp) = response {
                        transport.send(resp).await?;
                    }
                }
                None => {
                    info!("Transport closed, shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a single message and return the response (if any).
    pub async fn handle_message(&mut self, message: Message) -> Option<Message> {
        match message {
            Message::Request(req) => Some(Message::Response(self.handle_request(req).await)),
            Message::Notification(notif) => {
                self.handle_notification(notif).await;
                None
            }
            Message::Response(_) => {
                warn!("Received unexpected response");
                None
            }
        }
    }

    async fn handle_request(&mut self, request: Request) -> Response {
        debug!(method = %request.method, id = %request.id, "Handling request");

        let result = match request.method.as_str() {
            methods::INITIALIZE => self.handle_initialize(&request).await,
            methods::PING => Ok(serde_json::json!({})),

            // Check initialization for other methods
            _ if !self.initialized => Err(JsonRpcError::new(-32002, "Server not initialized")),

            methods::TOOLS_LIST => self.handle_tools_list(&request).await,
            methods::TOOLS_CALL => self.handle_tools_call(&request).await,
            methods::RESOURCES_LIST => self.handle_resources_list(&request).await,
            methods::RESOURCES_READ => self.handle_resources_read(&request).await,
            methods::RESOURCES_TEMPLATES_LIST => {
                self.handle_resource_templates_list(&request).await
            }
            methods::PROMPTS_LIST => self.handle_prompts_list(&request).await,
            methods::PROMPTS_GET => self.handle_prompts_get(&request).await,
            methods::LOGGING_SET_LEVEL => self.handle_set_log_level(&request).await,

            _ => Err(JsonRpcError::method_not_found(&request.method)),
        };

        match result {
            Ok(value) => Response::success(request.id, value),
            Err(error) => Response::error(Some(request.id), error),
        }
    }

    async fn handle_notification(&mut self, notification: Notification) {
        debug!(method = %notification.method, "Handling notification");

        match notification.method.as_str() {
            methods::INITIALIZED => {
                info!("Client confirmed initialization");
            }
            methods::NOTIFICATION_CANCELLED => {
                if let Ok(params) = notification.parse_params::<CancelledParams>() {
                    debug!(request_id = %params.request_id, "Request cancelled");
                }
            }
            _ => {
                debug!(method = %notification.method, "Unknown notification");
            }
        }
    }

    async fn handle_initialize(
        &mut self,
        request: &Request,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let params: InitializeParams = request
            .parse_params()
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        info!(
            client = %params.client_info.name,
            version = %params.client_info.version,
            protocol = %params.protocol_version,
            "Client initializing"
        );

        // Check protocol version
        if !SUPPORTED_VERSIONS.contains(&params.protocol_version.as_str()) {
            return Err(JsonRpcError::new(
                -32002,
                format!(
                    "Unsupported protocol version: {}. Supported: {:?}",
                    params.protocol_version, SUPPORTED_VERSIONS
                ),
            ));
        }

        self.client_info = Some(params.client_info);
        self.client_capabilities = Some(params.capabilities);
        self.initialized = true;

        let result = InitializeResult {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: self.handler.capabilities(),
            server_info: self.handler.server_info(),
            instructions: self.handler.instructions(),
        };

        serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_tools_list(
        &self,
        request: &Request,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let params: ListToolsParams = request
            .parse_params()
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let result = self
            .handler
            .list_tools(params)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_tools_call(
        &self,
        request: &Request,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let params: CallToolParams = request
            .parse_params()
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        debug!(tool = %params.name, "Calling tool");

        let result = self.handler.call_tool(params).await.map_err(|e| {
            JsonRpcError::new(crate::error::MCP_TOOL_EXECUTION_ERROR, e.to_string())
        })?;

        serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_resources_list(
        &self,
        request: &Request,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let params: ListResourcesParams = request
            .parse_params()
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let result = self
            .handler
            .list_resources(params)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_resources_read(
        &self,
        request: &Request,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let params: ReadResourceParams = request
            .parse_params()
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        debug!(uri = %params.uri, "Reading resource");

        let result =
            self.handler.read_resource(params).await.map_err(|e| {
                JsonRpcError::new(crate::error::MCP_RESOURCE_READ_ERROR, e.to_string())
            })?;

        serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_resource_templates_list(
        &self,
        request: &Request,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let params: ListResourceTemplatesParams = request
            .parse_params()
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let result = self
            .handler
            .list_resource_templates(params)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_prompts_list(
        &self,
        request: &Request,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let params: ListPromptsParams = request
            .parse_params()
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        let result = self
            .handler
            .list_prompts(params)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_prompts_get(
        &self,
        request: &Request,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let params: GetPromptParams = request
            .parse_params()
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        debug!(name = %params.name, "Getting prompt");

        let result = self
            .handler
            .get_prompt(params)
            .await
            .map_err(|e| JsonRpcError::new(crate::error::MCP_PROMPT_ERROR, e.to_string()))?;

        serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn handle_set_log_level(
        &self,
        request: &Request,
    ) -> std::result::Result<serde_json::Value, JsonRpcError> {
        let params: SetLevelParams = request
            .parse_params()
            .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?;

        debug!(level = ?params.level, "Setting log level");

        self.handler
            .set_log_level(params)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))?;

        Ok(serde_json::json!({}))
    }
}

/// Send a notification through the transport.
pub async fn send_notification<T: Transport>(
    transport: &T,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<()> {
    let notification = Message::notification(method, params);
    transport.send(notification).await
}

/// Send a tools list changed notification.
pub async fn notify_tools_changed<T: Transport>(transport: &T) -> Result<()> {
    send_notification(transport, methods::NOTIFICATION_TOOLS_LIST_CHANGED, None).await
}

/// Send a resources list changed notification.
pub async fn notify_resources_changed<T: Transport>(transport: &T) -> Result<()> {
    send_notification(
        transport,
        methods::NOTIFICATION_RESOURCES_LIST_CHANGED,
        None,
    )
    .await
}

/// Send a prompts list changed notification.
pub async fn notify_prompts_changed<T: Transport>(transport: &T) -> Result<()> {
    send_notification(transport, methods::NOTIFICATION_PROMPTS_LIST_CHANGED, None).await
}

/// Send a resource updated notification.
pub async fn notify_resource_updated<T: Transport>(transport: &T, uri: &str) -> Result<()> {
    send_notification(
        transport,
        methods::NOTIFICATION_RESOURCES_UPDATED,
        Some(serde_json::json!({ "uri": uri })),
    )
    .await
}

/// Send a log message notification.
pub async fn send_log_message<T: Transport>(
    transport: &T,
    level: LogLevel,
    logger: Option<&str>,
    data: serde_json::Value,
) -> Result<()> {
    let params = LoggingMessage {
        level,
        logger: logger.map(String::from),
        data,
    };
    send_notification(
        transport,
        methods::NOTIFICATION_MESSAGE,
        Some(serde_json::to_value(params)?),
    )
    .await
}
