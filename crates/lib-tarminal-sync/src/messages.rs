//! Sync protocol messages
//!
//! Core message types for the Tarminal synchronization protocol.
//! All messages are JSON-serializable for cross-platform compatibility.

use crate::{DeviceId, SyncMetadata};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Messages exchanged between peers during synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    /// Initial handshake with device info
    Hello {
        device_id: DeviceId,
        display_name: String,
        app_version: String,
        protocol_version: u32,
    },

    /// Request full state sync
    RequestFullSync,

    /// Full state response
    FullState { state: AppState },

    /// Incremental workspace update
    WorkspaceUpdate { workspace: SyncableWorkspace },

    /// Incremental session update
    SessionUpdate { session: SyncableSession },

    /// Incremental command block update
    CommandBlockUpdate { block: SyncableCommandBlock },

    /// Delete notification (tombstone)
    Delete {
        entity_type: EntityType,
        entity_id: Uuid,
        deleted_by: DeviceId,
        deleted_at: chrono::DateTime<chrono::Utc>,
    },

    /// Acknowledgment
    Ack { message_id: Uuid },

    /// Ping for keepalive
    Ping,

    /// Pong response
    Pong,
}

/// Entity types for delete operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Workspace,
    Session,
    CommandBlock,
}

/// Complete application state for full sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub workspaces: Vec<SyncableWorkspace>,
    pub sessions: Vec<SyncableSession>,
    pub command_blocks: Vec<SyncableCommandBlock>,
}

/// Syncable workspace entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncableWorkspace {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub session_ids: Vec<Uuid>,
    pub active_session_id: Option<Uuid>,
    pub sync_metadata: SyncMetadata,
}

/// Syncable session entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncableSession {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub title: String,
    pub command_block_ids: Vec<Uuid>,
    pub current_directory: String,
    pub session_type: SessionType,
    pub sync_metadata: SyncMetadata,
}

/// Session type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    /// Block-based terminal (command + output blocks)
    BlockBased,
    /// Full PTY terminal (interactive shell)
    Interactive,
}

/// Syncable command block entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncableCommandBlock {
    pub id: Uuid,
    pub session_id: Uuid,
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_metadata: SyncMetadata,
}

/// Signaling server messages for device pairing and relay
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    /// Register device with server using client secret
    /// Server derives deterministic device_id from secret using HMAC
    /// On reconnect, device_id must match derived ID (prevents secret theft attacks)
    Register {
        secret: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        device_id: Option<String>,
    },

    /// Registration confirmed with derived device ID
    /// Same secret always produces same device_id (persistent sessions)
    Registered { device_id: String },

    /// Create a pairing code
    CreatePairingCode,

    /// Pairing code generated
    PairingCode { code: String },

    /// Use a pairing code to connect
    UsePairingCode { code: String },

    /// Successfully paired with peer
    Paired { peer_id: String },

    /// Pairing failed
    PairingFailed { reason: String },

    /// Sync data payload (forwarded as-is)
    SyncData { payload: JsonValue },

    /// Peer came online
    PeerConnected { peer_id: String },

    /// Peer went offline
    PeerDisconnected { peer_id: String },

    /// Error message
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_message_serialization() {
        let msg = SyncMessage::Hello {
            device_id: Uuid::new_v4(),
            display_name: "Test Device".to_string(),
            app_version: "1.0".to_string(),
            protocol_version: 1,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: SyncMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            SyncMessage::Hello { display_name, .. } => {
                assert_eq!(display_name, "Test Device");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_signaling_message_serialization() {
        let msg = SignalingMessage::Register {
            secret: "test-secret-with-at-least-32-chars-for-validation".to_string(),
            device_id: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            SignalingMessage::Register { secret, device_id } => {
                assert_eq!(secret, "test-secret-with-at-least-32-chars-for-validation");
                assert_eq!(device_id, None);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
