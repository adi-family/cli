use lib_signaling_protocol::SignalingMessage;
use std::time::Duration;

use crate::config::SpawnerConfig;
use crate::docker::CocoonDocker;
use crate::state::{SpawnedCocoon, SpawnerState};

/// Handle a SpawnCocoon request. Returns the SignalingMessage response to send back.
pub async fn handle_spawn(
    request_id: String,
    setup_token: String,
    name: Option<String>,
    kind: &str,
    config: &SpawnerConfig,
    docker: &CocoonDocker,
    state: &SpawnerState,
) -> SignalingMessage {
    if !state.can_spawn().await {
        let max = state.max_concurrent().await;
        return spawn_error(request_id, format!("concurrency limit reached (max {max})"));
    }

    let kind_config = match config.find_kind(kind) {
        Some(k) => k,
        None => return spawn_error(request_id, format!("unknown cocoon kind: {kind}")),
    };

    let container_name = name.unwrap_or_else(|| {
        let short_id = &uuid::Uuid::new_v4().to_string()[..8];
        format!("cocoon-spawner-{short_id}")
    });

    let container_id = match docker
        .spawn_cocoon(
            &container_name,
            kind_config,
            &config.signaling_url,
            &setup_token,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => return spawn_error(request_id, e.to_string()),
    };

    state
        .add_cocoon(SpawnedCocoon {
            container_name,
            container_id: container_id.clone(),
            kind: kind.to_string(),
            setup_token,
            spawned_at: chrono::Utc::now(),
            request_id: request_id.clone(),
        })
        .await;

    SignalingMessage::SpawnCocoonResult {
        request_id,
        success: true,
        device_id: None,
        container_id: Some(container_id),
        error: None,
    }
}

/// Handle a TerminateCocoon request. Returns the SignalingMessage response to send back.
pub async fn handle_terminate(
    request_id: String,
    container_id: &str,
    docker: &CocoonDocker,
    state: &SpawnerState,
) -> SignalingMessage {
    let cocoon = match state.find_by_container_id(container_id).await {
        Some(c) => c,
        None => {
            return SignalingMessage::TerminateCocoonResult {
                request_id,
                success: false,
                error: Some(format!("container not found: {container_id}")),
            }
        }
    };

    if let Err(e) = docker.terminate_cocoon(&cocoon.container_name).await {
        return SignalingMessage::TerminateCocoonResult {
            request_id,
            success: false,
            error: Some(e.to_string()),
        };
    }

    state.remove_cocoon(&cocoon.container_name).await;
    state.release_token(&cocoon.setup_token).await;

    SignalingMessage::TerminateCocoonResult {
        request_id,
        success: true,
        error: None,
    }
}

/// Periodically check tracked containers and remove dead ones.
pub async fn health_check_loop(docker: CocoonDocker, state: SpawnerState, interval: Duration) {
    loop {
        tokio::time::sleep(interval).await;

        let names = state.container_names().await;
        for name in names {
            if !docker.is_running(&name).await {
                tracing::warn!("container {name} is no longer running, removing from state");
                if let Some(cocoon) = state.remove_cocoon(&name).await {
                    state.release_token(&cocoon.setup_token).await;
                }
            }
        }

        tracing::debug!("health check complete, {} active cocoons", state.count().await);
    }
}

fn spawn_error(request_id: String, error: String) -> SignalingMessage {
    tracing::error!("spawn failed: {error}");
    SignalingMessage::SpawnCocoonResult {
        request_id,
        success: false,
        device_id: None,
        container_id: None,
        error: Some(error),
    }
}
