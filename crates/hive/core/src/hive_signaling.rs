//! Hive ↔ Signaling Server WebSocket connection.
//!
//! Registers the hive daemon as a device on the signaling server,
//! advertises supported cocoon kinds, and translates spawn/terminate
//! requests into hive daemon `CreateService`/`StartService`/`DeleteService` calls.

use crate::hive_config::ServiceConfig;
use crate::source_manager::SourceManager;
use futures::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use lib_signaling_protocol::{CocoonKind, SignalingMessage};
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

type HmacSha256 = Hmac<Sha256>;

/// Configuration for connecting the hive daemon to the signaling server.
#[derive(Debug, Clone)]
pub struct HiveSignalingConfig {
    pub signaling_url: String,
    pub hive_secret: String,
    pub device_secret: String,
    pub cocoon_kinds: Vec<CocoonKind>,
    pub cocoon_source_id: String,
    pub reconnect_delay: Duration,
}

fn hmac_sign(data: &str, secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC-SHA256 accepts any key size");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Run the signaling connection loop with automatic reconnection.
///
/// Registers as a device, then translates `HiveSpawnCocoon`/`HiveTerminateCocoon`
/// into hive service operations. Reconnects on disconnect until `shutdown_rx` fires.
pub async fn run_signaling_loop(
    config: HiveSignalingConfig,
    source_manager: Arc<SourceManager>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    loop {
        if *shutdown_rx.borrow() {
            info!("signaling shutdown requested");
            return;
        }

        match connect_and_run(&config, &source_manager, &mut shutdown_rx).await {
            Ok(()) => info!("signaling connection closed cleanly"),
            Err(e) => warn!("signaling connection error: {e}"),
        }

        if *shutdown_rx.borrow() {
            return;
        }

        info!("reconnecting to signaling in {}s", config.reconnect_delay.as_secs());
        tokio::select! {
            _ = tokio::time::sleep(config.reconnect_delay) => {}
            _ = shutdown_rx.changed() => return,
        }
    }
}

async fn connect_and_run(
    config: &HiveSignalingConfig,
    source_manager: &Arc<SourceManager>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> anyhow::Result<()> {
    info!("connecting to signaling server: {}", config.signaling_url);

    let (ws, _) = tokio_tungstenite::connect_async(&config.signaling_url).await?;
    let (mut sink, mut stream) = ws.split();

    // Register as a hive device
    let hive_id_signature = hmac_sign("hive", &config.hive_secret);
    let register_msg = SignalingMessage::HiveRegister {
        hive_id: "hive".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        cocoon_kinds: config.cocoon_kinds.clone(),
        hive_id_signature,
    };

    let json = serde_json::to_string(&register_msg)?;
    sink.send(Message::Text(json.into())).await?;

    // Wait for registration confirmation
    let hive_id = wait_for_registration(&mut stream).await?;
    info!("registered as hive: {hive_id}");

    // Message loop
    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_message(&text, config, source_manager, &mut sink).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sink.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("signaling connection closed");
                        return Ok(());
                    }
                    Some(Err(e)) => return Err(e.into()),
                    _ => {}
                }
            }
            _ = shutdown_rx.changed() => {
                info!("shutdown during signaling message loop");
                let _ = sink.close().await;
                return Ok(());
            }
        }
    }
}

async fn wait_for_registration(
    stream: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
             + Unpin),
) -> anyhow::Result<String> {
    while let Some(msg) = stream.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(SignalingMessage::HiveRegisterResponse { hive_id }) =
                serde_json::from_str::<SignalingMessage>(&text)
            {
                return Ok(hive_id);
            }
        }
    }
    Err(anyhow::anyhow!("connection closed before registration"))
}

