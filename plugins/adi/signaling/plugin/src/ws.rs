use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use lib_signaling_protocol::{
    AuthOption, AuthRequirement, ConnectionInfo, DeviceInfo, IceServer, SignalingMessage,
};
use serde::Deserialize;
use signaling_core::{
    security::{derive_device_id, validate_secret},
    state::{AppState, DeviceMeta, UserDevice},
    tokens::extract_user_id,
    utils::generate_pairing_code,
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientKind {
    App,
    Cocoon,
}

#[derive(Deserialize)]
pub struct WsQuery {
    #[serde(default = "default_kind")]
    kind: ClientKind,
}

fn default_kind() -> ClientKind {
    ClientKind::App
}

fn build_connection_info(state: &AppState) -> ConnectionInfo {
    let ice_servers = if state.ice_servers.is_empty() {
        None
    } else {
        let servers: Vec<IceServer> = state
            .ice_servers
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();
        if servers.is_empty() { None } else { Some(servers) }
    };
    ConnectionInfo {
        manual_allowed: state.allow_manual_registration,
        ice_servers,
    }
}

fn device_info_from(ud: &UserDevice) -> DeviceInfo {
    DeviceInfo {
        device_id: ud.device_id.clone(),
        tags: ud.tags.clone(),
        online: ud.online,
        device_type: ud.device_type.clone(),
        device_config: ud.device_config.clone(),
    }
}

fn build_user_device_infos(state: &AppState, user_id: &str) -> Vec<DeviceInfo> {
    state.get_user_devices(user_id).iter().map(device_info_from).collect()
}

fn notify_device_list(state: &AppState, user_id: &str) {
    let devices = build_user_device_infos(state, user_id);
    if let Ok(json) = serde_json::to_string(&SignalingMessage::DeviceDeviceListUpdated { devices }) {
        state.notify_user(user_id, &json);
    }
}

pub async fn ws_handler(
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    info!(kind = ?query.kind, "New WebSocket connection");
    ws.on_upgrade(move |socket| handle_socket(socket, state, query.kind))
}

async fn handle_socket(socket: WebSocket, state: AppState, kind: ClientKind) {
    let (mut sender, mut receiver): (SplitSink<WebSocket, Message>, SplitStream<WebSocket>) =
        socket.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let mut device_id: Option<String> = None;
    let mut user_id: Option<String> = None;
    let mut app_conn_id: Option<u64> = None;
    let auth_required = kind == ClientKind::App && state.auth_domain.is_some();

    match kind {
        ClientKind::App => {
            let hello = SignalingMessage::AuthHello {
                auth_kind: state
                    .auth_domain
                    .as_ref()
                    .map(|_| "adi.auth".to_string())
                    .unwrap_or_else(|| "none".to_string()),
                auth_domain: state
                    .auth_domain
                    .clone()
                    .unwrap_or_else(|| "none".to_string()),
                auth_requirement: if state.auth_domain.is_some() {
                    AuthRequirement::Required
                } else {
                    AuthRequirement::Optional
                },
                auth_options: if state.auth_domain.is_some() {
                    vec![AuthOption::Verified]
                } else {
                    vec![AuthOption::Anonymous]
                },
            };
            if let Ok(json) = serde_json::to_string(&hello) {
                debug!("Sending AuthHello to app client");
                let _ = tx.send(json);
            }
        }
        ClientKind::Cocoon => {
            debug!("Cocoon client connected, waiting for device_register");
        }
    }

    while let Some(Ok(msg)) = receiver.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        let parsed: SignalingMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "Failed to parse incoming message");
                send_msg(&tx, &SignalingMessage::SystemError {
                    message: format!("Invalid message: {}", e),
                });
                continue;
            }
        };

        // Enforce auth for app clients: only AuthAuthenticate is allowed before authenticating
        if auth_required && user_id.is_none() && !matches!(parsed, SignalingMessage::AuthAuthenticate { .. }) {
            warn!("Unauthenticated app client attempted to send message");
            send_msg(&tx, &SignalingMessage::SystemError {
                message: "Authentication required. Send auth_authenticate first.".to_string(),
            });
            continue;
        }

        match parsed {
            SignalingMessage::AuthAuthenticate { access_token } if kind == ClientKind::App => {
                match extract_user_id(&access_token) {
                    Ok(uid) => {
                        info!(user_id = %uid, "User authenticated");
                        user_id = Some(uid.clone());

                        let conn_id = state.next_connection_id();
                        app_conn_id = Some(conn_id);
                        state
                            .user_connections
                            .entry(uid.clone())
                            .or_default()
                            .insert(conn_id, tx.clone());

                        send_msg(&tx, &SignalingMessage::AuthAuthenticateResponse {
                            user_id: uid.clone(),
                        });

                        let devices = build_user_device_infos(&state, &uid);
                        send_msg(&tx, &SignalingMessage::AuthHelloAuthed {
                            user_id: uid,
                            connection_info: build_connection_info(&state),
                            devices,
                        });
                    }
                    Err(e) => {
                        warn!(error = %e, "Authentication failed");
                        send_msg(&tx, &SignalingMessage::SystemError {
                            message: format!("Authentication failed: {}", e),
                        });
                    }
                }
            }

            SignalingMessage::DeviceRegister {
                secret,
                device_id: provided_id,
                version,
                tags,
                device_type,
                device_config,
            } if kind == ClientKind::Cocoon => {
                if let Err(e) = validate_secret(&secret) {
                    warn!(error = %e, "Secret validation failed");
                    send_msg(&tx, &SignalingMessage::SystemError { message: e });
                    continue;
                }

                let derived_id = derive_device_id(&secret, &state.hmac_salt);

                if let Some(ref provided) = provided_id {
                    if *provided != derived_id {
                        warn!(provided = %provided, derived = %derived_id, "Device ID mismatch");
                        send_msg(&tx, &SignalingMessage::SystemError {
                            message: "Device ID mismatch — secret does not match previously registered device".to_string(),
                        });
                        continue;
                    }
                }

                // Validate setup_token and extract owner
                let mut owner_id: Option<String> = None;
                if let Some(ref t) = tags {
                    if let Some(token) = t.get("setup_token") {
                        match extract_user_id(token) {
                            Ok(uid) => {
                                info!(device_id = %derived_id, owner = %uid, "Setup token validated, owner assigned");
                                owner_id = Some(uid);
                            }
                            Err(e) => {
                                warn!(error = %e, "Invalid setup_token in registration");
                                send_msg(&tx, &SignalingMessage::SystemError {
                                    message: format!("Invalid setup_token: {}", e),
                                });
                                continue;
                            }
                        }
                    }
                }

                if let Some(ref old_id) = device_id {
                    debug!(old_device_id = %old_id, new_device_id = %derived_id, "Re-registering, removing old connection");
                    state.connections.remove(old_id);
                    state.device_meta.remove(old_id);
                }

                device_id = Some(derived_id.clone());
                state.connections.insert(derived_id.clone(), tx.clone());

                if let Some(ref uid) = owner_id {
                    state.device_owners.insert(derived_id.clone(), uid.clone());
                }

                // Strip setup_token from tags — never persist or echo it back
                let clean_tags = tags.map(|mut t| { t.remove("setup_token"); t });

                let meta = DeviceMeta {
                    tags: clean_tags.clone().unwrap_or_default(),
                    device_type: device_type.clone(),
                    device_config: device_config.clone(),
                };
                state.device_meta.insert(derived_id.clone(), meta);

                info!(device_id = %derived_id, version = %version, owner = ?owner_id, device_type = ?device_type, "Device registered");

                send_msg(&tx, &SignalingMessage::DeviceRegisterResponse {
                    device_id: derived_id.clone(),
                    tags: clean_tags,
                });

                // Notify owner's app connections about updated device list
                if let Some(ref uid) = owner_id {
                    notify_device_list(&state, uid);
                }

                if let Some(peer_id) = state.paired_devices.get(&derived_id) {
                    let peer_id = peer_id.value().clone();
                    if state.connections.contains_key(&peer_id) {
                        info!(device_id = %derived_id, peer_id = %peer_id, "Paired peer is online, notifying both");
                        send_msg(&tx, &SignalingMessage::DevicePeerConnected {
                            peer_id: peer_id.clone(),
                        });
                        if let Some(peer_tx) = state.connections.get(&peer_id) {
                            send_msg(peer_tx.value(), &SignalingMessage::DevicePeerConnected {
                                peer_id: derived_id.clone(),
                            });
                        }
                    }
                }
            }

            SignalingMessage::DeviceDeregister { device_id: did, reason } if kind == ClientKind::Cocoon => {
                info!(device_id = %did, reason = ?reason, "Device deregistered");

                // Capture owner before removing
                let owner = state.device_owners.get(&did).map(|o| o.value().clone());

                state.connections.remove(&did);
                state.device_meta.remove(&did);
                state.device_owners.remove(&did);

                // Notify owner's app connections
                if let Some(ref uid) = owner {
                    notify_device_list(&state, uid);
                }

                if let Some((_, peer_id)) = state.paired_devices.remove(&did) {
                    state.paired_devices.remove(&peer_id);
                    if let Some(peer_tx) = state.connections.get(&peer_id) {
                        send_msg(peer_tx.value(), &SignalingMessage::DevicePeerDisconnected {
                            peer_id: did.clone(),
                        });
                    }
                }

                send_msg(&tx, &SignalingMessage::DeviceDeregisterResponse { device_id: did });
            }

            SignalingMessage::PairingCreateCode => {
                let Some(ref did) = device_id else {
                    send_msg(&tx, &SignalingMessage::SystemError {
                        message: "Must register before creating pairing code".to_string(),
                    });
                    continue;
                };

                let code = generate_pairing_code();
                info!(device_id = %did, code = %code, "Pairing code created");
                state.pairing_codes.insert(code.clone(), did.clone());
                send_msg(&tx, &SignalingMessage::PairingCreateCodeResponse { code });
            }

            SignalingMessage::PairingUseCode { code } => {
                let Some(ref did) = device_id else {
                    send_msg(&tx, &SignalingMessage::SystemError {
                        message: "Must register before using pairing code".to_string(),
                    });
                    continue;
                };

                match state.pairing_codes.remove(&code) {
                    Some((_, peer_id)) => {
                        if peer_id == *did {
                            warn!(device_id = %did, code = %code, "Self-pairing attempt rejected");
                            send_msg(&tx, &SignalingMessage::PairingFailed {
                                reason: "Cannot pair with yourself".to_string(),
                            });
                            continue;
                        }

                        info!(device_id = %did, peer_id = %peer_id, code = %code, "Devices paired");
                        state.paired_devices.insert(did.clone(), peer_id.clone());
                        state.paired_devices.insert(peer_id.clone(), did.clone());

                        send_msg(&tx, &SignalingMessage::PairingUseCodeResponse {
                            peer_id: peer_id.clone(),
                        });

                        if let Some(peer_tx) = state.connections.get(&peer_id) {
                            send_msg(peer_tx.value(), &SignalingMessage::PairingUseCodeResponse {
                                peer_id: did.clone(),
                            });
                        }
                    }
                    None => {
                        warn!(device_id = %did, code = %code, "Invalid pairing code used");
                        send_msg(&tx, &SignalingMessage::PairingFailed {
                            reason: "Invalid or expired pairing code".to_string(),
                        });
                    }
                }
            }

            SignalingMessage::SyncData { payload } => {
                // App clients (browsers) may send a routing envelope:
                //   { "to": "<target_device_id>", "data": <actual_payload> }
                // The server unwraps it and forwards `data` directly to the target device.
                // Cocoon clients use the existing pairing-based routing.
                if kind == ClientKind::App {
                    let to = payload
                        .as_object()
                        .and_then(|o| o.get("to"))
                        .and_then(|v| v.as_str())
                        .map(str::to_owned);
                    let inner = payload
                        .as_object()
                        .and_then(|o| o.get("data"))
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    let Some(target) = to else {
                        send_msg(&tx, &SignalingMessage::SystemError {
                            message: "App sync_data must include a 'to' device_id".to_string(),
                        });
                        continue;
                    };

                    if let Some(peer_tx) = state.connections.get(&target) {
                        info!(to = %target, "App client relaying SyncData to device");
                        send_msg(peer_tx.value(), &SignalingMessage::SyncData { payload: inner });
                    } else {
                        info!(to = %target, "App SyncData dropped — target device offline");
                    }
                } else {
                    let Some(ref did) = device_id else {
                        send_msg(&tx, &SignalingMessage::SystemError {
                            message: "Must register before syncing data".to_string(),
                        });
                        continue;
                    };

                    if let Some(peer_id) = state.paired_devices.get(did) {
                        let peer = peer_id.value().clone();
                        if let Some(peer_tx) = state.connections.get(&peer) {
                            debug!(from = %did, to = %peer, "Relaying SyncData");
                            send_msg(peer_tx.value(), &SignalingMessage::SyncData { payload });
                        } else {
                            debug!(from = %did, to = %peer, "SyncData dropped — peer offline");
                        }
                    } else {
                        // No paired device — route to the device owner's App connections
                        if let Some(owner_id) = state.device_owners.get(did).map(|o| o.value().clone()) {
                            if let Ok(json) = serde_json::to_string(&SignalingMessage::SyncData { payload }) {
                                debug!(from = %did, owner = %owner_id, "Relaying SyncData to owner app connections");
                                state.notify_user(&owner_id, &json);
                            }
                        } else {
                            debug!(device_id = %did, "SyncData dropped — no paired device and no owner");
                        }
                    }
                }
            }

            SignalingMessage::DeviceUpdateTags { tags } if kind == ClientKind::Cocoon => {
                let Some(ref did) = device_id else {
                    send_msg(&tx, &SignalingMessage::SystemError {
                        message: "Must register before updating tags".to_string(),
                    });
                    continue;
                };

                info!(device_id = %did, tags = ?tags, "Tags updated");
                if let Some(mut meta) = state.device_meta.get_mut(did) {
                    meta.tags = tags.clone();
                }

                send_msg(&tx, &SignalingMessage::DeviceUpdateTagsResponse {
                    device_id: did.clone(),
                    tags,
                });

                // Notify owner
                if let Some(owner) = state.device_owners.get(did) {
                    notify_device_list(&state, owner.value());
                }
            }

            SignalingMessage::DeviceUpdateDevice { tags, device_config } if kind == ClientKind::Cocoon => {
                let Some(ref did) = device_id else {
                    send_msg(&tx, &SignalingMessage::SystemError {
                        message: "Must register before updating device".to_string(),
                    });
                    continue;
                };

                info!(device_id = %did, "Device updated");
                let current_meta = state.device_meta.get(did).map(|m| m.clone());
                let mut meta = current_meta.unwrap_or(DeviceMeta {
                    tags: Default::default(),
                    device_type: None,
                    device_config: None,
                });

                if let Some(new_tags) = tags {
                    meta.tags = new_tags;
                }
                if device_config.is_some() {
                    meta.device_config = device_config;
                }

                let response_tags = meta.tags.clone();
                let response_config = meta.device_config.clone();
                state.device_meta.insert(did.clone(), meta);

                send_msg(&tx, &SignalingMessage::DeviceUpdateDeviceResponse {
                    device_id: did.clone(),
                    tags: response_tags,
                    device_config: response_config,
                });

                // Notify owner
                if let Some(owner) = state.device_owners.get(did) {
                    notify_device_list(&state, owner.value());
                }
            }

            SignalingMessage::DeviceQueryDevices { tag_filter } => {
                debug!(filter = ?tag_filter, "Querying devices by tags");
                let devices: Vec<DeviceInfo> = state
                    .device_meta
                    .iter()
                    .filter(|entry| {
                        tag_filter.iter().all(|(k, v)| {
                            entry.value().tags.get(k).map(|val| val == v).unwrap_or(false)
                        })
                    })
                    .map(|entry| {
                        let m = entry.value();
                        DeviceInfo {
                            device_id: entry.key().clone(),
                            tags: m.tags.clone(),
                            online: state.connections.contains_key(entry.key()),
                            device_type: m.device_type.clone(),
                            device_config: m.device_config.clone(),
                        }
                    })
                    .collect();

                debug!(count = devices.len(), "Device query returned results");
                send_msg(&tx, &SignalingMessage::DeviceQueryDevicesResponse { devices });
            }

            other => {
                warn!(kind = ?kind, msg_type = ?std::mem::discriminant(&other), "Unsupported message type for client kind");
                send_msg(&tx, &SignalingMessage::SystemError {
                    message: "Unsupported message type for this client kind".to_string(),
                });
            }
        }
    }

    if let Some(ref did) = device_id {
        info!(device_id = %did, "Device disconnected");
        state.connections.remove(did);
        state.device_meta.remove(did);

        // Notify owner's app connections about device going offline
        if let Some(owner) = state.device_owners.get(did).map(|o| o.value().clone()) {
            notify_device_list(&state, &owner);
        }

        if let Some((_, peer_id)) = state.paired_devices.remove(did) {
            info!(device_id = %did, peer_id = %peer_id, "Notifying peer of disconnect");
            state.paired_devices.remove(&peer_id);
            if let Some(peer_tx) = state.connections.get(&peer_id) {
                send_msg(peer_tx.value(), &SignalingMessage::DevicePeerDisconnected {
                    peer_id: did.clone(),
                });
            }
        }
    } else {
        debug!("Anonymous connection closed (never registered)");
    }

    // Clean up app connection from user_connections
    if let (Some(ref uid), Some(conn_id)) = (&user_id, app_conn_id) {
        if let Some(mut conns) = state.user_connections.get_mut(uid) {
            conns.remove(&conn_id);
            if conns.is_empty() {
                drop(conns);
                state.user_connections.remove(uid);
            }
        }
    }

    send_task.abort();
}

