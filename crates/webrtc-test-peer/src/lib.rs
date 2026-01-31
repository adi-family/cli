//! WebRTC Test Peer Library
//!
//! A minimal WebRTC peer for E2E testing of web-app WebRTC functionality.
//! Can be used as a library for programmatic testing or as a CLI tool.
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use webrtc_test_peer::{TestPeer, Config};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = Config::test_default();
//!     let peer = TestPeer::new(config);
//!     
//!     // Run until connection closes or timeout
//!     peer.run().await.expect("Peer failed");
//! }
//! ```

pub mod config;
pub mod signaling;
pub mod webrtc;
pub mod handlers;

pub use config::{Config, CliArgs, TestScenario};
pub use signaling::{SignalingClient, SignalingEvent, ConnectionState};
pub use webrtc::{WebRtcManager, WebRtcEvent, DataChannelMsg};
pub use handlers::{PtyHandler, SilkHandler, FileSystemHandler, MessageHandler};

use lib_signaling_protocol::SignalingMessage;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Test peer that combines signaling and WebRTC
pub struct TestPeer {
    config: Config,
    signaling: Arc<SignalingClient>,
    webrtc: Option<Arc<WebRtcManager>>,
}

impl TestPeer {
    /// Create a new test peer with the given configuration
    pub fn new(config: Config) -> Self {
        let signaling = Arc::new(SignalingClient::new(config.clone()));
        
        Self {
            config,
            signaling,
            webrtc: None,
        }
    }

    /// Get signaling event subscription
    pub fn subscribe_signaling(&self) -> broadcast::Receiver<SignalingEvent> {
        self.signaling.subscribe()
    }

    /// Get WebRTC event subscription (only available after connection)
    pub fn subscribe_webrtc(&self) -> Option<broadcast::Receiver<WebRtcEvent>> {
        self.webrtc.as_ref().map(|w| w.subscribe())
    }

    /// Get the device ID (after registration)
    pub async fn device_id(&self) -> Option<String> {
        self.signaling.get_device_id().await
    }

    /// Run the test peer (blocking)
    pub async fn run(mut self) -> Result<(), String> {
        self.run_internal().await
    }

    /// Run with a timeout
    pub async fn run_with_timeout(mut self, timeout: std::time::Duration) -> Result<(), String> {
        tokio::select! {
            result = self.run_internal() => result,
            _ = tokio::time::sleep(timeout) => {
                tracing::info!("Test peer timed out after {:?}", timeout);
                Ok(())
            }
        }
    }

