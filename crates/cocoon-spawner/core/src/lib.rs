mod config;
mod docker;
mod error;
mod signaling;
mod spawner;
mod state;

pub use config::{KindConfig, SpawnerConfig};
pub use docker::CocoonDocker;
pub use error::SpawnerError;
pub use signaling::run_signaling_loop;
pub use state::{SpawnedCocoon, SpawnerState};

/// Run the cocoon spawner with the given config.
///
/// Connects to the signaling server, registers as a hive,
/// and handles spawn/terminate requests until shutdown.
pub async fn run(config: SpawnerConfig) -> anyhow::Result<()> {
    let docker = CocoonDocker::new()?;
    docker.verify_connection().await?;
    tracing::info!("docker connection verified");

    let state = SpawnerState::new(config.max_concurrent, config.setup_tokens.clone());

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let signaling_handle = {
        let config = config.clone();
        let docker = docker.clone();
        let state = state.clone();
        tokio::spawn(async move {
            run_signaling_loop(config, docker, state, shutdown_rx).await
        })
    };

    let health_handle = {
        let docker = docker.clone();
        let state = state.clone();
        let interval = config.health_check_interval;
        tokio::spawn(async move {
            spawner::health_check_loop(docker, state, interval).await;
        })
    };

    tokio::signal::ctrl_c().await?;
    tracing::info!("shutdown signal received");
    let _ = shutdown_tx.send(true);

    let _ = signaling_handle.await;
    health_handle.abort();

    Ok(())
}
