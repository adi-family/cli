use lib_signaling_protocol::SignalingMessage;
use lib_env_parse::{env_vars, env_opt};

env_vars! {
    WebrtcIceServers => "WEBRTC_ICE_SERVERS",
    WebrtcTurnUsername => "WEBRTC_TURN_USERNAME",
    WebrtcTurnCredential => "WEBRTC_TURN_CREDENTIAL",
}
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

fn build_ice_servers() -> Vec<RTCIceServer> {
    let ice_servers_env = env_opt(EnvVar::WebrtcIceServers.as_str());
    let turn_username = env_opt(EnvVar::WebrtcTurnUsername.as_str());
    let turn_credential = env_opt(EnvVar::WebrtcTurnCredential.as_str());

    let urls: Vec<String> = ice_servers_env
        .as_ref()
        .map(|s| s.split(',').map(|u| u.trim().to_string()).filter(|u| !u.is_empty()).collect())
        .unwrap_or_default();

    if urls.is_empty() {
        return vec![];
    }

    let stun_urls: Vec<String> = urls.iter().filter(|u| u.starts_with("stun:")).cloned().collect();
    let turn_urls: Vec<String> = urls.iter().filter(|u| u.starts_with("turn:") || u.starts_with("turns:")).cloned().collect();

    let mut ice_servers = Vec::new();

    if !stun_urls.is_empty() {
        tracing::info!("Configured {} STUN server(s): {:?}", stun_urls.len(), stun_urls);
        ice_servers.push(RTCIceServer {
            urls: stun_urls,
            ..Default::default()
        });
    }

    if !turn_urls.is_empty() {
        let has_credentials = turn_username.is_some() && turn_credential.is_some();
        tracing::info!(
            "Configured {} TURN server(s): {:?} (credentials: {})",
            turn_urls.len(),
            turn_urls,
            if has_credentials { "provided" } else { "none" }
        );

        ice_servers.push(RTCIceServer {
            urls: turn_urls,
            username: turn_username.unwrap_or_default(),
            credential: turn_credential.unwrap_or_default(),
            ..Default::default()
        });
    }

    ice_servers
}

pub struct WebRtcSession {
    pub session_id: String,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub data_channels: HashMap<String, Arc<RTCDataChannel>>,
    pub state: String,
}

pub struct WebRtcManager {
    sessions: Arc<Mutex<HashMap<String, WebRtcSession>>>,
    signaling_tx: mpsc::UnboundedSender<SignalingMessage>,
    close_timeout: std::time::Duration,
}

