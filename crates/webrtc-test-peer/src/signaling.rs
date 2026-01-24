//! Signaling client for WebSocket connection to signaling server
//!
//! Handles device registration and message routing.

use futures::{SinkExt, StreamExt};
use lib_tarminal_sync::SignalingMessage;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, broadcast};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::config::Config;

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Registering,
    Connected,
    Error,
}

/// Signaling client events
#[derive(Debug, Clone)]
pub enum SignalingEvent {
    /// Connection state changed
    StateChanged(ConnectionState),
    /// Device registered successfully
    Registered { device_id: String },
    /// WebRTC session started
    WebRtcSessionStarted { session_id: String, device_id: String },
    /// WebRTC offer received
    WebRtcOffer { session_id: String, sdp: String },
    /// WebRTC ICE candidate received
    WebRtcIceCandidate {
        session_id: String,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u32>,
    },
    /// WebRTC session ended
    WebRtcSessionEnded { session_id: String, reason: Option<String> },
    /// Error occurred
    Error { message: String },
}

/// Signaling client for connecting to signaling server
pub struct SignalingClient {
    config: Config,
    state: Arc<Mutex<ConnectionState>>,
    outgoing_tx: mpsc::UnboundedSender<SignalingMessage>,
    outgoing_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<SignalingMessage>>>>,
    event_tx: broadcast::Sender<SignalingEvent>,
    device_id: Arc<Mutex<Option<String>>>,
}

