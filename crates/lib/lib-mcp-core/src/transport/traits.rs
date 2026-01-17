//! Transport traits for MCP communication.

use async_trait::async_trait;

use crate::jsonrpc::Message;
use crate::Result;

/// A transport that can send and receive MCP messages.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a message through the transport.
    async fn send(&self, message: Message) -> Result<()>;

    /// Receive the next message from the transport.
    ///
    /// Returns `None` when the transport is closed.
    async fn receive(&self) -> Result<Option<Message>>;

    /// Close the transport.
    async fn close(&self) -> Result<()>;
}

/// A transport that can be split into separate send and receive halves.
pub trait SplitTransport: Transport {
    type Sender: TransportSender;
    type Receiver: TransportReceiver;

    /// Split the transport into sender and receiver halves.
    fn split(self) -> (Self::Sender, Self::Receiver);
}

/// The sending half of a split transport.
#[async_trait]
pub trait TransportSender: Send + Sync {
    /// Send a message.
    async fn send(&self, message: Message) -> Result<()>;

    /// Close the sender.
    async fn close(&self) -> Result<()>;
}

/// The receiving half of a split transport.
#[async_trait]
pub trait TransportReceiver: Send + Sync {
    /// Receive the next message.
    async fn receive(&self) -> Result<Option<Message>>;
}

/// Configuration for a transport.
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Maximum message size in bytes.
    pub max_message_size: usize,

    /// Read timeout in milliseconds.
    pub read_timeout_ms: Option<u64>,

    /// Write timeout in milliseconds.
    pub write_timeout_ms: Option<u64>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10 MB
            read_timeout_ms: None,
            write_timeout_ms: None,
        }
    }
}
