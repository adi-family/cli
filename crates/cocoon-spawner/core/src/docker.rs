use anyhow::{anyhow, Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
    RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::service::HostConfig;
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;

use crate::KindConfig;

const DOCKER_API_TIMEOUT: u64 = 120;

/// Docker client wrapper for cocoon container lifecycle management.
#[derive(Debug, Clone)]
pub struct CocoonDocker {
    client: Docker,
}

impl CocoonDocker {
    /// Connect to the Docker daemon via socket discovery.
    pub fn new() -> Result<Self> {
        let socket_paths = docker_socket_paths();
        for path in &socket_paths {
            if std::path::Path::new(path).exists() {
                match Docker::connect_with_socket(path, DOCKER_API_TIMEOUT, bollard::API_DEFAULT_VERSION)
                {
                    Ok(client) => {
                        tracing::debug!("connected to docker socket: {path}");
                        return Ok(Self { client });
                    }
                    Err(e) => {
                        tracing::warn!("failed to connect to {path}: {e}");
                    }
                }
            }
        }

        Docker::connect_with_local_defaults()
            .map(|client| Self { client })
            .map_err(|e| anyhow!("no docker socket found: {e}"))
    }

    /// Verify the Docker daemon is reachable.
    pub async fn verify_connection(&self) -> Result<()> {
        tokio::time::timeout(std::time::Duration::from_secs(5), self.client.ping())
            .await
            .map_err(|_| anyhow!("docker daemon did not respond within 5s"))?
            .context("docker daemon is not reachable")?;
        Ok(())
    }

    /// Spawn a cocoon container with the given configuration.
    ///
    /// Returns the container ID.
    pub async fn spawn_cocoon(
        &self,
        name: &str,
        kind_config: &KindConfig,
        config: &crate::config::SpawnerConfig,
        setup_token: &str,
    ) -> Result<String> {
        self.pull_image_if_needed(&kind_config.image).await?;

        // Force remove any existing container with same name
        let _ = self
            .client
            .remove_container(
                name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        let mut env = vec![
            format!("SIGNALING_SERVER_URL={}", config.signaling_url),
            format!("COCOON_SETUP_TOKEN={setup_token}"),
        ];

        if let Some(ice) = &config.webrtc_ice_servers {
            env.push(format!("WEBRTC_ICE_SERVERS={ice}"));
        }
        if let Some(user) = &config.webrtc_turn_username {
            env.push(format!("WEBRTC_TURN_USERNAME={user}"));
        }
        if let Some(cred) = &config.webrtc_turn_credential {
            env.push(format!("WEBRTC_TURN_CREDENTIAL={cred}"));
        }

        let mut host_config = HostConfig {
            binds: Some(vec![format!("{name}:/cocoon")]),
            restart_policy: Some(bollard::service::RestartPolicy {
                name: Some(bollard::service::RestartPolicyNameEnum::UNLESS_STOPPED),
                ..Default::default()
            }),
            ..Default::default()
        };

        if let Some(nano_cpus) = kind_config.cpu_limit {
            host_config.nano_cpus = Some(nano_cpus);
        }
        if let Some(mem_mb) = kind_config.memory_limit_mb {
            host_config.memory = Some(mem_mb * 1024 * 1024);
        }

        // Handle .local domains with --add-host
        if config.signaling_url.contains(".local") {
            host_config.extra_hosts = Some(vec!["host.docker.internal:host-gateway".to_string()]);
        }

        let container_config = Config {
            image: Some(kind_config.image.clone()),
            env: Some(env),
            host_config: Some(host_config),
            ..Default::default()
        };

        tracing::info!("creating container {name} from {}", kind_config.image);

        let response = self
            .client
            .create_container(
                Some(CreateContainerOptions {
                    name,
                    platform: None,
                }),
                container_config,
            )
            .await
            .with_context(|| format!("failed to create container '{name}'"))?;

        self.client
            .start_container(name, None::<StartContainerOptions<String>>)
            .await
            .with_context(|| format!("failed to start container '{name}'"))?;

        tracing::info!("container {name} started (id={})", &response.id[..12]);
        Ok(response.id)
    }

    /// Stop and remove a cocoon container.
    pub async fn terminate_cocoon(&self, container_name: &str) -> Result<()> {
        tracing::info!("terminating container {container_name}");

        self.client
            .stop_container(container_name, Some(StopContainerOptions { t: 10 }))
            .await
            .with_context(|| format!("failed to stop container '{container_name}'"))?;

        self.client
            .remove_container(
                container_name,
                Some(RemoveContainerOptions {
                    force: false,
                    ..Default::default()
                }),
            )
            .await
            .with_context(|| format!("failed to remove container '{container_name}'"))?;

        tracing::info!("container {container_name} terminated");
        Ok(())
    }

    /// Check if a container is currently running.
    pub async fn is_running(&self, container_name: &str) -> bool {
        match self
            .client
            .inspect_container(container_name, None::<InspectContainerOptions>)
            .await
        {
            Ok(info) => info.state.and_then(|s| s.running).unwrap_or(false),
            Err(_) => false,
        }
    }

    /// List spawner-managed containers by name prefix.
    pub async fn list_spawned(&self, prefix: &str) -> Vec<String> {
        let mut filters = HashMap::new();
        filters.insert("name", vec![prefix]);

        match self
            .client
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters,
                ..Default::default()
            }))
            .await
        {
            Ok(containers) => containers
                .into_iter()
                .filter_map(|c| {
                    c.names
                        .and_then(|names| names.into_iter().next())
                        .map(|n| n.trim_start_matches('/').to_string())
                })
                .collect(),
            Err(e) => {
                tracing::warn!("failed to list containers: {e}");
                vec![]
            }
        }
    }

    async fn pull_image_if_needed(&self, image: &str) -> Result<()> {
        tracing::info!("pulling image {image}");
        let opts = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = self.client.create_image(Some(opts), None, None);
        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        tracing::debug!("pull: {status}");
                    }
                }
                Err(e) => {
                    tracing::warn!("pull failed (may use cached): {e}");
                    break;
                }
            }
        }
        Ok(())
    }
}

fn docker_socket_paths() -> Vec<String> {
    let mut paths = vec!["/var/run/docker.sock".to_string()];
    if let Ok(home) = std::env::var("HOME") {
        paths.push(format!("{home}/.docker/run/docker.sock"));
    }
    paths
}