impl WebRtcManager {
    pub fn new(signaling_tx: mpsc::UnboundedSender<SignalingMessage>) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            signaling_tx,
            close_timeout: std::time::Duration::from_secs(5),
        }
    }

    #[cfg(test)]
    pub fn with_close_timeout(
        signaling_tx: mpsc::UnboundedSender<SignalingMessage>,
        close_timeout: std::time::Duration,
    ) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            signaling_tx,
            close_timeout,
        }
    }

    pub async fn create_session(&self, session_id: String) -> Result<(), String> {
        let ice_servers = build_ice_servers();
        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        let mut media_engine = MediaEngine::default();

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine)
            .map_err(|e| format!("Failed to register interceptors: {}", e))?;

        let mut setting_engine = SettingEngine::default();
        setting_engine.detach_data_channels();

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .with_setting_engine(setting_engine)
            .build();

        let peer_connection = api
            .new_peer_connection(config)
            .await
            .map_err(|e| format!("Failed to create peer connection: {}", e))?;

        let peer_connection = Arc::new(peer_connection);

        let session_id_clone = session_id.clone();
        let signaling_tx_clone = self.signaling_tx.clone();
        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let session_id = session_id_clone.clone();
            let tx = signaling_tx_clone.clone();

            Box::pin(async move {
                if let Some(c) = candidate {
                    if let Ok(json) = c.to_json() {
                        let candidate_type = if json.candidate.contains("typ host") {
                            "host"
                        } else if json.candidate.contains("typ srflx") {
                            "srflx (STUN)"
                        } else if json.candidate.contains("typ relay") {
                            "relay (TURN)"
                        } else if json.candidate.contains("typ prflx") {
                            "prflx"
                        } else {
                            "unknown"
                        };
                        tracing::debug!(
                            "ICE candidate gathered for session {}: type={}, mid={:?}",
                            session_id,
                            candidate_type,
                            json.sdp_mid
                        );

                        let _ = tx.send(SignalingMessage::WebRtcIceCandidate {
                            session_id,
                            candidate: json.candidate,
                            sdp_mid: json.sdp_mid,
                            sdp_mline_index: json.sdp_mline_index.map(|i| i as u32),
                        });
                    }
                } else {
                    tracing::debug!("ICE gathering complete for session {}", session_id);
                }
            })
        }));

        let session_id_clone = session_id.clone();
        peer_connection.on_ice_gathering_state_change(Box::new(move |state| {
            let session_id = session_id_clone.clone();
            Box::pin(async move {
                tracing::debug!(
                    "ICE gathering state for session {}: {:?}",
                    session_id,
                    state
                );
            })
        }));

        let session_id_clone = session_id.clone();
        peer_connection.on_ice_connection_state_change(Box::new(move |state| {
            let session_id = session_id_clone.clone();
            Box::pin(async move {
                tracing::info!(
                    "ICE connection state for session {}: {:?}",
                    session_id,
                    state
                );
            })
        }));

        let session_id_clone = session_id.clone();
        let signaling_tx_clone = self.signaling_tx.clone();
        let sessions_clone = self.sessions.clone();
        peer_connection.on_peer_connection_state_change(Box::new(move |state| {
            let session_id = session_id_clone.clone();
            let tx = signaling_tx_clone.clone();
            let sessions = sessions_clone.clone();

            Box::pin(async move {
                tracing::info!("WebRTC session {} state changed: {:?}", session_id, state);

                match state {
                    RTCPeerConnectionState::Connected => {
                        tracing::info!("WebRTC session {} connected", session_id);
                        if let Some(session) = sessions.lock().await.get_mut(&session_id) {
                            session.state = "connected".to_string();
                        }
                    }
                    RTCPeerConnectionState::Disconnected
                    | RTCPeerConnectionState::Failed
                    | RTCPeerConnectionState::Closed => {
                        let reason = match state {
                            RTCPeerConnectionState::Disconnected => "disconnected",
                            RTCPeerConnectionState::Failed => {
                                tracing::warn!(
                                    "WebRTC session {} failed - check WEBRTC_ICE_SERVERS config",
                                    session_id
                                );
                                "failed"
                            }
                            RTCPeerConnectionState::Closed => "closed",
                            _ => "unknown",
                        };

                        let _ = tx.send(SignalingMessage::WebRtcSessionEnded {
                            session_id: session_id.clone(),
                            reason: Some(reason.to_string()),
                        });

                        sessions.lock().await.remove(&session_id);
                    }
                    _ => {}
                }
            })
        }));

        let session_id_clone = session_id.clone();
        let signaling_tx_clone = self.signaling_tx.clone();
        let sessions_clone = self.sessions.clone();
        peer_connection.on_data_channel(Box::new(move |dc| {
            let session_id = session_id_clone.clone();
            let tx = signaling_tx_clone.clone();
            let sessions = sessions_clone.clone();
            let dc_label = dc.label().to_string();

            Box::pin(async move {
                tracing::info!(
                    "WebRTC session {} data channel opened: {}",
                    session_id,
                    dc_label
                );

                if let Some(session) = sessions.lock().await.get_mut(&session_id) {
                    session.data_channels.insert(dc_label.clone(), dc.clone());
                }

                let dc_label_clone = dc_label.clone();
                let session_id_clone = session_id.clone();
                let tx_clone = tx.clone();
                dc.on_message(Box::new(move |msg: DataChannelMessage| {
                    let session_id = session_id_clone.clone();
                    let channel = dc_label_clone.clone();
                    let tx = tx_clone.clone();

                    Box::pin(async move {
                        let (data, binary) = if msg.is_string {
                            (String::from_utf8_lossy(&msg.data).to_string(), false)
                        } else {
                            (base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &msg.data), true)
                        };

                        let _ = tx.send(SignalingMessage::WebRtcData {
                            session_id,
                            channel,
                            data,
                            binary,
                        });
                    })
                }));
            })
        }));

        let session = WebRtcSession {
            session_id: session_id.clone(),
            peer_connection,
            data_channels: HashMap::new(),
            state: "pending".to_string(),
        };

        self.sessions.lock().await.insert(session_id, session);

        Ok(())
    }

    pub async fn handle_offer(&self, session_id: &str, sdp: &str) -> Result<String, String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        let offer = RTCSessionDescription::offer(sdp.to_string())
            .map_err(|e| format!("Failed to parse SDP offer: {}", e))?;

        session
            .peer_connection
            .set_remote_description(offer)
            .await
            .map_err(|e| format!("Failed to set remote description: {}", e))?;

        let answer = session
            .peer_connection
            .create_answer(None)
            .await
            .map_err(|e| format!("Failed to create answer: {}", e))?;

        session
            .peer_connection
            .set_local_description(answer.clone())
            .await
            .map_err(|e| format!("Failed to set local description: {}", e))?;

        Ok(answer.sdp)
    }

    pub async fn add_ice_candidate(
        &self,
        session_id: &str,
        candidate: &str,
        sdp_mid: Option<&str>,
        sdp_mline_index: Option<u32>,
    ) -> Result<(), String> {
        let candidate_type = if candidate.contains("typ host") {
            "host"
        } else if candidate.contains("typ srflx") {
            "srflx (STUN)"
        } else if candidate.contains("typ relay") {
            "relay (TURN)"
        } else if candidate.contains("typ prflx") {
            "prflx"
        } else {
            "unknown"
        };
        tracing::debug!(
            "Remote ICE candidate for session {}: type={}, mid={:?}",
            session_id,
            candidate_type,
            sdp_mid
        );

        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        let ice_candidate = webrtc::ice_transport::ice_candidate::RTCIceCandidateInit {
            candidate: candidate.to_string(),
            sdp_mid: sdp_mid.map(|s| s.to_string()),
            sdp_mline_index: sdp_mline_index.map(|i| i as u16),
            ..Default::default()
        };

        session
            .peer_connection
            .add_ice_candidate(ice_candidate)
            .await
            .map_err(|e| format!("Failed to add ICE candidate: {}", e))?;

        Ok(())
    }

    pub async fn send_data(
        &self,
        session_id: &str,
        channel: &str,
        data: &str,
        binary: bool,
    ) -> Result<(), String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        let dc = session
            .data_channels
            .get(channel)
            .ok_or_else(|| format!("Data channel {} not found", channel))?;

        let bytes = if binary {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                .map_err(|e| format!("Failed to decode base64: {}", e))?
        } else {
            data.as_bytes().to_vec()
        };

        dc.send(&bytes.into())
            .await
            .map_err(|e| format!("Failed to send data: {}", e))?;

        Ok(())
    }

    pub async fn close_session(&self, session_id: &str) -> Result<(), String> {
        if let Some(session) = self.sessions.lock().await.remove(session_id) {
            let close_result = tokio::time::timeout(
                self.close_timeout,
                session.peer_connection.close(),
            )
            .await;

            match close_result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::warn!(
                        "Failed to close peer connection for session {}: {}",
                        session_id,
                        e
                    );
                }
                Err(_) => {
                    tracing::warn!(
                        "Timeout closing peer connection for session {}",
                        session_id
                    );
                }
            }
        }
        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<String> {
        self.sessions
            .lock()
            .await
            .keys()
            .cloned()
            .collect()
    }

    pub async fn session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }

    pub async fn session_exists(&self, session_id: &str) -> bool {
        self.sessions.lock().await.contains_key(session_id)
    }

    pub async fn get_session_state(&self, session_id: &str) -> Option<String> {
        self.sessions
            .lock()
            .await
            .get(session_id)
            .map(|s| s.state.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn create_test_manager() -> (WebRtcManager, mpsc::UnboundedReceiver<SignalingMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let manager =
            WebRtcManager::with_close_timeout(tx, std::time::Duration::from_millis(100));
        (manager, rx)
    }

    #[tokio::test]
    async fn test_create_single_session() {
        let (manager, _rx) = create_test_manager();

        let result = manager.create_session("session-1".to_string()).await;
        assert!(result.is_ok(), "Failed to create session: {:?}", result);

        assert!(manager.session_exists("session-1").await);
        assert_eq!(manager.session_count().await, 1);
    }

    #[tokio::test]
    async fn test_create_multiple_sessions_sequentially() {
        let (manager, _rx) = create_test_manager();

        for i in 1..=5 {
            let session_id = format!("session-{}", i);
            let result = manager.create_session(session_id.clone()).await;
            assert!(
                result.is_ok(),
                "Failed to create session {}: {:?}",
                i,
                result
            );
            assert!(manager.session_exists(&session_id).await);
        }

        assert_eq!(manager.session_count().await, 5);

        let sessions = manager.list_sessions().await;
        for i in 1..=5 {
            assert!(
                sessions.contains(&format!("session-{}", i)),
                "Session {} not found in list",
                i
            );
        }
    }

    #[tokio::test]
    async fn test_close_session_and_cleanup() {
        let (manager, _rx) = create_test_manager();

        manager
            .create_session("session-to-close".to_string())
            .await
            .expect("Failed to create session");
        assert!(manager.session_exists("session-to-close").await);

        manager
            .close_session("session-to-close")
            .await
            .expect("Failed to close session");

        assert!(!manager.session_exists("session-to-close").await);
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_recreate_session_after_close() {
        let (manager, _rx) = create_test_manager();

        manager
            .create_session("recyclable-session".to_string())
            .await
            .expect("Failed to create initial session");
        assert!(manager.session_exists("recyclable-session").await);

        manager
            .close_session("recyclable-session")
            .await
            .expect("Failed to close session");
        assert!(!manager.session_exists("recyclable-session").await);

        let result = manager
            .create_session("recyclable-session".to_string())
            .await;
        assert!(
            result.is_ok(),
            "Failed to recreate session after close: {:?}",
            result
        );
        assert!(manager.session_exists("recyclable-session").await);
    }
}