    async fn run_internal(&mut self) -> Result<(), String> {
        // Create WebRTC manager with signaling sender
        let webrtc = Arc::new(WebRtcManager::new(
            self.signaling.get_sender(),
            self.config.scenario.clone(),
        ));
        self.webrtc = Some(webrtc.clone());

        // Create message handlers
        let pty_handler = PtyHandler::new(self.config.scenario.pty.clone());
        let silk_handler = SilkHandler::new(self.config.scenario.silk.clone());
        let fs_handler = FileSystemHandler::new(
            self.config.scenario.filesystem.clone(),
            self.config.mock_fs_root.clone(),
        );

        // Subscribe to events
        let mut signaling_events = self.signaling.subscribe();
        let mut webrtc_events = webrtc.subscribe();

        // Start signaling connection in background
        let signaling = self.signaling.clone();
        let mut signaling_handle = tokio::spawn(async move {
            signaling.connect().await
        });

        let signaling_tx = self.signaling.get_sender();
        let one_shot = self.config.one_shot;
        let mut session_completed = false;

        // Main event loop
        loop {
            tokio::select! {
                // Handle signaling events
                event = signaling_events.recv() => {
                    match event {
                        Ok(SignalingEvent::Registered { device_id }) => {
                            tracing::info!("Registered as device: {}", device_id);
                        }
                        Ok(SignalingEvent::WebRtcSessionStarted { session_id, .. }) => {
                            tracing::info!("Creating WebRTC session: {}", session_id);
                            if let Err(e) = webrtc.create_session(session_id.clone()).await {
                                tracing::error!("Failed to create session: {}", e);
                            }
                        }
                        Ok(SignalingEvent::WebRtcOffer { session_id, sdp }) => {
                            tracing::info!("Handling offer for session: {}", session_id);
                            match webrtc.handle_offer(&session_id, &sdp).await {
                                Ok(answer_sdp) => {
                                    let _ = signaling_tx.send(SignalingMessage::WebRtcAnswer {
                                        session_id,
                                        sdp: answer_sdp,
                                    });
                                }
                                Err(e) => {
                                    tracing::error!("Failed to handle offer: {}", e);
                                }
                            }
                        }
                        Ok(SignalingEvent::WebRtcIceCandidate { session_id, candidate, sdp_mid, sdp_mline_index }) => {
                            if let Err(e) = webrtc.add_ice_candidate(
                                &session_id,
                                &candidate,
                                sdp_mid.as_deref(),
                                sdp_mline_index,
                            ).await {
                                tracing::warn!("Failed to add ICE candidate: {}", e);
                            }
                        }
                        Ok(SignalingEvent::WebRtcSessionEnded { session_id, reason }) => {
                            tracing::info!("Session {} ended: {:?}", session_id, reason);
                            let _ = webrtc.close_session(&session_id).await;
                            session_completed = true;
                            if one_shot {
                                tracing::info!("One-shot mode, exiting");
                                return Ok(());
                            }
                        }
                        Ok(SignalingEvent::StateChanged(ConnectionState::Error)) => {
                            tracing::error!("Signaling connection error");
                            return Err("Signaling connection error".to_string());
                        }
                        Ok(SignalingEvent::Error { message }) => {
                            tracing::error!("Signaling error: {}", message);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("Signaling channel closed");
                            break;
                        }
                        _ => {}
                    }
                }

                // Handle WebRTC events
                event = webrtc_events.recv() => {
                    match event {
                        Ok(WebRtcEvent::DataChannelOpened { session_id, label }) => {
                            tracing::info!("Data channel opened: {} on session {}", label, session_id);
                        }
                        Ok(WebRtcEvent::DataChannelMessage(msg)) => {
                            tracing::debug!("Data channel message on {}: {}", msg.channel, &msg.data[..std::cmp::min(100, msg.data.len())]);
                            
                            // Route to appropriate handler
                            let response = match msg.channel.as_str() {
                                "pty" | "terminal" => pty_handler.handle(&msg.data),
                                "silk" => silk_handler.handle(&msg.data),
                                "file" => fs_handler.handle(&msg.data),
                                _ => {
                                    tracing::warn!("Unknown channel: {}", msg.channel);
                                    None
                                }
                            };

                            // Send response back
                            if let Some(response_data) = response {
                                // Handle multi-line responses (multiple JSON objects)
                                for line in response_data.lines() {
                                    if !line.is_empty() {
                                        if let Err(e) = webrtc.send_data(
                                            &msg.session_id,
                                            &msg.channel,
                                            line,
                                            false,
                                        ).await {
                                            tracing::warn!("Failed to send response: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Ok(WebRtcEvent::SessionClosed { session_id, reason }) => {
                            tracing::info!("WebRTC session {} closed: {}", session_id, reason);
                            session_completed = true;
                            if one_shot {
                                return Ok(());
                            }
                        }
                        Ok(WebRtcEvent::SessionStateChanged { session_id, state }) => {
                            tracing::info!("Session {} state: {}", session_id, state);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("WebRTC channel closed");
                            break;
                        }
                        Err(_) => {}
                    }
                }

                // Check if signaling task finished
                _ = &mut signaling_handle => {
                    tracing::info!("Signaling connection closed");
                    break;
                }
            }
        }

        if session_completed {
            Ok(())
        } else {
            Err("Connection closed without completing".to_string())
        }
    }
}

/// Builder for TestPeer configuration
pub struct TestPeerBuilder {
    config: Config,
}

impl TestPeerBuilder {
    /// Create a new builder with default test config
    pub fn new() -> Self {
        Self {
            config: Config::test_default(),
        }
    }

    /// Set signaling URL
    pub fn signaling_url(mut self, url: &str) -> Self {
        self.config.signaling_url = url.to_string();
        self
    }

    /// Set device ID
    pub fn device_id(mut self, id: &str) -> Self {
        self.config.device_id = id.to_string();
        self
    }

    /// Set display name
    pub fn name(mut self, name: &str) -> Self {
        self.config.name = name.to_string();
        self
    }

    /// Enable one-shot mode (exit after first session)
    pub fn one_shot(mut self, enabled: bool) -> Self {
        self.config.one_shot = enabled;
        self
    }

    /// Set connection timeout
    pub fn timeout(mut self, duration: std::time::Duration) -> Self {
        self.config.timeout = Some(duration);
        self
    }

    /// Set ICE drop rate for testing
    pub fn drop_ice_rate(mut self, rate: f64) -> Self {
        self.config.scenario.drop_ice_rate = rate;
        self
    }

    /// Set data channel drop rate for testing
    pub fn drop_data_rate(mut self, rate: f64) -> Self {
        self.config.scenario.drop_data_rate = rate;
        self
    }

    /// Set artificial latency
    pub fn latency(mut self, duration: std::time::Duration) -> Self {
        self.config.scenario.latency = duration;
        self
    }

    /// Don't send WebRTC answer (for timeout testing)
    pub fn no_answer(mut self, enabled: bool) -> Self {
        self.config.scenario.no_answer = enabled;
        self
    }

    /// Set mock filesystem root
    pub fn mock_fs_root(mut self, path: std::path::PathBuf) -> Self {
        self.config.mock_fs_root = Some(path);
        self
    }

    /// Build the TestPeer
    pub fn build(self) -> TestPeer {
        TestPeer::new(self.config)
    }
}

impl Default for TestPeerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
