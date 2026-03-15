//! Cocoon Spawner Runner Plugin for Hive
//!
//! Spawns cocoon containers via Docker with signaling-specific environment
//! variables injected automatically.
//!
//! ## Configuration
//!
//! ```yaml
//! runner:
//!   type: cocoon-spawner
//!   cocoon-spawner:
//!     image: adi/cocoon-ubuntu:latest
//!     signaling_url: ws://signaling.example.com/ws
//!     setup_token: <token>
//!     ice_servers: stun:stun.l.google.com:19302
//! ```

use anyhow::{anyhow, Context, Result as AnyhowResult};
use bollard::container::{
    Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use bollard::Docker;
use lib_plugin_abi_v3::{
    async_trait,
    hooks::HookExitStatus,
    runner::{ProcessHandle, Runner, RuntimeContext},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_RUNNER,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub struct CocoonRunnerPlugin {
    client: std::sync::OnceLock<Docker>,
    runtime: std::sync::OnceLock<tokio::runtime::Runtime>,
}

impl Default for CocoonRunnerPlugin {
    fn default() -> Self {
        Self::new_lazy()
    }
}

impl CocoonRunnerPlugin {
    pub fn new_lazy() -> Self {
        Self {
            client: std::sync::OnceLock::new(),
            runtime: std::sync::OnceLock::new(),
        }
    }

    fn runtime_handle(&self) -> tokio::runtime::Handle {
        self.runtime
            .get_or_init(|| {
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .enable_all()
                    .build()
                    .expect("Failed to create Tokio runtime for cocoon runner")
            })
            .handle()
            .clone()
    }

    fn get_client(&self) -> AnyhowResult<Docker> {
        if let Some(client) = self.client.get() {
            return Ok(client.clone());
        }
        let _guard = self.runtime_handle().enter();
        let client = Docker::connect_with_local_defaults()
            .or_else(|_| Docker::connect_with_unix_defaults())
            .or_else(|_| {
                Docker::connect_with_socket(
                    "/var/run/docker.sock",
                    120,
                    bollard::API_DEFAULT_VERSION,
                )
            })
            .context("Failed to connect to Docker. Is Docker running?")?;
        let _ = self.client.set(client);
        Ok(self
            .client
            .get()
            .ok_or_else(|| anyhow!("Failed to initialize Docker client"))?
            .clone())
    }

    fn extract_config(config: &serde_json::Value) -> AnyhowResult<CocoonConfig> {
        let value = config
            .get("cocoon-spawner")
            .ok_or_else(|| anyhow!("Missing 'cocoon-spawner' configuration for cocoon-spawner runner"))?;
        serde_json::from_value(value.clone()).context("Failed to parse cocoon runner configuration")
    }
}

#[async_trait]
impl Plugin for CocoonRunnerPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.runner.cocoon-spawner".to_string(),
            name: "cocoon-spawner".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Cocoon Spawner runner plugin".to_string()),
            category: Some(PluginCategory::Runner),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        match self.get_client() {
            Ok(_) => info!("Cocoon runner: Docker client initialized"),
            Err(e) => {
                warn!("Cocoon runner: failed to connect to Docker: {}", e);
                return Err(e.into());
            }
        }
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_RUNNER]
    }
}