async fn handle_message<S>(
    text: &str,
    config: &HiveSignalingConfig,
    source_manager: &Arc<SourceManager>,
    sink: &mut S,
) where
    S: SinkExt<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    let msg = match serde_json::from_str::<SignalingMessage>(text) {
        Ok(m) => m,
        Err(e) => {
            debug!("ignoring unrecognized message: {e}");
            return;
        }
    };

    let response = match msg {
        SignalingMessage::HiveSpawnCocoon {
            request_id,
            setup_token,
            name,
            kind,
        } => {
            info!("spawn request: kind={kind} request_id={request_id}");
            Some(handle_spawn(
                request_id,
                setup_token,
                name,
                &kind,
                config,
                source_manager,
            ).await)
        }
        SignalingMessage::HiveTerminateCocoon {
            request_id,
            container_id,
        } => {
            info!("terminate request: container_id={container_id} request_id={request_id}");
            Some(handle_terminate(request_id, &container_id, config, source_manager).await)
        }
        _ => {
            debug!("ignoring message type");
            None
        }
    };

    if let Some(resp) = response {
        if let Ok(json) = serde_json::to_string(&resp) {
            if let Err(e) = sink.send(Message::Text(json.into())).await {
                error!("failed to send response: {e}");
            }
        }
    }
}

/// Translate a cocoon spawn request into hive CreateService + StartService.
async fn handle_spawn(
    request_id: String,
    setup_token: String,
    name: Option<String>,
    kind: &str,
    config: &HiveSignalingConfig,
    source_manager: &Arc<SourceManager>,
) -> SignalingMessage {
    let kind_config = match config.cocoon_kinds.iter().find(|k| k.id == kind) {
        Some(k) => k,
        None => {
            return spawn_error(request_id, format!("unknown cocoon kind: {kind}"));
        }
    };

    let container_name = name.unwrap_or_else(|| {
        let short_id = &uuid::Uuid::new_v4().to_string()[..8];
        format!("cocoon-{short_id}")
    });

    // Build a ServiceConfig for the cocoon-spawner runner
    let service_config_json = serde_json::json!({
        "runner": {
            "type": "cocoon-spawner",
            "cocoon-spawner": {
                "image": kind_config.image,
                "signaling_url": config.signaling_url,
                "setup_token": setup_token,
            }
        },
        "restart": "never"
    });

    let service_config: ServiceConfig = match serde_json::from_value(service_config_json) {
        Ok(c) => c,
        Err(e) => {
            return spawn_error(request_id, format!("failed to build service config: {e}"));
        }
    };

    // Create the service
    if let Err(e) = source_manager
        .create_service(&config.cocoon_source_id, &container_name, service_config)
        .await
    {
        return spawn_error(request_id, format!("create service failed: {e}"));
    }

    // Start the service
    let fqn = format!("{}:{}", config.cocoon_source_id, container_name);
    if let Err(e) = source_manager.start_service(&fqn).await {
        // Clean up on start failure
        let _ = source_manager.delete_service(&fqn).await;
        return spawn_error(request_id, format!("start service failed: {e}"));
    }

    info!("cocoon spawned: {container_name}");

    SignalingMessage::HiveSpawnCocoonResult {
        request_id,
        success: true,
        device_id: None,
        container_id: Some(container_name),
        error: None,
    }
}

/// Translate a cocoon terminate request into hive DeleteService (which stops first).
async fn handle_terminate(
    request_id: String,
    container_id: &str,
    config: &HiveSignalingConfig,
    source_manager: &Arc<SourceManager>,
) -> SignalingMessage {
    let fqn = format!("{}:{}", config.cocoon_source_id, container_id);

    if let Err(e) = source_manager.delete_service(&fqn).await {
        return SignalingMessage::HiveTerminateCocoonResult {
            request_id,
            success: false,
            error: Some(format!("delete service failed: {e}")),
        };
    }

    info!("cocoon terminated: {container_id}");

    SignalingMessage::HiveTerminateCocoonResult {
        request_id,
        success: true,
        error: None,
    }
}

fn spawn_error(request_id: String, error: String) -> SignalingMessage {
    error!("spawn failed: {error}");
    SignalingMessage::HiveSpawnCocoonResult {
        request_id,
        success: false,
        device_id: None,
        container_id: None,
        error: Some(error),
    }
}
