//! Stdio transport for MCP.
//!
//! This transport uses standard input/output for communication,
//! with each message on a separate line (newline-delimited JSON).

use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tracing::{debug, trace};

use super::{Transport, TransportConfig, TransportReceiver, TransportSender};
use crate::jsonrpc::Message;
use crate::{Error, Result};

/// Stdio transport using stdin/stdout.
pub struct StdioTransport {
    reader: Arc<Mutex<BufReader<tokio::io::Stdin>>>,
    writer: Arc<Mutex<tokio::io::Stdout>>,
    config: TransportConfig,
}

impl StdioTransport {
    /// Create a new stdio transport with default configuration.
    pub fn new() -> Self {
        Self::with_config(TransportConfig::default())
    }

    /// Create a new stdio transport with custom configuration.
    pub fn with_config(config: TransportConfig) -> Self {
        Self {
            reader: Arc::new(Mutex::new(BufReader::new(tokio::io::stdin()))),
            writer: Arc::new(Mutex::new(tokio::io::stdout())),
            config,
        }
    }

    /// Split into sender and receiver.
    pub fn split(self) -> (StdioSender, StdioReceiver) {
        (
            StdioSender {
                writer: self.writer,
            },
            StdioReceiver {
                reader: self.reader,
                config: self.config,
            },
        )
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&self, message: Message) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        trace!(msg = %json, "Sending message");

        let mut writer = self.writer.lock().await;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        debug!("Message sent");
        Ok(())
    }

    async fn receive(&self) -> Result<Option<Message>> {
        let mut reader = self.reader.lock().await;
        let mut line = String::new();

        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            debug!("EOF reached on stdin");
            return Ok(None);
        }

        let line = line.trim();
        if line.is_empty() {
            // Skip empty lines
            return self.receive().await;
        }

        if line.len() > self.config.max_message_size {
            return Err(Error::InvalidMessage(format!(
                "Message too large: {} bytes (max: {})",
                line.len(),
                self.config.max_message_size
            )));
        }

        trace!(msg = %line, "Received message");
        let message: Message = serde_json::from_str(line)?;
        debug!(method = ?message.method(), "Message parsed");

        Ok(Some(message))
    }

    async fn close(&self) -> Result<()> {
        // Stdio can't really be closed, just flush
        let mut writer = self.writer.lock().await;
        writer.flush().await?;
        Ok(())
    }
}

/// Sending half of stdio transport.
pub struct StdioSender {
    writer: Arc<Mutex<tokio::io::Stdout>>,
}

#[async_trait]
impl TransportSender for StdioSender {
    async fn send(&self, message: Message) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        trace!(msg = %json, "Sending message");

        let mut writer = self.writer.lock().await;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        Ok(())
    }

    async fn close(&self) -> Result<()> {
        let mut writer = self.writer.lock().await;
        writer.flush().await?;
        Ok(())
    }
}

/// Receiving half of stdio transport.
pub struct StdioReceiver {
    reader: Arc<Mutex<BufReader<tokio::io::Stdin>>>,
    config: TransportConfig,
}

#[async_trait]
impl TransportReceiver for StdioReceiver {
    async fn receive(&self) -> Result<Option<Message>> {
        let mut reader = self.reader.lock().await;
        let mut line = String::new();

        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Ok(None);
        }

        let line = line.trim();
        if line.is_empty() {
            drop(reader);
            return self.receive().await;
        }

        if line.len() > self.config.max_message_size {
            return Err(Error::InvalidMessage(format!(
                "Message too large: {} bytes (max: {})",
                line.len(),
                self.config.max_message_size
            )));
        }

        let message: Message = serde_json::from_str(line)?;
        Ok(Some(message))
    }
}

/// Stdio transport using custom readers/writers.
///
/// This is useful for testing or when you want to use different streams.
pub struct CustomStdioTransport<R, W> {
    reader: Arc<Mutex<BufReader<R>>>,
    writer: Arc<Mutex<W>>,
    config: TransportConfig,
}

impl<R, W> CustomStdioTransport<R, W>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    /// Create a new custom stdio transport.
    pub fn new(reader: R, writer: W) -> Self {
        Self::with_config(reader, writer, TransportConfig::default())
    }

    /// Create a new custom stdio transport with configuration.
    pub fn with_config(reader: R, writer: W, config: TransportConfig) -> Self {
        Self {
            reader: Arc::new(Mutex::new(BufReader::new(reader))),
            writer: Arc::new(Mutex::new(writer)),
            config,
        }
    }
}

#[async_trait]
impl<R, W> Transport for CustomStdioTransport<R, W>
where
    R: tokio::io::AsyncRead + Unpin + Send + Sync + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + Sync + 'static,
{
    async fn send(&self, message: Message) -> Result<()> {
        let json = serde_json::to_string(&message)?;

        let mut writer = self.writer.lock().await;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        Ok(())
    }

    async fn receive(&self) -> Result<Option<Message>> {
        let mut reader = self.reader.lock().await;
        let mut line = String::new();

        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Ok(None);
        }

        let line = line.trim();
        if line.is_empty() {
            drop(reader);
            return self.receive().await;
        }

        if line.len() > self.config.max_message_size {
            return Err(Error::InvalidMessage(format!(
                "Message too large: {} bytes (max: {})",
                line.len(),
                self.config.max_message_size
            )));
        }

        let message: Message = serde_json::from_str(line)?;
        Ok(Some(message))
    }

    async fn close(&self) -> Result<()> {
        let mut writer = self.writer.lock().await;
        writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn test_custom_stdio_transport() {
        // Create a bidirectional pipe: client writes to one end, server reads from the other
        let (client_to_server, server_from_client) = duplex(1024);
        let (server_to_client, client_from_server) = duplex(1024);

        let client = CustomStdioTransport::new(client_from_server, client_to_server);
        let server = CustomStdioTransport::new(server_from_client, server_to_client);

        // Send from client to server
        let msg = Message::notification("test/notify", None);
        client.send(msg).await.unwrap();

        // Receive on server
        let received = server.receive().await.unwrap().unwrap();
        assert_eq!(received.method(), Some("test/notify"));
    }
}