#[async_trait]
impl Runner for CocoonRunnerPlugin {
    async fn start(
        &self,
        service_name: &str,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        _ctx: &RuntimeContext,
    ) -> PluginResult<ProcessHandle> {
        let cocoon_config = Self::extract_config(config)?;
        let container_name = format!("cocoon-{service_name}");

        let mut env_vec: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();

        // Inject cocoon-specific environment
        env_vec.push(format!(
            "SIGNALING_SERVER_URL={}",
            cocoon_config.signaling_url
        ));
        if let Some(token) = &cocoon_config.setup_token {
            env_vec.push(format!("COCOON_SETUP_TOKEN={token}"));
        }
        if let Some(ice) = &cocoon_config.ice_servers {
            env_vec.push(format!("WEBRTC_ICE_SERVERS={ice}"));
        }
        if let Some(turn_user) = &cocoon_config.turn_username {
            env_vec.push(format!("WEBRTC_TURN_USERNAME={turn_user}"));
        }
        if let Some(turn_cred) = &cocoon_config.turn_credential {
            env_vec.push(format!("WEBRTC_TURN_CREDENTIAL={turn_cred}"));
        }

        let container_config = Config {
            image: Some(cocoon_config.image.clone()),
            env: Some(env_vec),
            host_config: Some(bollard::service::HostConfig {
                cap_drop: Some(vec!["ALL".to_string()]),
                security_opt: Some(vec!["no-new-privileges:true".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let client = self.get_client()?;
        let rt = self.runtime_handle();
        let image = cocoon_config.image.clone();

        let inner = rt
            .spawn(async move {
                pull_image_if_needed(&client, &image).await?;

                let _ = client
                    .remove_container(
                        &container_name,
                        Some(RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    )
                    .await;

                info!("Creating cocoon container {container_name} from {image}");
                client
                    .create_container(
                        Some(CreateContainerOptions {
                            name: &container_name,
                            platform: None,
                        }),
                        container_config,
                    )
                    .await
                    .context("Failed to create cocoon container")?;

                client
                    .start_container(&container_name, None::<StartContainerOptions<String>>)
                    .await
                    .context("Failed to start cocoon container")?;

                info!("Cocoon container {container_name} started");
                Ok::<_, anyhow::Error>(
                    ProcessHandle::docker(container_name).with_metadata("image", &image),
                )
            })
            .await
            .map_err(|e| anyhow!("Cocoon task panicked: {e}"))?;

        Ok(inner?)
    }

    async fn stop(&self, handle: &ProcessHandle) -> PluginResult<()> {
        let container_name = handle
            .container_name
            .as_ref()
            .ok_or_else(|| anyhow!("Missing container name in handle"))?
            .clone();

        let client = self.get_client()?;
        let rt = self.runtime_handle();

        let inner = rt
            .spawn(async move {
                info!("Stopping cocoon container {container_name}");
                client
                    .stop_container(
                        &container_name,
                        Some(StopContainerOptions { t: 10 }),
                    )
                    .await
                    .context("Failed to stop cocoon container")?;

                client
                    .remove_container(
                        &container_name,
                        Some(RemoveContainerOptions {
                            force: false,
                            ..Default::default()
                        }),
                    )
                    .await
                    .context("Failed to remove cocoon container")?;

                info!("Cocoon container {container_name} stopped and removed");
                Ok::<_, anyhow::Error>(())
            })
            .await
            .map_err(|e| anyhow!("Cocoon task panicked: {e}"))?;

        Ok(inner?)
    }

    async fn is_running(&self, handle: &ProcessHandle) -> bool {
        let container_name = match &handle.container_name {
            Some(name) => name.clone(),
            None => return false,
        };

        let client = match self.get_client() {
            Ok(c) => c,
            Err(_) => return false,
        };

        let rt = self.runtime_handle();
        rt.spawn(async move {
            match client.inspect_container(&container_name, None).await {
                Ok(info) => info.state.and_then(|s| s.running).unwrap_or(false),
                Err(_) => false,
            }
        })
        .await
        .unwrap_or(false)
    }

    async fn logs(
        &self,
        handle: &ProcessHandle,
        lines: Option<usize>,
    ) -> PluginResult<Vec<String>> {
        let container_name = handle
            .container_name
            .as_ref()
            .ok_or_else(|| anyhow!("Missing container name in handle"))?
            .clone();

        let client = self.get_client()?;
        let rt = self.runtime_handle();
        let tail = lines
            .map(|l| l.to_string())
            .unwrap_or_else(|| "100".to_string());

        let inner = rt
            .spawn(async move {
                use futures::StreamExt;
                let options = LogsOptions::<String> {
                    stdout: true,
                    stderr: true,
                    tail,
                    ..Default::default()
                };

                let mut stream = client.logs(&container_name, Some(options));
                let mut logs = Vec::new();

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(output) => logs.push(output.to_string()),
                        Err(e) => {
                            debug!("Error reading cocoon log: {e}");
                            break;
                        }
                    }
                }
                Ok::<_, anyhow::Error>(logs)
            })
            .await
            .map_err(|e| anyhow!("Cocoon task panicked: {e}"))?;

        Ok(inner?)
    }

    fn supports_hooks(&self) -> bool {
        false
    }

    async fn run_hook(
        &self,
        _config: &serde_json::Value,
        _env: HashMap<String, String>,
        _ctx: &RuntimeContext,
    ) -> PluginResult<HookExitStatus> {
        Err(anyhow!("Cocoon runner does not support hooks").into())
    }
}

async fn pull_image_if_needed(client: &Docker, image: &str) -> AnyhowResult<()> {
    use futures::StreamExt;

    let mut filters = HashMap::new();
    filters.insert("reference", vec![image]);
    let options = ListImagesOptions {
        filters,
        ..Default::default()
    };
    let images = client.list_images(Some(options)).await?;
    if !images.is_empty() {
        return Ok(());
    }

    info!("Pulling cocoon image: {image}");
    let mut stream = client.create_image(
        Some(CreateImageOptions {
            from_image: image,
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(result) = stream.next().await {
        result?;
    }
    info!("Image pulled: {image}");
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoonConfig {
    pub image: String,
    pub signaling_url: String,
    pub setup_token: Option<String>,
    pub ice_servers: Option<String>,
    pub turn_username: Option<String>,
    pub turn_credential: Option<String>,
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CocoonRunnerPlugin::new_lazy())
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create_runner() -> Box<dyn Runner> {
    Box::new(CocoonRunnerPlugin::new_lazy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = CocoonRunnerPlugin::new_lazy();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.runner.cocoon-spawner");
        assert_eq!(meta.name, "cocoon-spawner");
    }

    #[test]
    fn test_extract_config() {
        let config = serde_json::json!({
            "cocoon-spawner": {
                "image": "adi/cocoon-ubuntu:latest",
                "signaling_url": "ws://signaling.example.com/ws",
                "setup_token": "abc123",
                "ice_servers": "stun:stun.l.google.com:19302"
            }
        });

        let cocoon_config = CocoonRunnerPlugin::extract_config(&config).unwrap();
        assert_eq!(cocoon_config.image, "adi/cocoon-ubuntu:latest");
        assert_eq!(cocoon_config.signaling_url, "ws://signaling.example.com/ws");
        assert_eq!(cocoon_config.setup_token, Some("abc123".to_string()));
    }
}
