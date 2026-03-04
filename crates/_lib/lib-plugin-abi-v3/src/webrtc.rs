//! WebRTC handlers trait for plugins
//!
//! Plugins can implement WebRTC message handlers for peer-to-peer communication.
//! Signaling is managed by the CLI core; plugins only handle data channels.

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// WebRTC handlers trait
///
/// Plugins implementing this trait can handle WebRTC peer connections.
/// The CLI core manages signaling; plugins receive connected peers and messages.
///
/// # Example
///
/// ```rust,ignore
/// #[async_trait]
/// impl WebRtcHandlers for TasksPlugin {
///     async fn on_connect(&self, peer: Peer) -> Result<()> {
///         println!("Peer connected: {}", peer.id);
///         Ok(())
///     }
///
///     async fn on_message(&self, peer: Peer, msg: Message) -> Result<()> {
///         match msg.channel.as_str() {
///             "tasks" => self.handle_task_message(peer, msg).await,
///             _ => Ok(())
///         }
///     }
///
///     async fn on_disconnect(&self, peer: Peer) -> Result<()> {
///         println!("Peer disconnected: {}", peer.id);
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait WebRtcHandlers: Plugin {
    /// Called when a peer connects
    async fn on_connect(&self, peer: Peer) -> Result<()> {
        let _ = peer;
        Ok(())
    }

    /// Called when a message is received from a peer
    async fn on_message(&self, peer: Peer, msg: Message) -> Result<()> {
        let _ = (peer, msg);
        Ok(())
    }

    /// Called when a peer disconnects
    async fn on_disconnect(&self, peer: Peer) -> Result<()> {
        let _ = peer;
        Ok(())
    }

    /// Check if this plugin has WebRTC handlers
    fn has_webrtc_handlers(&self) -> bool {
        true
    }
}

/// WebRTC peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    /// Unique peer identifier
    pub id: String,

    /// Peer display name (if available)
    pub name: Option<String>,

    /// Connection metadata
    pub metadata: serde_json::Value,
}

impl Peer {
    /// Create a new peer
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// Set peer name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// WebRTC message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Data channel name
    pub channel: String,

    /// Message type identifier
    pub msg_type: String,

    /// Message payload
    pub payload: Vec<u8>,

    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

impl Message {
    /// Create a new message
    pub fn new(channel: impl Into<String>, msg_type: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            channel: channel.into(),
            msg_type: msg_type.into(),
            payload,
            metadata: None,
        }
    }

    /// Create a text message
    pub fn text(channel: impl Into<String>, msg_type: impl Into<String>, text: &str) -> Self {
        Self::new(channel, msg_type, text.as_bytes().to_vec())
    }

    /// Create a JSON message
    pub fn json<T: Serialize>(
        channel: impl Into<String>,
        msg_type: impl Into<String>,
        data: &T,
    ) -> Result<Self> {
        let payload = serde_json::to_vec(data)?;
        Ok(Self::new(channel, msg_type, payload))
    }

    /// Parse payload as JSON
    pub fn parse_json<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        Ok(serde_json::from_slice(&self.payload)?)
    }

    /// Get payload as string
    pub fn as_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.payload)
            .map_err(|e| crate::PluginError::Runtime(e.to_string()))
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}