fn send_msg(tx: &mpsc::UnboundedSender<String>, msg: &SignalingMessage) {
    if let Ok(json) = serde_json::to_string(msg) {
        let _ = tx.send(json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use futures::{SinkExt, StreamExt};
    use lib_signaling_protocol::SignalingMessage;
    use std::collections::HashMap;
    use tokio_tungstenite::{connect_async, tungstenite::Message as TsMessage};

    async fn spawn_server() -> String {
        spawn_server_with_auth(None).await
    }

    async fn spawn_server_with_auth(auth_domain: Option<String>) -> String {
        let state = AppState::new(
            "test-salt-for-deterministic-ids".to_string(),
            auth_domain,
            true,
            vec![],
        );
        let app = Router::new()
            .route("/ws", get(ws_handler))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        format!("ws://127.0.0.1:{}/ws", addr.port())
    }

    async fn recv_msg(
        stream: &mut (impl StreamExt<Item = Result<TsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin),
    ) -> SignalingMessage {
        loop {
            match stream.next().await.unwrap().unwrap() {
                TsMessage::Text(t) => return serde_json::from_str(&t).unwrap(),
                _ => continue,
            }
        }
    }

    async fn send(
        sink: &mut (impl SinkExt<TsMessage> + Unpin),
        msg: &SignalingMessage,
    ) {
        let json = serde_json::to_string(msg).unwrap();
        sink.send(TsMessage::Text(json.into())).await.ok();
    }

    #[tokio::test]
    async fn test_app_gets_auth_hello() {
        let url = spawn_server().await;
        // Default kind=app
        let (ws, _) = connect_async(&url).await.unwrap();
        let (mut _sink, mut stream) = ws.split();

        let hello = recv_msg(&mut stream).await;
        match hello {
            SignalingMessage::AuthHello {
                auth_kind,
                auth_requirement,
                auth_options,
                ..
            } => {
                assert_eq!(auth_kind, "none");
                assert!(matches!(auth_requirement, AuthRequirement::Optional));
                assert_eq!(auth_options.len(), 1);
                assert!(matches!(auth_options[0], AuthOption::Anonymous));
            }
            other => panic!("Expected AuthHello, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cocoon_registration_flow() {
        let url = spawn_server().await;
        let cocoon_url = format!("{}?kind=cocoon", url);
        let (ws, _) = connect_async(&cocoon_url).await.unwrap();
        let (mut sink, mut stream) = ws.split();

        // 1. No AuthHello — cocoon sends DeviceRegister directly
        let secret = "xK9mP2qR7wL4nJ6vB8cT3fY5hA0gD1eS";
        let tags = HashMap::from([
            ("kind".to_string(), "desktop".to_string()),
            ("os".to_string(), "macos".to_string()),
        ]);

        send(&mut sink, &SignalingMessage::DeviceRegister {
            secret: secret.to_string(),
            device_id: None,
            version: "1.0.0".to_string(),
            tags: Some(tags.clone()),
            device_type: Some("cocoon".to_string()),
            device_config: Some(serde_json::json!({"image": "ubuntu"})),
        }).await;

        // 2. Server responds with device_id + tags
        let registered = recv_msg(&mut stream).await;
        let device_id = match registered {
            SignalingMessage::DeviceRegisterResponse { ref device_id, ref tags } => {
                assert!(!device_id.is_empty(), "device_id should be non-empty");
                let t = tags.as_ref().unwrap();
                assert_eq!(t["kind"], "desktop");
                assert_eq!(t["os"], "macos");
                device_id.clone()
            }
            other => panic!("Expected DeviceRegisterResponse, got: {:?}", other),
        };

        // 3. Re-register with same secret
        send(&mut sink, &SignalingMessage::DeviceRegister {
            secret: secret.to_string(),
            device_id: Some(device_id.clone()),
            version: "1.0.1".to_string(),
            tags: Some(HashMap::from([("kind".into(), "laptop".into())])),
            device_type: Some("cocoon".to_string()),
            device_config: None,
        }).await;

        let re_registered = recv_msg(&mut stream).await;
        match re_registered {
            SignalingMessage::DeviceRegisterResponse { device_id: did, tags: t } => {
                assert_eq!(did, device_id, "Same secret must produce same device_id");
                assert_eq!(t.as_ref().unwrap()["kind"], "laptop");
            }
            other => panic!("Expected DeviceRegisterResponse, got: {:?}", other),
        }

        // 4. Update tags
        let new_tags = HashMap::from([
            ("kind".to_string(), "laptop".to_string()),
            ("expose_ip".to_string(), "true".to_string()),
        ]);
        send(&mut sink, &SignalingMessage::DeviceUpdateTags { tags: new_tags.clone() }).await;

        let tags_updated = recv_msg(&mut stream).await;
        match tags_updated {
            SignalingMessage::DeviceUpdateTagsResponse { device_id: did, tags: t } => {
                assert_eq!(did, device_id);
                assert_eq!(t["expose_ip"], "true");
                assert_eq!(t["kind"], "laptop");
            }
            other => panic!("Expected DeviceUpdateTagsResponse, got: {:?}", other),
        }

        // 5. Query devices
        send(&mut sink, &SignalingMessage::DeviceQueryDevices {
            tag_filter: HashMap::from([("kind".into(), "laptop".into())]),
        }).await;

        let device_list = recv_msg(&mut stream).await;
        match device_list {
            SignalingMessage::DeviceQueryDevicesResponse { devices } => {
                assert_eq!(devices.len(), 1);
                assert_eq!(devices[0].device_id, device_id);
                assert!(devices[0].online);
                assert_eq!(devices[0].device_type, Some("cocoon".to_string()));
            }
            other => panic!("Expected DeviceQueryDevicesResponse, got: {:?}", other),
        }

        // 6. Deregister
        send(&mut sink, &SignalingMessage::DeviceDeregister {
            device_id: device_id.clone(),
            reason: Some("test cleanup".to_string()),
        }).await;

        let deregistered = recv_msg(&mut stream).await;
        match deregistered {
            SignalingMessage::DeviceDeregisterResponse { device_id: did } => {
                assert_eq!(did, device_id);
            }
            other => panic!("Expected DeviceDeregisterResponse, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cocoon_rejects_weak_secret() {
        let url = spawn_server().await;
        let cocoon_url = format!("{}?kind=cocoon", url);
        let (ws, _) = connect_async(&cocoon_url).await.unwrap();
        let (mut sink, mut stream) = ws.split();

        send(&mut sink, &SignalingMessage::DeviceRegister {
            secret: "short".to_string(),
            device_id: None,
            version: "1.0.0".to_string(),
            tags: None,
            device_type: None,
            device_config: None,
        }).await;

        let err = recv_msg(&mut stream).await;
        match err {
            SignalingMessage::SystemError { message } => {
                assert!(message.contains("Secret too short"));
            }
            other => panic!("Expected SystemError, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pairing_and_sync() {
        let url = spawn_server().await;
        let cocoon_url = format!("{}?kind=cocoon", url);

        // Device A connects as cocoon
        let (ws_a, _) = connect_async(&cocoon_url).await.unwrap();
        let (mut sink_a, mut stream_a) = ws_a.split();

        send(&mut sink_a, &SignalingMessage::DeviceRegister {
            secret: "aB3cD4eF5gH6iJ7kL8mN9oP0qR1sT2uV".to_string(),
            device_id: None,
            version: "1.0.0".to_string(),
            tags: None,
            device_type: None,
            device_config: None,
        }).await;
        let reg_a = recv_msg(&mut stream_a).await;
        let id_a = match reg_a {
            SignalingMessage::DeviceRegisterResponse { device_id, .. } => device_id,
            other => panic!("Expected DeviceRegisterResponse, got: {:?}", other),
        };

        // Device A creates pairing code
        send(&mut sink_a, &SignalingMessage::PairingCreateCode).await;
        let code = match recv_msg(&mut stream_a).await {
            SignalingMessage::PairingCreateCodeResponse { code } => code,
            other => panic!("Expected PairingCreateCodeResponse, got: {:?}", other),
        };

        // Device B connects as cocoon, registers, uses pairing code
        let (ws_b, _) = connect_async(&cocoon_url).await.unwrap();
        let (mut sink_b, mut stream_b) = ws_b.split();

        send(&mut sink_b, &SignalingMessage::DeviceRegister {
            secret: "xY9wV8uT7sR6qP5oN4mL3kJ2iH1gF0eD".to_string(),
            device_id: None,
            version: "1.0.0".to_string(),
            tags: None,
            device_type: None,
            device_config: None,
        }).await;
        let reg_b = recv_msg(&mut stream_b).await;
        let id_b = match reg_b {
            SignalingMessage::DeviceRegisterResponse { device_id, .. } => device_id,
            other => panic!("Expected DeviceRegisterResponse, got: {:?}", other),
        };

        send(&mut sink_b, &SignalingMessage::PairingUseCode { code }).await;

        // Both should receive Paired
        let paired_b = recv_msg(&mut stream_b).await;
        match paired_b {
            SignalingMessage::PairingUseCodeResponse { peer_id } => assert_eq!(peer_id, id_a),
            other => panic!("Expected PairingUseCodeResponse for B, got: {:?}", other),
        }

        let paired_a = recv_msg(&mut stream_a).await;
        match paired_a {
            SignalingMessage::PairingUseCodeResponse { peer_id } => assert_eq!(peer_id, id_b),
            other => panic!("Expected PairingUseCodeResponse for A, got: {:?}", other),
        }

        // Device A sends SyncData -> Device B receives it
        let payload = serde_json::json!({"action": "ping", "ts": 12345});
        send(&mut sink_a, &SignalingMessage::SyncData { payload: payload.clone() }).await;

        let sync = recv_msg(&mut stream_b).await;
        match sync {
            SignalingMessage::SyncData { payload: p } => {
                assert_eq!(p["action"], "ping");
                assert_eq!(p["ts"], 12345);
            }
            other => panic!("Expected SyncData, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_app_auth_required_blocks_unauthenticated() {
        let url = spawn_server_with_auth(Some("https://auth.example.com".to_string())).await;
        let (ws, _) = connect_async(&url).await.unwrap();
        let (mut sink, mut stream) = ws.split();

        // Should receive AuthHello with required auth
        let hello = recv_msg(&mut stream).await;
        match hello {
            SignalingMessage::AuthHello { auth_requirement, .. } => {
                assert!(matches!(auth_requirement, AuthRequirement::Required));
            }
            other => panic!("Expected AuthHello, got: {:?}", other),
        }

        // Try to send a non-auth message before authenticating
        send(&mut sink, &SignalingMessage::DeviceQueryDevices {
            tag_filter: HashMap::new(),
        }).await;

        let err = recv_msg(&mut stream).await;
        match err {
            SignalingMessage::SystemError { message } => {
                assert!(message.contains("Authentication required"));
            }
            other => panic!("Expected SystemError for unauthenticated request, got: {:?}", other),
        }
    }

    fn make_jwt(sub: &str) -> String {
        use signaling_core::tokens::base64url_encode;
        let header = base64url_encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
        let payload = base64url_encode(format!("{{\"sub\":\"{}\"}}", sub).as_bytes());
        let sig = base64url_encode(b"fake-signature");
        format!("{}.{}.{}", header, payload, sig)
    }

    #[tokio::test]
    async fn test_cocoon_setup_token_validates_owner() {
        let url = spawn_server().await;
        let cocoon_url = format!("{}?kind=cocoon", url);
        let (ws, _) = connect_async(&cocoon_url).await.unwrap();
        let (mut sink, mut stream) = ws.split();

        let token = make_jwt("user-123");
        let tags = HashMap::from([
            ("setup_token".to_string(), token),
            ("name".to_string(), "my-cocoon".to_string()),
        ]);

        send(&mut sink, &SignalingMessage::DeviceRegister {
            secret: "xK9mP2qR7wL4nJ6vB8cT3fY5hA0gD1eS".to_string(),
            device_id: None,
            version: "1.0.0".to_string(),
            tags: Some(tags),
            device_type: Some("cocoon".to_string()),
            device_config: None,
        }).await;

        let resp = recv_msg(&mut stream).await;
        match resp {
            SignalingMessage::DeviceRegisterResponse { ref tags, .. } => {
                let t = tags.as_ref().unwrap();
                // setup_token should be stripped from stored tags
                assert!(!t.contains_key("setup_token"), "setup_token should be stripped");
                assert_eq!(t["name"], "my-cocoon");
            }
            other => panic!("Expected DeviceRegisterResponse, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_cocoon_invalid_setup_token_rejected() {
        let url = spawn_server().await;
        let cocoon_url = format!("{}?kind=cocoon", url);
        let (ws, _) = connect_async(&cocoon_url).await.unwrap();
        let (mut sink, mut stream) = ws.split();

        let tags = HashMap::from([
            ("setup_token".to_string(), "not-a-valid-jwt".to_string()),
        ]);

        send(&mut sink, &SignalingMessage::DeviceRegister {
            secret: "xK9mP2qR7wL4nJ6vB8cT3fY5hA0gD1eS".to_string(),
            device_id: None,
            version: "1.0.0".to_string(),
            tags: Some(tags),
            device_type: None,
            device_config: None,
        }).await;

        let err = recv_msg(&mut stream).await;
        match err {
            SignalingMessage::SystemError { message } => {
                assert!(message.contains("Invalid setup_token"));
            }
            other => panic!("Expected SystemError for invalid setup_token, got: {:?}", other),
        }
    }
}
