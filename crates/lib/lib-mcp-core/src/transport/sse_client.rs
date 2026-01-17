//! SSE (Server-Sent Events) client transport for MCP.
//!
//! This transport connects to an MCP server over HTTP using SSE
//! for server-to-client messages and POST requests for client-to-server.

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, trace, warn};

use super::{Transport, TransportConfig};
use crate::jsonrpc::Message;
use crate::{Error, Result};

/// SSE client transport configuration.
#[derive(Debug, Clone)]
pub struct SseClientConfig {
    /// Base URL of the MCP server.
    pub base_url: String,

    /// Optional authentication token.
    pub auth_token: Option<String>,

    /// Transport configuration.
    pub transport: TransportConfig,
}

impl SseClientConfig {
    /// Create a new SSE client configuration.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            auth_token: None,
            transport: TransportConfig::default(),
        }
    }

    /// Set the authentication token.
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }
}

/// SSE client transport.
pub struct SseClientTransport {
    config: SseClientConfig,
    client: Client,
    incoming_rx: Arc<Mutex<mpsc::Receiver<Message>>>,
    #[allow(dead_code)]
    incoming_tx: mpsc::Sender<Message>,
    endpoint_url: Arc<Mutex<Option<String>>>,
}

impl SseClientTransport {
    /// Create a new SSE client transport.
    pub async fn connect(config: SseClientConfig) -> Result<Self> {
        let client = Client::builder()
            .build()
            .map_err(|e| Error::Transport(e.to_string()))?;

        let (incoming_tx, incoming_rx) = mpsc::channel(100);

        let transport = Self {
            config,
            client,
            incoming_rx: Arc::new(Mutex::new(incoming_rx)),
            incoming_tx,
            endpoint_url: Arc::new(Mutex::new(None)),
        };

        // Start SSE connection
        transport.start_sse_listener().await?;

        Ok(transport)
    }

    async fn start_sse_listener(&self) -> Result<()> {
        let sse_url = format!("{}/sse", self.config.base_url);
        debug!(url = %sse_url, "Connecting to SSE endpoint");

        let mut request = self.client.get(&sse_url);

        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::Transport(format!(
                "SSE connection failed: {}",
                response.status()
            )));
        }

        // Spawn task to read SSE events
        let incoming_tx = self.incoming_tx.clone();
        let endpoint_url = self.endpoint_url.clone();
        let max_size = self.config.transport.max_message_size;

        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&text);

                        // Process complete SSE events
                        while let Some(pos) = buffer.find("\n\n") {
                            let event = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            if let Err(e) =
                                process_sse_event(&event, &incoming_tx, &endpoint_url, max_size)
                                    .await
                            {
                                error!(error = %e, "Error processing SSE event");
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Error reading SSE stream");
                        break;
                    }
                }
            }

            debug!("SSE stream ended");
        });

        Ok(())
    }

    async fn get_endpoint_url(&self) -> Result<String> {
        // Wait for endpoint URL from SSE
        let mut attempts = 0;
        loop {
            let url = self.endpoint_url.lock().await;
            if let Some(ref endpoint) = *url {
                return Ok(endpoint.clone());
            }
            drop(url);

            attempts += 1;
            if attempts > 50 {
                return Err(Error::Transport(
                    "Timeout waiting for endpoint URL".to_string(),
                ));
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

async fn process_sse_event(
    event: &str,
    incoming_tx: &mpsc::Sender<Message>,
    endpoint_url: &Arc<Mutex<Option<String>>>,
    max_size: usize,
) -> Result<()> {
    let mut event_type = "message";
    let mut data = String::new();

    for line in event.lines() {
        if line.starts_with("event:") {
            event_type = line[6..].trim();
        } else if line.starts_with("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(line[5..].trim());
        }
    }

    if data.is_empty() {
        return Ok(());
    }

    trace!(event_type = %event_type, data = %data, "Received SSE event");

    match event_type {
        "endpoint" => {
            // Server is telling us the endpoint URL for sending messages
            let mut url = endpoint_url.lock().await;
            *url = Some(data);
            debug!(endpoint = %url.as_ref().unwrap(), "Received endpoint URL");
        }
        "message" => {
            if data.len() > max_size {
                return Err(Error::InvalidMessage(format!(
                    "Message too large: {} bytes",
                    data.len()
                )));
            }

            let message: Message = serde_json::from_str(&data)?;
            if incoming_tx.send(message).await.is_err() {
                warn!("Failed to send message to incoming channel");
            }
        }
        _ => {
            debug!(event_type = %event_type, "Unknown SSE event type");
        }
    }

    Ok(())
}

#[async_trait]
impl Transport for SseClientTransport {
    async fn send(&self, message: Message) -> Result<()> {
        let endpoint = self.get_endpoint_url().await?;
        let json = serde_json::to_string(&message)?;

        trace!(endpoint = %endpoint, msg = %json, "Sending message via POST");

        let mut request = self.client.post(&endpoint).body(json.clone());

        request = request.header("Content-Type", "application/json");

        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| Error::Transport(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Transport(format!(
                "POST request failed: {} - {}",
                status, body
            )));
        }

        debug!("Message sent successfully");
        Ok(())
    }

    async fn receive(&self) -> Result<Option<Message>> {
        let mut rx = self.incoming_rx.lock().await;
        match rx.recv().await {
            Some(msg) => {
                debug!(method = ?msg.method(), "Received message");
                Ok(Some(msg))
            }
            None => {
                debug!("Incoming channel closed");
                Ok(None)
            }
        }
    }

    async fn close(&self) -> Result<()> {
        // SSE connections are typically closed by dropping
        debug!("Closing SSE client transport");
        Ok(())
    }
}

/// Helper to create SSE client transport with builder pattern.
pub struct SseClientBuilder {
    config: SseClientConfig,
}

impl SseClientBuilder {
    /// Create a new builder with the server URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            config: SseClientConfig::new(base_url),
        }
    }

    /// Set authentication token.
    pub fn auth_token(mut self, token: impl Into<String>) -> Self {
        self.config.auth_token = Some(token.into());
        self
    }

    /// Set maximum message size.
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.config.transport.max_message_size = size;
        self
    }

    /// Connect and create the transport.
    pub async fn connect(self) -> Result<SseClientTransport> {
        SseClientTransport::connect(self.config).await
    }
}
