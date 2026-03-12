//! Docker Runner Plugin for Hive
//!
//! Executes services in Docker containers using the bollard crate.
//!
//! ## Configuration
//!
//! ```yaml
//! runner:
//!   type: docker
//!   docker:
//!     image: postgres:15
//!     ports:
//!       - "{{runtime.port.main}}:5432"
//!     volumes:
//!       - "./data:/var/lib/postgresql/data"
//!     environment:
//!       POSTGRES_PASSWORD: "secret"
//! ```

use anyhow::{anyhow, Context, Result as AnyhowResult};
use bollard::container::{
    Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions, WaitContainerOptions,
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

pub struct DockerRunnerPlugin {
    client: std::sync::OnceLock<Docker>,
    /// Dedicated Tokio runtime for bollard's hyper client.
    /// Needed when loaded as a cdylib — each dylib has separate TLS, so the
    /// host runtime is invisible to bollard's `Handle::current()` call.
    runtime: std::sync::OnceLock<tokio::runtime::Runtime>,
    socket: Option<String>,
}

impl Default for DockerRunnerPlugin {
    fn default() -> Self {
        Self::new_lazy()
    }
}

impl DockerRunnerPlugin {
    /// Creates the plugin without connecting to Docker; connection happens lazily on first use.
    pub fn new_lazy() -> Self {
        Self {
            client: std::sync::OnceLock::new(),
            runtime: std::sync::OnceLock::new(),
            socket: None,
        }
    }

    pub fn try_new() -> AnyhowResult<Self> {
        let plugin = Self::new_lazy();
        plugin.get_client()?;
        Ok(plugin)
    }

    pub fn new() -> Self {
        Self::try_new().expect("Failed to connect to Docker. Is Docker running?")
    }

    pub fn with_socket(socket: &str) -> AnyhowResult<Self> {
        let plugin = Self {
            client: std::sync::OnceLock::new(),
            runtime: std::sync::OnceLock::new(),
            socket: Some(socket.to_string()),
        };
        plugin.get_client()?;
        Ok(plugin)
    }

    fn connect_docker(socket: Option<&str>) -> AnyhowResult<Docker> {
        if let Some(socket_path) = socket {
            Docker::connect_with_socket(socket_path, 120, bollard::API_DEFAULT_VERSION)
                .context("Failed to connect to Docker socket")
        } else {
            Docker::connect_with_local_defaults()
                .or_else(|_| Docker::connect_with_unix_defaults())
                .or_else(|_| {
                    Docker::connect_with_socket("/var/run/docker.sock", 120, bollard::API_DEFAULT_VERSION)
                })
                .context("Failed to connect to Docker. Is Docker running?")
        }
    }

    /// Returns the dedicated runtime handle for this cdylib.
    ///
    /// Each cdylib has its own copy of Tokio's TLS, so the host runtime is
    /// invisible to bollard's hyper client, timers, etc. All Docker async
    /// operations are spawned on this runtime.
    fn runtime_handle(&self) -> tokio::runtime::Handle {
        self.runtime
            .get_or_init(|| {
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .enable_all()
                    .build()
                    .expect("Failed to create Tokio runtime for Docker client")
            })
            .handle()
            .clone()
    }

    /// Returns a clone of the Docker client, lazily connecting on first call.
    fn get_client(&self) -> AnyhowResult<Docker> {
        if let Some(client) = self.client.get() {
            return Ok(client.clone());
        }
        // enter() guard is sync-only — dropped before any await points
        let _guard = self.runtime_handle().enter();
        let client = Self::connect_docker(self.socket.as_deref())?;
        let _ = self.client.set(client);
        Ok(self
            .client
            .get()
            .ok_or_else(|| anyhow!("Failed to initialize Docker client"))?
            .clone())
    }

    fn extract_config(config: &serde_json::Value) -> AnyhowResult<DockerConfig> {
        let docker_value = config
            .get("docker")
            .ok_or_else(|| anyhow!("Missing 'docker' configuration for docker runner"))?;

        serde_json::from_value(docker_value.clone())
            .context("Failed to parse docker runner configuration")
    }
}

/// Pulls the Docker image if not already present locally.
async fn pull_image_if_needed(client: &Docker, image: &str) -> AnyhowResult<()> {
    use futures::StreamExt;

    let mut filters = HashMap::new();
    filters.insert("reference", vec![image]);
    let options = ListImagesOptions { filters, ..Default::default() };
    let images = client.list_images(Some(options)).await?;
    if !images.is_empty() {
        info!("Image already exists locally: {}", image);
        return Ok(());
    }

    info!("Pulling Docker image: {}", image);
    let mut stream = client.create_image(
        Some(CreateImageOptions { from_image: image, ..Default::default() }),
        None,
        None,
    );
    while let Some(result) = stream.next().await {
        result?;
    }
    info!("Image pulled successfully: {}", image);
    Ok(())
}

#[async_trait]
impl Plugin for DockerRunnerPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.runner.docker".to_string(),
            name: "docker".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Docker container runner plugin".to_string()),
            category: Some(PluginCategory::Runner),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        let socket = ctx.config.get("socket").and_then(|v| v.as_str());
        if let Some(s) = socket {
            self.socket = Some(s.to_string());
        }
        match self.get_client() {
            Ok(_) => info!("Docker client initialized successfully"),
            Err(e) => {
                warn!("Failed to connect to Docker: {}", e);
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
impl Runner for DockerRunnerPlugin {
    async fn start(
        &self,
        service_name: &str,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> PluginResult<ProcessHandle> {
        let docker_config = Self::extract_config(config)?;
        let container_name = docker_config
            .container_name
            .clone()
            .unwrap_or_else(|| format!("hive-{}", service_name));

        let mut port_bindings = HashMap::new();
        let mut exposed_ports = HashMap::new();

        for port_spec in &docker_config.ports {
            // Split at the last colon so "${PORT:main}:80" correctly separates
            // host ("${PORT:main}") from container port ("80").
            let Some((host_part, container_port)) = port_spec.rsplit_once(':') else {
                continue;
            };
            let host_port_str = ctx.interpolate(host_part)?;

            // Exposed port key (e.g., "5432/tcp")
            let exposed_key = if container_port.contains('/') {
                container_port.to_string()
            } else {
                format!("{}/tcp", container_port)
            };

            exposed_ports.insert(exposed_key.clone(), HashMap::new());

            port_bindings.insert(
                exposed_key,
                Some(vec![bollard::service::PortBinding {
                    host_ip: Some(docker_config.bind_ip.clone()),
                    host_port: Some(host_port_str),
                }]),
            );
        }

        let mut env_vec: Vec<String> = env.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        for (k, v) in &docker_config.environment {
            let interpolated = ctx.interpolate(v)?;
            env_vec.push(format!("{}={}", k, interpolated));
        }

        // Relative host paths are validated against the working directory to block path-traversal.
        let binds: Vec<String> = docker_config
            .volumes
            .iter()
            .filter_map(|v| {
                // Format: "HOST:CONTAINER" or "HOST:CONTAINER:MODE"
                let parts: Vec<&str> = v.splitn(2, ':').collect();
                if parts.len() >= 2 {
                    let host_path = parts[0];
                    let rest = &v[host_path.len()..]; // ":CONTAINER" or ":CONTAINER:MODE"

                    if host_path.starts_with("./") || host_path.starts_with("../") {
                        let abs_path = ctx.working_dir.join(host_path);

                        // Canonicalize to resolve symlinks, then ensure the result stays within working_dir.
                        match abs_path.canonicalize() {
                            Ok(canonical) => {
                                // Security: Ensure the resolved path is within the working directory
                                let working_dir_canonical = match ctx.working_dir.canonicalize() {
                                    Ok(p) => p,
                                    Err(e) => {
                                        warn!("Cannot canonicalize working dir: {}", e);
                                        return None;
                                    }
                                };
                                
                                if !canonical.starts_with(&working_dir_canonical) {
                                    warn!(
                                        "Path traversal blocked: {} resolves outside working directory",
                                        host_path
                                    );
                                    return None;
                                }
                                
                                Some(format!("{}{}", canonical.display(), rest))
                            }
                            Err(e) => {
                                if host_path.contains("..") {
                                    let normalized = abs_path.components().collect::<std::path::PathBuf>();
                                    if !normalized.starts_with(&ctx.working_dir) {
                                        warn!(
                                            "Suspicious path pattern blocked: {} ({})",
                                            host_path, e
                                        );
                                        return None;
                                    }
                                }
                                // Path doesn't exist yet but may be created at runtime.
                                debug!("Volume path {} does not exist yet: {}", host_path, e);
                                Some(format!("{}{}", abs_path.display(), rest))
                            }
                        }
                    } else if host_path.starts_with('/') {
                        Some(v.clone())
                    } else {
                        Some(v.clone())
                    }
                } else {
                    Some(v.clone())
                }
            })
            .collect();

        let mut cap_drop = Vec::new();
        if docker_config.security.cap_drop_all {
            cap_drop.push("ALL".to_string());
        }

        let cap_add = if docker_config.security.cap_add.is_empty() {
            None
        } else {
            Some(docker_config.security.cap_add.clone())
        };

        let mut security_opt = docker_config.security.security_opt.clone();
        if docker_config.security.no_new_privileges {
            security_opt.push("no-new-privileges:true".to_string());
        }

        let container_config = Config {
            image: Some(docker_config.image.clone()),
            cmd: docker_config.command.clone(),
            entrypoint: docker_config.entrypoint.clone(),
            env: Some(env_vec),
            exposed_ports: Some(exposed_ports),
            user: docker_config.security.user.clone(),
            host_config: Some(bollard::service::HostConfig {
                port_bindings: Some(port_bindings),
                binds: Some(binds),
                cap_drop: if cap_drop.is_empty() { None } else { Some(cap_drop) },
                cap_add,
                security_opt: if security_opt.is_empty() { None } else { Some(security_opt) },
                readonly_rootfs: Some(docker_config.security.read_only),
                ..Default::default()
            }),
            ..Default::default()
        };

        let client = self.get_client()?;
        let rt = self.runtime_handle();
        let image = docker_config.image.clone();
        let deny_pull = docker_config.deny_pull;

        let inner = rt.spawn(async move {
            if !deny_pull {
                pull_image_if_needed(&client, &image).await
                    .context(format!("Failed to pull image '{}'", image))?;
            }

            let _ = client
                .remove_container(
                    &container_name,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await;

            info!(
                "Creating container {} from image {}",
                container_name, image
            );
            client
                .create_container(
                    Some(CreateContainerOptions {
                        name: &container_name,
                        platform: None,
                    }),
                    container_config,
                )
                .await
                .context("Failed to create container")?;

            client
                .start_container(&container_name, None::<StartContainerOptions<String>>)
                .await
                .context("Failed to start container")?;

            info!("Container {} started", container_name);

            Ok::<_, anyhow::Error>(
                ProcessHandle::docker(container_name)
                    .with_metadata("image", &image),
            )
        })
        .await
        .map_err(|e| anyhow!("Docker task panicked: {}", e))?;

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

        let inner = rt.spawn(async move {
            info!("Stopping container {}", container_name);
            client
                .stop_container(&container_name, Some(StopContainerOptions { t: 10 }))
                .await
                .context("Failed to stop container")?;

            client
                .remove_container(
                    &container_name,
                    Some(RemoveContainerOptions {
                        force: false,
                        ..Default::default()
                    }),
                )
                .await
                .context("Failed to remove container")?;

            info!("Container {} stopped and removed", container_name);
            Ok::<_, anyhow::Error>(())
        })
        .await
        .map_err(|e| anyhow!("Docker task panicked: {}", e))?;

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

    async fn logs(&self, handle: &ProcessHandle, lines: Option<usize>) -> PluginResult<Vec<String>> {
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

        let inner = rt.spawn(async move {
            use futures::StreamExt;
            let options = LogsOptions::<String> {
                stdout: true,
                stderr: true,
                tail,
                ..Default::default()
            };

            let mut logs_stream = client.logs(&container_name, Some(options));
            let mut logs = Vec::new();

            while let Some(result) = logs_stream.next().await {
                match result {
                    Ok(output) => logs.push(output.to_string()),
                    Err(e) => {
                        debug!("Error reading log: {}", e);
                        break;
                    }
                }
            }

            Ok::<_, anyhow::Error>(logs)
        })
        .await
        .map_err(|e| anyhow!("Docker task panicked: {}", e))?;

        Ok(inner?)
    }

    fn supports_hooks(&self) -> bool {
        true
    }

    async fn run_hook(
        &self,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> PluginResult<HookExitStatus> {
        let docker_config = Self::extract_config(config)?;

        let hook_id = uuid::Uuid::new_v4().to_string();
        let container_name = format!("hive-hook-{}", &hook_id[..8]);

        let mut port_bindings = HashMap::new();
        let mut exposed_ports = HashMap::new();

        for port_spec in &docker_config.ports {
            let parts: Vec<&str> = port_spec.split(':').collect();
            if parts.len() == 2 {
                let host_port_str = ctx.interpolate(parts[0])?;
                let container_port = parts[1];
                let exposed_key = if container_port.contains('/') {
                    container_port.to_string()
                } else {
                    format!("{}/tcp", container_port)
                };
                exposed_ports.insert(exposed_key.clone(), HashMap::new());
                port_bindings.insert(
                    exposed_key,
                    Some(vec![bollard::service::PortBinding {
                        host_ip: Some(docker_config.bind_ip.clone()),
                        host_port: Some(host_port_str),
                    }]),
                );
            }
        }

        let mut env_vec: Vec<String> = env.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        for (k, v) in &docker_config.environment {
            let interpolated = ctx.interpolate(v)?;
            env_vec.push(format!("{}={}", k, interpolated));
        }

        let binds: Vec<String> = docker_config
            .volumes
            .iter()
            .filter_map(|v| {
                // Format: "HOST:CONTAINER" or "HOST:CONTAINER:MODE"
                let parts: Vec<&str> = v.splitn(2, ':').collect();
                if parts.len() >= 2 {
                    let host_path = parts[0];
                    let rest = &v[host_path.len()..];

                    if host_path.starts_with("./") || host_path.starts_with("../") {
                        let abs_path = ctx.working_dir.join(host_path);

                        match abs_path.canonicalize() {
                            Ok(canonical) => {
                                let working_dir_canonical = match ctx.working_dir.canonicalize() {
                                    Ok(p) => p,
                                    Err(e) => {
                                        warn!("Cannot canonicalize working dir: {}", e);
                                        return None;
                                    }
                                };
                                
                                if !canonical.starts_with(&working_dir_canonical) {
                                    warn!(
                                        "Path traversal blocked in hook: {} resolves outside working directory",
                                        host_path
                                    );
                                    return None;
                                }
                                
                                Some(format!("{}{}", canonical.display(), rest))
                            }
                            Err(_) => {
                                if host_path.contains("..") {
                                    warn!("Suspicious path pattern in hook: {}", host_path);
                                    return None;
                                }
                                Some(format!("{}{}", abs_path.display(), rest))
                            }
                        }
                    } else if host_path.starts_with('/') {
                        Some(v.clone())
                    } else {
                        Some(v.clone())
                    }
                } else {
                    Some(v.clone())
                }
            })
            .collect();

        let mut cap_drop = Vec::new();
        if docker_config.security.cap_drop_all {
            cap_drop.push("ALL".to_string());
        }

        let cap_add = if docker_config.security.cap_add.is_empty() {
            None
        } else {
            Some(docker_config.security.cap_add.clone())
        };

        let mut security_opt = docker_config.security.security_opt.clone();
        if docker_config.security.no_new_privileges {
            security_opt.push("no-new-privileges:true".to_string());
        }

        let container_config = Config {
            image: Some(docker_config.image.clone()),
            cmd: docker_config.command.clone(),
            entrypoint: docker_config.entrypoint.clone(),
            env: Some(env_vec),
            user: docker_config.security.user.clone(),
            exposed_ports: if exposed_ports.is_empty() {
                None
            } else {
                Some(exposed_ports)
            },
            host_config: Some(bollard::service::HostConfig {
                port_bindings: if port_bindings.is_empty() {
                    None
                } else {
                    Some(port_bindings)
                },
                binds: if binds.is_empty() { None } else { Some(binds) },
                auto_remove: Some(true),
                network_mode: docker_config.network_mode.clone(),
                cap_drop: if cap_drop.is_empty() { None } else { Some(cap_drop) },
                cap_add,
                security_opt: if security_opt.is_empty() { None } else { Some(security_opt) },
                readonly_rootfs: Some(docker_config.security.read_only),
                ..Default::default()
            }),
            ..Default::default()
        };

        let client = self.get_client()?;
        let rt = self.runtime_handle();
        let image = docker_config.image.clone();
        let deny_pull = docker_config.deny_pull;

        let inner = rt.spawn(async move {
            use futures::StreamExt;

            if !deny_pull {
                pull_image_if_needed(&client, &image).await
                    .context(format!("Failed to pull image '{}'", image))?;
            }

            let _ = client
                .remove_container(
                    &container_name,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await;

            info!(
                "Running hook container {} from image {}",
                container_name, image
            );

            client
                .create_container(
                    Some(CreateContainerOptions {
                        name: &container_name,
                        platform: None,
                    }),
                    container_config,
                )
                .await
                .context("Failed to create hook container")?;

            client
                .start_container(&container_name, None::<StartContainerOptions<String>>)
                .await
                .context("Failed to start hook container")?;

            let mut wait_stream = client.wait_container(
                &container_name,
                Some(WaitContainerOptions {
                    condition: "not-running",
                }),
            );

            let exit_code = match wait_stream.next().await {
                Some(Ok(response)) => response.status_code as i32,
                Some(Err(e)) => {
                    warn!("Error waiting for hook container: {}", e);
                    -1
                }
                None => {
                    warn!("Hook container wait stream ended unexpectedly");
                    -1
                }
            };

            let mut logs_stream = client.logs(
                &container_name,
                Some(LogsOptions::<String> {
                    stdout: true,
                    stderr: true,
                    tail: "all".to_string(),
                    ..Default::default()
                }),
            );

            let mut stdout = String::new();
            let stderr = String::new();

            while let Some(result) = logs_stream.next().await {
                match result {
                    Ok(output) => stdout.push_str(&output.to_string()),
                    Err(_) => break,
                }
            }

            info!(
                "Hook container {} finished with exit code {}",
                container_name, exit_code
            );

            let mut status = HookExitStatus {
                code: exit_code,
                output: None,
                stderr: None,
            };
            if !stdout.is_empty() {
                status.output = Some(stdout);
            }
            if !stderr.is_empty() {
                status.stderr = Some(stderr);
            }
            Ok::<_, anyhow::Error>(status)
        })
        .await
        .map_err(|e| anyhow!("Docker task panicked: {}", e))?;

        Ok(inner?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    pub image: String,
    /// Custom container name. Defaults to `hive-<service_name>` if not set.
    #[serde(default)]
    pub container_name: Option<String>,
    /// Format: `"HOST:CONTAINER"` or `"HOST:CONTAINER/proto"`
    #[serde(default)]
    pub ports: Vec<String>,
    /// Format: `"HOST:CONTAINER"` or `"HOST:CONTAINER:MODE"`
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    pub command: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub network_mode: Option<String>,
    pub restart: Option<String>,
    /// If `true`, skip automatic image pull before container creation.
    #[serde(default)]
    pub deny_pull: bool,
    #[serde(default)]
    pub security: DockerSecurityConfig,
    /// Host IP for port bindings. Defaults to `"127.0.0.1"` (local-only).
    #[serde(default = "default_bind_ip")]
    pub bind_ip: String,
}

fn default_bind_ip() -> String {
    "127.0.0.1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerSecurityConfig {
    /// Drops all Linux capabilities by default (secure baseline).
    #[serde(default = "default_true")]
    pub cap_drop_all: bool,
    /// Capabilities to restore after dropping all (e.g. `["NET_BIND_SERVICE"]`).
    #[serde(default)]
    pub cap_add: Vec<String>,
    #[serde(default = "default_true")]
    pub no_new_privileges: bool,
    #[serde(default)]
    pub read_only: bool,
    /// e.g. `"1000:1000"` or `"nobody"`
    pub user: Option<String>,
    /// e.g. `["seccomp=unconfined"]`
    #[serde(default)]
    pub security_opt: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for DockerSecurityConfig {
    fn default() -> Self {
        Self {
            cap_drop_all: true,
            cap_add: Vec::new(),
            no_new_privileges: true,
            read_only: false,
            user: None,
            security_opt: Vec::new(),
        }
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(DockerRunnerPlugin::new_lazy())
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create_runner() -> Box<dyn Runner> {
    Box::new(DockerRunnerPlugin::new_lazy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = DockerRunnerPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.runner.docker");
        assert_eq!(meta.name, "docker");
    }

    #[test]
    fn test_extract_config() {
        let config = serde_json::json!({
            "docker": {
                "image": "postgres:15",
                "ports": ["5432:5432"],
                "environment": {
                    "POSTGRES_PASSWORD": "secret"
                }
            }
        });

        let docker_config = DockerRunnerPlugin::extract_config(&config).unwrap();
        assert_eq!(docker_config.image, "postgres:15");
        assert_eq!(docker_config.ports.len(), 1);
    }
}
