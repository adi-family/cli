//! # Signaling Protocol
//!
//! WebSocket message protocol for ADI signaling infrastructure.
//! Handles device registration, authentication, pairing, and data sync.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

pub use serde;
pub use serde_json;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthRequirement {
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthOption {
    Verified,
    Anonymous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub manual_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub tags: HashMap<String, String>,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    Hello {
        auth_kind: String,
        auth_domain: String,
        auth_requirement: AuthRequirement,
        auth_options: Vec<AuthOption>,
    },

    Authenticate {
        access_token: String,
    },

    Authenticated {
        user_id: String,
    },

    HelloAuthed {
        user_id: String,
        connection_info: ConnectionInfo,
    },

    Register {
        secret: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        device_id: Option<String>,
        version: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<HashMap<String, String>>,
    },

    Registered {
        device_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<HashMap<String, String>>,
    },

    Deregister {
        device_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    Deregistered {
        device_id: String,
    },

    CreatePairingCode,

    PairingCode {
        code: String,
    },

    UsePairingCode {
        code: String,
    },

    Paired {
        peer_id: String,
    },

    PairingFailed {
        reason: String,
    },

    SyncData {
        payload: JsonValue,
    },

    PeerConnected {
        peer_id: String,
    },

    PeerDisconnected {
        peer_id: String,
    },

    UpdateTags {
        tags: HashMap<String, String>,
    },

    TagsUpdated {
        device_id: String,
        tags: HashMap<String, String>,
    },

    QueryDevices {
        tag_filter: HashMap<String, String>,
    },

    DeviceList {
        devices: Vec<DeviceInfo>,
    },

    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_serialization() {
        let msg = SignalingMessage::Register {
            secret: "test-secret-with-at-least-32-chars-for-validation".to_string(),
            device_id: None,
            version: "0.2.1".to_string(),
            tags: Some(HashMap::from([("kind".into(), "desktop".into())])),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            SignalingMessage::Register {
                secret,
                device_id,
                version,
                tags,
            } => {
                assert_eq!(secret, "test-secret-with-at-least-32-chars-for-validation");
                assert_eq!(device_id, None);
                assert_eq!(version, "0.2.1");
                assert_eq!(tags.unwrap()["kind"], "desktop");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_device_info_serialization() {
        let device = DeviceInfo {
            device_id: "dev-123".to_string(),
            tags: HashMap::from([("kind".into(), "desktop".into())]),
            online: true,
        };

        let json = serde_json::to_string(&device).unwrap();
        let deserialized: DeviceInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.device_id, "dev-123");
        assert!(deserialized.online);
        assert_eq!(deserialized.tags["kind"], "desktop");
    }

    #[test]
    fn test_query_devices_serialization() {
        let msg = SignalingMessage::QueryDevices {
            tag_filter: HashMap::from([("kind".into(), "desktop".into())]),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"query_devices\""));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::QueryDevices { tag_filter } => {
                assert_eq!(tag_filter["kind"], "desktop");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_connection_info_serialization() {
        let info = ConnectionInfo {
            manual_allowed: true,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ConnectionInfo = serde_json::from_str(&json).unwrap();
        assert!(deserialized.manual_allowed);
    }
}