impl SignalingClient {
    /// Create a new signaling client
    pub fn new(config: Config) -> Self {
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        let (event_tx, _) = broadcast::channel(100);

        Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            outgoing_tx,
            outgoing_rx: Arc::new(Mutex::new(Some(outgoing_rx))),
            event_tx,
            device_id: Arc::new(Mutex::new(None)),
        }
    }

    /// Get event receiver for subscribing to signaling events
    pub fn subscribe(&self) -> broadcast::Receiver<SignalingEvent> {
        self.event_tx.subscribe()
    }

    /// Get sender for outgoing messages
    pub fn get_sender(&self) -> mpsc::UnboundedSender<SignalingMessage> {
        self.outgoing_tx.clone()
    }

    /// Get current connection state
    pub async fn get_state(&self) -> ConnectionState {
        *self.state.lock().await
    }

    /// Get device ID (after registration)
    pub async fn get_device_id(&self) -> Option<String> {
        self.device_id.lock().await.clone()
    }

    /// Send a signaling message
    pub fn send(&self, msg: SignalingMessage) -> Result<(), String> {
        self.outgoing_tx
            .send(msg)
            .map_err(|e| format!("Failed to queue message: {}", e))
    }

    /// Connect to signaling server and run message loop
    pub async fn connect(&self) -> Result<(), String> {
        // Take the receiver (can only connect once)
        let outgoing_rx = self.outgoing_rx.lock().await.take()
            .ok_or("Already connected")?;

        self.set_state(ConnectionState::Connecting).await;

        let url = Url::parse(&self.config.signaling_url)
            .map_err(|e| format!("Invalid signaling URL: {}", e))?;

        tracing::info!("Connecting to signaling server: {}", url);

        let (ws_stream, _) = connect_async(url.as_str())
            .await
            .map_err(|e| format!("WebSocket connection failed: {}", e))?;

        tracing::info!("WebSocket connected, registering...");
        self.set_state(ConnectionState::Registering).await;

        let (write, read) = ws_stream.split();

        // Wrap write in mutex for sharing
        let write = Arc::new(Mutex::new(write));

        // Send registration message
        self.send_registration(write.clone()).await?;

        // Spawn outgoing message handler
        let write_clone = write.clone();
        let mut outgoing_rx = outgoing_rx;
        tokio::spawn(async move {
            while let Some(msg) = outgoing_rx.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    let mut w = write_clone.lock().await;
                    if let Err(e) = w.send(Message::Text(json)).await {
                        tracing::error!("Failed to send message: {}", e);
                        break;
                    }
                }
            }
        });

        // Process incoming messages
        self.process_incoming(read).await;

        Ok(())
    }

    async fn send_registration(
        &self,
        write: Arc<Mutex<futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
            >,
            Message
        >>>
    ) -> Result<(), String> {
        let msg = if let Some(setup_token) = &self.config.setup_token {
            SignalingMessage::RegisterWithSetupToken {
                secret: self.config.secret.clone(),
                setup_token: setup_token.clone(),
                name: Some(self.config.name.clone()),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }
        } else {
            // Don't send device_id - let server derive it from secret
            // The server uses HMAC(secret, salt) to generate deterministic device_id
            SignalingMessage::Register {
                secret: self.config.secret.clone(),
                device_id: None,
                version: env!("CARGO_PKG_VERSION").to_string(),
            }
        };

        let json = serde_json::to_string(&msg)
            .map_err(|e| format!("Failed to serialize registration: {}", e))?;

        let mut w = write.lock().await;
        w.send(Message::Text(json))
            .await
            .map_err(|e| format!("Failed to send registration: {}", e))
    }

    async fn process_incoming(
        &self,
        mut read: futures::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
            >
        >,
    ) {
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self.handle_message(&text).await {
                        tracing::error!("Error handling message: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("WebSocket closed");
                    self.set_state(ConnectionState::Disconnected).await;
                    break;
                }
                Ok(Message::Ping(data)) => {
                    // Pong is handled automatically by tungstenite
                    tracing::trace!("Received ping: {:?}", data);
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    self.set_state(ConnectionState::Error).await;
                    self.emit(SignalingEvent::Error {
                        message: e.to_string(),
                    });
                    break;
                }
            }
        }
    }

    async fn handle_message(&self, text: &str) -> Result<(), String> {
        let msg: SignalingMessage = serde_json::from_str(text)
            .map_err(|e| format!("Failed to parse message: {} - {}", e, text))?;

        tracing::debug!("Received: {:?}", msg);

        match msg {
            SignalingMessage::Registered { device_id } => {
                tracing::info!("Registered with device_id: {}", device_id);
                *self.device_id.lock().await = Some(device_id.clone());
                self.set_state(ConnectionState::Connected).await;
                self.emit(SignalingEvent::Registered { device_id });
            }

            SignalingMessage::RegisteredWithOwner { device_id, owner_id, name } => {
                tracing::info!(
                    "Registered with device_id: {}, owner: {}, name: {:?}",
                    device_id, owner_id, name
                );
                *self.device_id.lock().await = Some(device_id.clone());
                self.set_state(ConnectionState::Connected).await;
                self.emit(SignalingEvent::Registered { device_id });
            }

            SignalingMessage::WebRtcStartSession { session_id, device_id, .. } => {
                tracing::info!("WebRTC session started: {} for device {}", session_id, device_id);
                self.emit(SignalingEvent::WebRtcSessionStarted { session_id, device_id });
            }

            SignalingMessage::WebRtcOffer { session_id, sdp } => {
                tracing::info!("WebRTC offer received for session: {}", session_id);
                self.emit(SignalingEvent::WebRtcOffer { session_id, sdp });
            }

            SignalingMessage::WebRtcIceCandidate { session_id, candidate, sdp_mid, sdp_mline_index } => {
                tracing::debug!("ICE candidate received for session: {}", session_id);
                self.emit(SignalingEvent::WebRtcIceCandidate {
                    session_id,
                    candidate,
                    sdp_mid,
                    sdp_mline_index,
                });
            }

            SignalingMessage::WebRtcSessionEnded { session_id, reason } => {
                tracing::info!("WebRTC session ended: {} - {:?}", session_id, reason);
                self.emit(SignalingEvent::WebRtcSessionEnded { session_id, reason });
            }

            SignalingMessage::WebRtcError { session_id, code, message } => {
                tracing::error!("WebRTC error for {}: {} - {}", session_id, code, message);
                self.emit(SignalingEvent::Error {
                    message: format!("WebRTC error [{}]: {}", code, message),
                });
            }

            SignalingMessage::Error { message } => {
                tracing::error!("Signaling error: {}", message);
                self.emit(SignalingEvent::Error { message });
            }

            SignalingMessage::AccessDenied { reason } => {
                tracing::error!("Access denied: {}", reason);
                self.emit(SignalingEvent::Error {
                    message: format!("Access denied: {}", reason),
                });
            }

            _ => {
                tracing::trace!("Unhandled message type: {:?}", msg);
            }
        }

        Ok(())
    }

    async fn set_state(&self, state: ConnectionState) {
        let mut s = self.state.lock().await;
        if *s != state {
            *s = state;
            self.emit(SignalingEvent::StateChanged(state));
        }
    }

    fn emit(&self, event: SignalingEvent) {
        // Ignore send errors (no receivers)
        let _ = self.event_tx.send(event);
    }
}
