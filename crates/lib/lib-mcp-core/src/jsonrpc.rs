//! JSON-RPC 2.0 message types for MCP.
//!
//! MCP uses JSON-RPC 2.0 as its wire protocol. This module provides
//! the message types and serialization logic.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC version string.
pub const JSONRPC_VERSION: &str = "2.0";

/// A JSON-RPC request ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// Numeric ID.
    Number(i64),
    /// String ID.
    String(String),
}

impl From<i64> for RequestId {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{}", n),
            Self::String(s) => write!(f, "{}", s),
        }
    }
}

/// A JSON-RPC message (request, response, or notification).
///
/// Note: The order of variants matters for `#[serde(untagged)]` deserialization.
/// - Request: has `id` and `method` (both required)
/// - Notification: has `method` but no `id`
/// - Response: has `id` and (`result` or `error`), no `method`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    /// A request expecting a response (has `id` and `method`).
    Request(Request),
    /// A notification (no response expected, has `method` but no `id`).
    Notification(Notification),
    /// A response to a request (has `id` but no `method`).
    Response(Response),
}

impl Message {
    /// Create a request message.
    pub fn request(
        id: impl Into<RequestId>,
        method: impl Into<String>,
        params: Option<Value>,
    ) -> Self {
        Self::Request(Request::new(id, method, params))
    }

    /// Create a notification message.
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self::Notification(Notification::new(method, params))
    }

    /// Create a success response.
    pub fn response_success(id: RequestId, result: Value) -> Self {
        Self::Response(Response::success(id, result))
    }

    /// Create an error response.
    pub fn response_error(id: Option<RequestId>, error: JsonRpcError) -> Self {
        Self::Response(Response::error(id, error))
    }

    /// Get the method name if this is a request or notification.
    pub fn method(&self) -> Option<&str> {
        match self {
            Self::Request(r) => Some(&r.method),
            Self::Notification(n) => Some(&n.method),
            Self::Response(_) => None,
        }
    }
}

/// A JSON-RPC request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// Request ID.
    pub id: RequestId,

    /// Method name.
    pub method: String,

    /// Method parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Request {
    /// Create a new request.
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }

    /// Parse params as a specific type.
    pub fn parse_params<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        match &self.params {
            Some(v) => serde_json::from_value(v.clone()),
            None => serde_json::from_value(Value::Object(Default::default())),
        }
    }
}

/// A JSON-RPC notification (request without ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// Method name.
    pub method: String,

    /// Method parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Notification {
    /// Create a new notification.
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }

    /// Parse params as a specific type.
    pub fn parse_params<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        match &self.params {
            Some(v) => serde_json::from_value(v.clone()),
            None => serde_json::from_value(Value::Object(Default::default())),
        }
    }
}

/// A JSON-RPC response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,

    /// Request ID this is responding to.
    /// May be null for errors that couldn't be associated with a request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,

    /// Result (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl Response {
    /// Create a success response.
    pub fn success(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: Option<RequestId>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Check if this is a success response.
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Get the result, returning an error if this is an error response.
    pub fn into_result(self) -> Result<Value, JsonRpcError> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(self.result.unwrap_or(Value::Null)),
        }
    }

    /// Parse result as a specific type.
    pub fn parse_result<T: for<'de> Deserialize<'de>>(&self) -> Result<T, crate::Error> {
        if let Some(e) = &self.error {
            return Err(crate::Error::JsonRpc {
                code: e.code,
                message: e.message.clone(),
                data: e.data.clone(),
            });
        }
        let value = self.result.clone().unwrap_or(Value::Null);
        serde_json::from_value(value).map_err(Into::into)
    }
}

/// A JSON-RPC error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i32,

    /// Human-readable error message.
    pub message: String,

    /// Additional error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Create a new error.
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create an error with additional data.
    pub fn with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Parse error (invalid JSON).
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(crate::error::JSON_RPC_PARSE_ERROR, message)
    }

    /// Invalid request error.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(crate::error::JSON_RPC_INVALID_REQUEST, message)
    }

    /// Method not found error.
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(
            crate::error::JSON_RPC_METHOD_NOT_FOUND,
            format!("Method not found: {}", method.into()),
        )
    }

    /// Invalid params error.
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(crate::error::JSON_RPC_INVALID_PARAMS, message)
    }

    /// Internal error.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(crate::error::JSON_RPC_INTERNAL_ERROR, message)
    }
}

impl From<crate::Error> for JsonRpcError {
    fn from(err: crate::Error) -> Self {
        Self::new(err.to_json_rpc_code(), err.to_string())
    }
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = Request::new(1, "test/method", Some(serde_json::json!({"key": "value"})));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"test/method\""));
    }

    #[test]
    fn test_response_success() {
        let resp = Response::success(RequestId::Number(1), serde_json::json!({"result": "ok"}));
        assert!(resp.is_success());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_response_error() {
        let resp = Response::error(
            Some(RequestId::Number(1)),
            JsonRpcError::method_not_found("unknown"),
        );
        assert!(!resp.is_success());
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_notification_no_id() {
        let notif = Notification::new("test/notify", None);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(!json.contains("\"id\""));
    }
}
