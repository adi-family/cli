//! # Signaling Protocol
//!
//! WebSocket message protocol for ADI signaling infrastructure.
//! Auto-generated from `signaling.tsp` via `lib-typespec-api` protocol codegen.

include!(concat!(env!("OUT_DIR"), "/generated_protocol.rs"));

pub use messages::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_serialization() {
        let msg = SignalingMessage::DeviceRegister {
            secret: "test-secret-with-at-least-32-chars-for-validation".to_string(),
            device_id: None,
            version: "0.2.1".to_string(),
            tags: Some(std::collections::HashMap::from([("kind".into(), "desktop".into())])),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"device_register\""));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::DeviceRegister {
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
            tags: std::collections::HashMap::from([("kind".into(), "desktop".into())]),
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
        let msg = SignalingMessage::DeviceQueryDevices {
            tag_filter: std::collections::HashMap::from([("kind".into(), "desktop".into())]),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"device_query_devices\""));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::DeviceQueryDevices { tag_filter } => {
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
