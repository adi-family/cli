use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::{broadcast, mpsc};

pub mod protocol {
    pub mod types {
        pub use crate::{AdiMethodInfo, AdiPluginCapabilities, AdiPluginInfo};
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiPluginCapabilities {
    pub subscriptions: bool,
    pub notifications: bool,
    pub streaming: bool,
}

impl Default for AdiPluginCapabilities {
    fn default() -> Self {
        Self {
            subscriptions: false,
            notifications: false,
            streaming: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiMethodInfo {
    pub name: String,
    pub description: String,
    pub streaming: bool,
    pub params_schema: Option<JsonValue>,
    pub result_schema: Option<JsonValue>,
    pub deprecated: Option<bool>,
    pub deprecated_message: Option<String>,
}

impl Default for AdiMethodInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            streaming: false,
            params_schema: None,
            result_schema: None,
            deprecated: None,
            deprecated_message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiPluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub methods: Vec<AdiMethodInfo>,
    pub capabilities: AdiPluginCapabilities,
}

/// Result of handling a service request.
pub enum AdiHandleResult {
    /// Single response with opaque payload bytes
    Success(Bytes),
    /// Streaming response — receiver yields (chunk_bytes, is_final)
    Stream(mpsc::Receiver<(Bytes, bool)>),
}

#[derive(Debug, Clone)]
pub struct AdiServiceError {
    pub code: String,
    pub message: String,
}

impl AdiServiceError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self { code: "not_found".to_string(), message: message.into() }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self { code: "invalid_params".to_string(), message: message.into() }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self { code: "internal".to_string(), message: message.into() }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self { code: "method_not_found".to_string(), message: format!("Method '{}' not found", method) }
    }

    pub fn not_supported(message: impl Into<String>) -> Self {
        Self { code: "not_supported".to_string(), message: message.into() }
    }

    pub fn subscription_failed(message: impl Into<String>) -> Self {
        Self { code: "subscription_failed".to_string(), message: message.into() }
    }

    /// Serialize this error to JSON bytes for use as a frame payload.
    pub fn to_payload(&self) -> Bytes {
        let json = serde_json::json!({ "code": self.code, "message": self.message });
        Bytes::from(serde_json::to_vec(&json).unwrap())
    }
}

/// Caller identity resolved from the signaling session
#[derive(Debug, Clone)]
pub struct AdiCallerContext {
    pub user_id: Option<String>,
    pub device_id: Option<String>,
}

impl AdiCallerContext {
    pub fn anonymous() -> Self {
        Self { user_id: None, device_id: None }
    }

    pub fn require_user_id(&self) -> Result<&str, AdiServiceError> {
        self.user_id.as_deref().ok_or_else(|| {
            AdiServiceError::new("unauthorized", "No authenticated user. Cocoon must be claimed via setup_token.")
        })
    }
}

#[derive(Debug, Clone)]
pub struct SubscriptionEventInfo {
    pub name: String,
    pub description: String,
    pub data_schema: Option<JsonValue>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionEvent {
    pub event: String,
    pub data: JsonValue,
}

/// Trait that plugins implement to handle requests.
#[async_trait]
pub trait AdiService: Send + Sync {
    fn plugin_id(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> Option<&str> { None }
    fn methods(&self) -> Vec<AdiMethodInfo>;
    fn capabilities(&self) -> AdiPluginCapabilities { AdiPluginCapabilities::default() }

    /// Handle a request with opaque bytes payload.
    async fn handle(
        &self,
        ctx: &AdiCallerContext,
        method: &str,
        payload: Bytes,
    ) -> Result<AdiHandleResult, AdiServiceError>;

    fn subscription_events(&self) -> Vec<SubscriptionEventInfo> { vec![] }

    async fn subscribe(
        &self,
        _event: &str,
        _filter: Option<JsonValue>,
    ) -> Result<broadcast::Receiver<SubscriptionEvent>, AdiServiceError> {
        Err(AdiServiceError::not_supported("subscriptions not supported"))
    }

    fn on_client_connected(&self, _client_id: &str) {}
    fn on_client_disconnected(&self, _client_id: &str) {}
}

pub fn create_stream_channel(buffer_size: usize) -> (StreamSender, mpsc::Receiver<(Bytes, bool)>) {
    let (tx, rx) = mpsc::channel(buffer_size);
    (StreamSender { tx }, rx)
}

pub struct StreamSender {
    tx: mpsc::Sender<(Bytes, bool)>,
}

impl StreamSender {
    /// Send a chunk (not final).
    pub async fn send(&self, data: Bytes) -> Result<(), ()> {
        self.tx.send((data, false)).await.map_err(|_| ())
    }

    pub async fn send_final(&self, data: Bytes) -> Result<(), ()> {
        self.tx.send((data, true)).await.map_err(|_| ())
    }

    /// Close the stream without sending a final value.
    pub fn close(self) {
        drop(self.tx);
    }
}
