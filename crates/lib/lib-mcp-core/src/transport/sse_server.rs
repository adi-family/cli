//! SSE (Server-Sent Events) server transport for MCP.
//!
//! This module provides axum handlers for hosting an MCP server over HTTP
//! using SSE for server-to-client messages.

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{debug, trace};
use uuid::Uuid;

use crate::jsonrpc::Message;
use crate::{Error, Result};

/// State for the SSE server transport.
#[derive(Clone)]
pub struct SseServerState {
    inner: Arc<SseServerStateInner>,
}

struct SseServerStateInner {
    /// Broadcast channel for sending messages to all connected clients.
    outgoing_tx: broadcast::Sender<Message>,

    /// Channel for receiving messages from clients.
    incoming_tx: mpsc::Sender<(String, Message)>,
    incoming_rx: Mutex<mpsc::Receiver<(String, Message)>>,

    /// Connected clients (session_id -> endpoint_url).
    sessions: Mutex<std::collections::HashMap<String, String>>,

    /// Maximum message size.
    max_message_size: usize,
}

impl SseServerState {
    /// Create a new SSE server state.
    pub fn new() -> Self {
        Self::with_max_message_size(10 * 1024 * 1024) // 10 MB default
    }

    /// Create a new SSE server state with custom max message size.
    pub fn with_max_message_size(max_message_size: usize) -> Self {
        let (outgoing_tx, _) = broadcast::channel(100);
        let (incoming_tx, incoming_rx) = mpsc::channel(100);

        Self {
            inner: Arc::new(SseServerStateInner {
                outgoing_tx,
                incoming_tx,
                incoming_rx: Mutex::new(incoming_rx),
                sessions: Mutex::new(std::collections::HashMap::new()),
                max_message_size,
            }),
        }
    }

    /// Send a message to all connected clients.
    pub fn broadcast(&self, message: Message) -> Result<()> {
        self.inner
            .outgoing_tx
            .send(message)
            .map_err(|_| Error::ChannelSend)?;
        Ok(())
    }

    /// Receive a message from any client.
    ///
    /// Returns `(session_id, message)`.
    pub async fn receive(&self) -> Result<Option<(String, Message)>> {
        let mut rx = self.inner.incoming_rx.lock().await;
        Ok(rx.recv().await)
    }

    /// Get a stream of incoming messages.
    pub fn incoming_stream(&self) -> impl Stream<Item = (String, Message)> {
        let inner = self.inner.clone();
        async_stream::stream! {
            let mut rx = inner.incoming_rx.lock().await;
            while let Some(msg) = rx.recv().await {
                yield msg;
            }
        }
    }

    /// Create an axum router with SSE endpoints.
    pub fn router(self) -> axum::Router {
        use axum::routing::{get, post};

        axum::Router::new()
            .route("/sse", get(sse_handler))
            .route("/message", post(message_handler))
            .with_state(self)
    }
}

impl Default for SseServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// SSE endpoint handler - establishes SSE connection with client.
pub async fn sse_handler(
    State(state): State<SseServerState>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let session_id = Uuid::new_v4().to_string();
    debug!(session_id = %session_id, "New SSE connection");

    // Register session
    {
        let mut sessions = state.inner.sessions.lock().await;
        sessions.insert(
            session_id.clone(),
            format!("/message?session={}", session_id),
        );
    }

    // Subscribe to outgoing messages
    let mut outgoing_rx = state.inner.outgoing_tx.subscribe();

    let stream = async_stream::stream! {
        // First, send the endpoint URL
        let endpoint = format!("/message?session={}", session_id);
        yield Ok(Event::default().event("endpoint").data(endpoint));

        // Then stream messages
        loop {
            match outgoing_rx.recv().await {
                Ok(message) => {
                    match serde_json::to_string(&message) {
                        Ok(json) => {
                            trace!(session = %session_id, msg = %json, "Sending SSE message");
                            yield Ok(Event::default().event("message").data(json));
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to serialize message");
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(session = %session_id, lagged = n, "SSE client lagged");
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!(session = %session_id, "Outgoing channel closed");
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Message endpoint handler - receives messages from clients.
pub async fn message_handler(
    State(state): State<SseServerState>,
    axum::extract::Query(params): axum::extract::Query<MessageParams>,
    Json(message): Json<Message>,
) -> Response {
    let session_id = params.session.unwrap_or_else(|| "unknown".to_string());
    debug!(session = %session_id, method = ?message.method(), "Received message");

    // Check message size
    let json = match serde_json::to_string(&message) {
        Ok(j) => j,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid JSON: {}", e),
            )
                .into_response();
        }
    };

    if json.len() > state.inner.max_message_size {
        return (
            axum::http::StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "Message too large: {} bytes (max: {})",
                json.len(),
                state.inner.max_message_size
            ),
        )
            .into_response();
    }

    // Forward to incoming channel
    if state
        .inner
        .incoming_tx
        .send((session_id, message))
        .await
        .is_err()
    {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Server is shutting down",
        )
            .into_response();
    }

    axum::http::StatusCode::ACCEPTED.into_response()
}

/// Query parameters for message endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct MessageParams {
    /// Session ID from SSE connection.
    pub session: Option<String>,
}

/// Builder for SSE server.
pub struct SseServerBuilder {
    max_message_size: usize,
}

impl SseServerBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024,
        }
    }

    /// Set maximum message size.
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Build the SSE server state.
    pub fn build(self) -> SseServerState {
        SseServerState::with_max_message_size(self.max_message_size)
    }
}

impl Default for SseServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsonrpc::Message;

    #[tokio::test]
    async fn test_sse_server_state() {
        let state = SseServerState::new();

        // Test broadcasting
        let msg = Message::notification("test/notify", None);
        // Note: broadcast will fail if no subscribers, which is expected
        let _ = state.broadcast(msg);
    }
}
