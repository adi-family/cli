//! Signaling server client for browser debug operations

use crate::{
    BrowserDebugTab, ConsoleEntry, ConsoleFilters, Error, NetworkFilters, NetworkRequest, Result,
    SignalingMessage,
};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info};

/// Client for browser debug operations via signaling server
pub struct BrowserDebugClient {
    #[allow(dead_code)]
    signaling_url: String,
    access_token: String,
    sender: mpsc::UnboundedSender<String>,
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<SignalingMessage>>>>,
}

impl BrowserDebugClient {
    /// Connect to the signaling server
    pub async fn connect(signaling_url: &str, access_token: &str) -> Result<Self> {
        info!("Connecting to signaling server: {}", signaling_url);

        let url = if signaling_url.contains("?") {
            format!("{}&token={}", signaling_url, access_token)
        } else {
            format!("{}?token={}", signaling_url, access_token)
        };

        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| Error::Connection(format!("WebSocket connection failed: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();

        // Channel for sending messages
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        // Pending request tracking
        let pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<SignalingMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = pending_requests.clone();

        // Spawn write task
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = write.send(Message::Text(msg.into())).await {
                    error!("WebSocket write error: {}", e);
                    break;
                }
            }
        });

        // Spawn read task
        tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        debug!("Received message: {}", text);
                        if let Ok(msg) = serde_json::from_str::<SignalingMessage>(&text) {
                            Self::handle_message(msg, &pending_clone).await;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket closed");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket read error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(Self {
            signaling_url: signaling_url.to_string(),
            access_token: access_token.to_string(),
            sender: tx,
            pending_requests,
        })
    }

    async fn handle_message(
        msg: SignalingMessage,
        pending: &Arc<Mutex<HashMap<String, oneshot::Sender<SignalingMessage>>>>,
    ) {
        // Extract request_id from response messages
        let request_id = match &msg {
            SignalingMessage::BrowserDebugTabs { .. } => Some("list_tabs".to_string()),
            SignalingMessage::BrowserDebugNetworkData { request_id, .. } => {
                Some(request_id.clone())
            }
            SignalingMessage::BrowserDebugConsoleData { request_id, .. } => {
                Some(request_id.clone())
            }
            SignalingMessage::AccessDenied { .. } => {
                // Could be for any pending request - send to all
                let mut pending_guard = pending.lock().await;
                for (_, sender) in pending_guard.drain() {
                    let _ = sender.send(msg.clone());
                }
                return;
            }
            SignalingMessage::Error { .. } => {
                let mut pending_guard = pending.lock().await;
                for (_, sender) in pending_guard.drain() {
                    let _ = sender.send(msg.clone());
                }
                return;
            }
            _ => None,
        };

        if let Some(req_id) = request_id {
            let mut pending_guard = pending.lock().await;
            if let Some(sender) = pending_guard.remove(&req_id) {
                let _ = sender.send(msg);
            }
        }
    }

    fn send_message(&self, msg: &SignalingMessage) -> Result<()> {
        let json = serde_json::to_string(msg)?;
        self.sender
            .send(json)
            .map_err(|_| Error::Connection("Channel closed".to_string()))
    }

    async fn wait_for_response(&self, request_id: &str) -> Result<SignalingMessage> {
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.to_string(), tx);
        }

        // Wait with timeout
        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| Error::Timeout("Request timed out".to_string()))?
            .map_err(|_| Error::Connection("Response channel closed".to_string()))
    }

    /// List all browser debug tabs available to the current user
    pub async fn list_tabs(&self) -> Result<Vec<BrowserDebugTab>> {
        self.send_message(&SignalingMessage::BrowserDebugListTabs {
            access_token: self.access_token.clone(),
        })?;

        let response = self.wait_for_response("list_tabs").await?;

        match response {
            SignalingMessage::BrowserDebugTabs { tabs } => Ok(tabs),
            SignalingMessage::AccessDenied { reason } => Err(Error::AccessDenied(reason)),
            SignalingMessage::Error { message } => Err(Error::Connection(message)),
            _ => Err(Error::Connection("Unexpected response".to_string())),
        }
    }

    /// Get network requests from a debug tab
    pub async fn get_network(
        &self,
        token: &str,
        filters: Option<NetworkFilters>,
    ) -> Result<Vec<NetworkRequest>> {
        let request_id = uuid::Uuid::new_v4().to_string();

        self.send_message(&SignalingMessage::BrowserDebugGetNetwork {
            request_id: request_id.clone(),
            token: token.to_string(),
            filters,
        })?;

        let response = self.wait_for_response(&request_id).await?;

        match response {
            SignalingMessage::BrowserDebugNetworkData { requests, .. } => Ok(requests),
            SignalingMessage::AccessDenied { reason } => Err(Error::AccessDenied(reason)),
            SignalingMessage::Error { message } => Err(Error::Connection(message)),
            _ => Err(Error::Connection("Unexpected response".to_string())),
        }
    }

    /// Get console entries from a debug tab
    pub async fn get_console(
        &self,
        token: &str,
        filters: Option<ConsoleFilters>,
    ) -> Result<Vec<ConsoleEntry>> {
        let request_id = uuid::Uuid::new_v4().to_string();

        self.send_message(&SignalingMessage::BrowserDebugGetConsole {
            request_id: request_id.clone(),
            token: token.to_string(),
            filters,
        })?;

        let response = self.wait_for_response(&request_id).await?;

        match response {
            SignalingMessage::BrowserDebugConsoleData { entries, .. } => Ok(entries),
            SignalingMessage::AccessDenied { reason } => Err(Error::AccessDenied(reason)),
            SignalingMessage::Error { message } => Err(Error::Connection(message)),
            _ => Err(Error::Connection("Unexpected response".to_string())),
        }
    }
}
