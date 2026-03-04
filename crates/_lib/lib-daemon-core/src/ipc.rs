//! IPC protocol traits and utilities

use serde::{Deserialize, Serialize};

/// Trait for daemon request/response protocol
///
/// Implement this trait for your daemon's request types to define
/// their corresponding response types.
///
/// # Example
///
/// ```
/// use serde::{Serialize, Deserialize};
/// use lib_daemon_core::DaemonProtocol;
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// enum MyRequest {
///     GetStatus,
///     SetValue { key: String, value: String },
///     Shutdown,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// enum MyResponse {
///     Status { uptime: u64 },
///     Ok { message: String },
///     Error { code: String, message: String },
/// }
///
/// impl DaemonProtocol for MyRequest {
///     type Response = MyResponse;
/// }
/// ```
pub trait DaemonProtocol: Serialize + for<'de> Deserialize<'de> {
    /// The response type for this request
    type Response: Serialize + for<'de> Deserialize<'de>;
}

/// Helper trait for encoding/decoding messages over the wire
pub trait MessageCodec: Sized {
    /// Encode message to JSON string
    fn encode(&self) -> serde_json::Result<String>
    where
        Self: Serialize,
    {
        serde_json::to_string(self)
    }

    /// Decode message from JSON string
    fn decode(data: &str) -> serde_json::Result<Self>
    where
        Self: for<'de> Deserialize<'de>,
    {
        serde_json::from_str(data)
    }
}

// Blanket implementation for all types that implement Serialize + Deserialize
impl<T> MessageCodec for T where T: Serialize + for<'de> Deserialize<'de> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    enum TestRequest {
        Ping,
        GetValue { key: String },
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    enum TestResponse {
        Pong,
        Value { value: String },
        Error { message: String },
    }

    impl DaemonProtocol for TestRequest {
        type Response = TestResponse;
    }

    #[test]
    fn test_message_encode_decode() {
        let request = TestRequest::GetValue {
            key: "test".to_string(),
        };

        // Encode
        let encoded = request.encode().unwrap();
        assert!(encoded.contains("GetValue") || encoded.contains("get_value"));
        assert!(encoded.contains("test"));

        // Decode
        let decoded: TestRequest = TestRequest::decode(&encoded).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn test_response_encode_decode() {
        let response = TestResponse::Value {
            value: "hello".to_string(),
        };

        let encoded = response.encode().unwrap();
        let decoded: TestResponse = TestResponse::decode(&encoded).unwrap();
        assert_eq!(decoded, response);
    }
}
