//! Transport layer abstraction
//!
//! Defines the interface for sync transport implementations.
//! Actual transports (WebSocket, peer-to-peer, etc.) implement this trait.

use crate::{DeviceId, SyncMessage};
use std::error::Error;
use std::fmt;

/// Result type for transport operations
pub type TransportResult<T> = Result<T, TransportError>;

/// Transport layer errors
#[derive(Debug, Clone)]
pub enum TransportError {
    PeerNotConnected,
    EncodingFailed(String),
    DecodingFailed(String),
    SendFailed(String),
    NotStarted,
    Other(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::PeerNotConnected => write!(f, "Peer is not connected"),
            TransportError::EncodingFailed(msg) => write!(f, "Failed to encode message: {}", msg),
            TransportError::DecodingFailed(msg) => write!(f, "Failed to decode message: {}", msg),
            TransportError::SendFailed(msg) => write!(f, "Failed to send message: {}", msg),
            TransportError::NotStarted => write!(f, "Transport not started"),
            TransportError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl Error for TransportError {}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub device_id: DeviceId,
    pub display_name: String,
    pub app_version: String,
}

/// Event callbacks for transport layer
pub trait TransportDelegate: Send + Sync {
    /// A new peer was discovered
    fn on_peer_discovered(&mut self, peer: PeerInfo);

    /// Successfully connected to a peer
    fn on_peer_connected(&mut self, peer: PeerInfo);

    /// Disconnected from a peer
    fn on_peer_disconnected(&mut self, peer: PeerInfo);

    /// Received a message from a peer
    fn on_message_received(&mut self, message: SyncMessage, from: PeerInfo);

    /// Error occurred
    fn on_error(&mut self, error: TransportError);
}

/// Transport layer interface
pub trait TransportLayer: Send + Sync {
    /// Local device ID
    fn device_id(&self) -> DeviceId;

    /// Start discovering and accepting peers
    fn start_discovery(&mut self) -> TransportResult<()>;

    /// Stop discovery
    fn stop_discovery(&mut self) -> TransportResult<()>;

    /// Send message to specific peer
    fn send(&mut self, message: SyncMessage, to: DeviceId) -> TransportResult<()>;

    /// Broadcast message to all connected peers
    fn broadcast(&mut self, message: SyncMessage) -> TransportResult<()>;

    /// Get list of connected peers
    fn connected_peers(&self) -> Vec<PeerInfo>;

    /// Set delegate for events
    fn set_delegate(&mut self, delegate: Box<dyn TransportDelegate>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_error_display() {
        let err = TransportError::PeerNotConnected;
        assert_eq!(err.to_string(), "Peer is not connected");

        let err = TransportError::SendFailed("timeout".to_string());
        assert_eq!(err.to_string(), "Failed to send message: timeout");
    }
}
