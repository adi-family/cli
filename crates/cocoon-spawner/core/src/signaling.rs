use futures::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use lib_signaling_protocol::SignalingMessage;
use sha2::Sha256;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

use crate::config::SpawnerConfig;
use crate::docker::CocoonDocker;
use crate::spawner;
use crate::state::SpawnerState;

type HmacSha256 = Hmac<Sha256>;

/// HMAC-SHA256 sign data with the given secret. Returns hex-encoded signature.
fn hmac_sign(data: &str, secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC-SHA256 accepts any key size");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Run the signaling connection loop with automatic reconnection.
///
/// Registers as a Hive, then routes `SpawnCocoon`/`TerminateCocoon` messages
/// to the spawner handlers. Reconnects on disconnect until `shutdown_rx` fires.
pub async fn run_signaling_loop(
    config: SpawnerConfig,
    docker: CocoonDocker,
    state: SpawnerState,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    loop {
        if *shutdown_rx.borrow() {
            tracing::info!("shutdown requested, exiting signaling loop");
            return;
        }

        match connect_and_run(&config, &docker, &state, &mut shutdown_rx).await {
            Ok(()) => {
                tracing::info!("signaling connection closed cleanly");
            }
            Err(e) => {
                tracing::warn!("signaling connection error: {e}");
            }
        }

        if *shutdown_rx.borrow() {
            return;
        }

        tracing::info!(
            "reconnecting in {}s",
            config.reconnect_delay.as_secs()
        );

        tokio::select! {
            _ = tokio::time::sleep(config.reconnect_delay) => {}
            _ = shutdown_rx.changed() => return,
        }
    }
}

async fn connect_and_run(
    config: &SpawnerConfig,
    docker: &CocoonDocker,
    state: &SpawnerState,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> anyhow::Result<()> {
    tracing::info!("connecting to signaling server: {}", config.signaling_url);

    let (ws, _) = tokio_tungstenite::connect_async(&config.signaling_url).await?;
    let (mut sink, mut stream) = ws.split();

    tracing::info!("connected, registering as hive: {}", config.hive_id);

    // Send RegisterHive
    let signature = hmac_sign(&config.hive_id, &config.hive_secret);
    let register_msg = SignalingMessage::HiveRegister {
        hive_id: config.hive_id.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        cocoon_kinds: config.cocoon_kinds(),
        hive_id_signature: signature,
    };
    let json = serde_json::to_string(&register_msg)?;
    sink.send(Message::Text(json.into())).await?;

    // Wait for HiveRegistered
    let registered = wait_for_registration(&mut stream).await?;
    tracing::info!("registered as hive: {registered}");

    // Message loop
    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_message(&text, config, docker, state, &mut sink).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sink.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("server closed connection");
                        return Ok(());
                    }
                    Some(Err(e)) => {
                        return Err(e.into());
                    }
                    None => {
                        tracing::info!("stream ended");
                        return Ok(());
                    }
                    _ => {}
                }
            }
            _ = shutdown_rx.changed() => {
                tracing::info!("shutdown during message loop");
                let _ = sink.close().await;
                return Ok(());
            }
        }
    }
}

async fn wait_for_registration(
    stream: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
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
    config: &SpawnerConfig,
    docker: &CocoonDocker,
    state: &SpawnerState,
    sink: &mut S,
) where
    S: SinkExt<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    let msg = match serde_json::from_str::<SignalingMessage>(text) {
        Ok(m) => m,
        Err(e) => {
            tracing::debug!("ignoring unrecognized message: {e}");
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
            tracing::info!("spawn request: kind={kind} request_id={request_id}");
            Some(spawner::handle_spawn(request_id, setup_token, name, &kind, config, docker, state).await)
        }
        SignalingMessage::HiveTerminateCocoon {
            request_id,
            container_id,
        } => {
            tracing::info!("terminate request: container_id={container_id} request_id={request_id}");
            Some(spawner::handle_terminate(request_id, &container_id, docker, state).await)
        }
        _ => {
            tracing::debug!("ignoring message type");
            None
        }
    };

    if let Some(resp) = response {
        if let Ok(json) = serde_json::to_string(&resp) {
            if let Err(e) = sink.send(Message::Text(json.into())).await {
                tracing::error!("failed to send response: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sign_produces_hex() {
        let sig = hmac_sign("test-hive", "secret123");
        assert!(!sig.is_empty());
        // Should be 64 hex chars (32 bytes)
        assert_eq!(sig.len(), 64);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hmac_sign_deterministic() {
        let a = hmac_sign("data", "key");
        let b = hmac_sign("data", "key");
        assert_eq!(a, b);
    }

    #[test]
    fn hmac_sign_different_keys() {
        let a = hmac_sign("data", "key1");
        let b = hmac_sign("data", "key2");
        assert_ne!(a, b);
    }
}
