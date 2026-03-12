use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions, WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::Docker;
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::error::{ExecutorError, Result};
use crate::types::{Package, WorkerRequest, WorkerResponse};

/// Container architecture detected from Docker image
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerArch {
    Amd64,
    Arm64,
}

impl ContainerArch {
    /// Returns the Rust target triple for this architecture
    pub fn target_triple(&self) -> &'static str {
        match self {
            ContainerArch::Amd64 => "x86_64-unknown-linux-musl",
            ContainerArch::Arm64 => "aarch64-unknown-linux-musl",
        }
    }

    /// Returns the binary suffix for this architecture
    pub fn binary_suffix(&self) -> &'static str {
        match self {
            ContainerArch::Amd64 => "x86_64",
            ContainerArch::Arm64 => "aarch64",
        }
    }
}

const CONTAINER_WORKER_BIN: &str = "/usr/local/bin/adi-worker";
const CONTAINER_INPUT_DIR: &str = "/adi/input";
const CONTAINER_OUTPUT_DIR: &str = "/adi/output";
const RESPONSE_FILENAME: &str = "response.json";

pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    pub fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    pub async fn verify_package(&self, package: &Package) -> Result<VerifyInfo> {
        self.pull_image(package).await?;

        let image_url = package.image_url();
        let inspect = self.docker.inspect_image(&image_url).await?;

        Ok(VerifyInfo {
            image_id: inspect.id,
            size: inspect.size.map(|s| s as u64),
        })
    }

    pub async fn pull_image(&self, package: &Package) -> Result<()> {
        let image_url = package.image_url();
        info!(image = %image_url, "Pulling image");

        let mut options = CreateImageOptions {
            from_image: image_url.clone(),
            ..Default::default()
        };

        if let Some(tag_pos) = image_url.rfind(':') {
            let (image, tag) = image_url.split_at(tag_pos);
            options.from_image = image.to_string();
            options.tag = tag.trim_start_matches(':').to_string();
        }

        let credentials =
            package
                .credentials()
                .map(|(user, password)| bollard::auth::DockerCredentials {
                    username: Some(user),
                    password: Some(password),
                    ..Default::default()
                });

        let mut stream = self.docker.create_image(Some(options), None, credentials);

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        debug!(status = %status, "Pull progress");
                    }
                }
                Err(e) => {
                    return Err(ExecutorError::ImagePullFailed(e.to_string()));
                }
            }
        }

        info!(image = %image_url, "Image pulled successfully");
        Ok(())
    }

    pub async fn create_container(
        &self,
        package: &Package,
        job_id: &str,
        request: &WorkerRequest,
    ) -> Result<ContainerInfo> {
        let container_name = format!("adi-executor-{}", job_id);
        let image_url = package.image_url();

        // Detect container architecture and find matching worker binary
        let arch = self.get_image_arch(&image_url).await?;
        let worker_binary = find_worker_binary(arch)?;
        info!(arch = ?arch, binary = %worker_binary.display(), "Selected worker binary for container architecture");

        // Create host directories
        let host_base_dir = std::env::temp_dir().join(format!("adi-executor-{}", job_id));
        let host_input_dir = host_base_dir.join("input");
        let host_output_dir = host_base_dir.join("output");

        for dir in [&host_input_dir, &host_output_dir] {
            tokio::fs::create_dir_all(dir)
                .await
                .map_err(|e| ExecutorError::Internal(format!("create dir: {}", e)))?;
        }

        // Write request.json to input dir
        let request_json = serde_json::to_string_pretty(request)
            .map_err(|e| ExecutorError::Internal(format!("serialize request: {}", e)))?;
        tokio::fs::write(host_input_dir.join("request.json"), &request_json)
            .await
            .map_err(|e| ExecutorError::Internal(format!("write request.json: {}", e)))?;

        // Get original CMD from image
        let original_cmd = self.get_image_cmd(&image_url).await.unwrap_or_default();

        let host_config = HostConfig {
            auto_remove: Some(false),        // We need to read response before removal
            memory: Some(512 * 1024 * 1024), // 512MB limit
            cpu_period: Some(100000),
            cpu_quota: Some(50000), // 50% CPU
            mounts: Some(vec![
                // Worker binary
                Mount {
                    target: Some(CONTAINER_WORKER_BIN.to_string()),
                    source: Some(worker_binary.to_string_lossy().to_string()),
                    typ: Some(MountTypeEnum::BIND),
                    read_only: Some(true),
                    ..Default::default()
                },
                // Input directory
                Mount {
                    target: Some(CONTAINER_INPUT_DIR.to_string()),
                    source: Some(host_input_dir.to_string_lossy().to_string()),
                    typ: Some(MountTypeEnum::BIND),
                    read_only: Some(true),
                    ..Default::default()
                },
                // Output directory
                Mount {
                    target: Some(CONTAINER_OUTPUT_DIR.to_string()),
                    source: Some(host_output_dir.to_string_lossy().to_string()),
                    typ: Some(MountTypeEnum::BIND),
                    read_only: Some(false),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        let config = Config {
            image: Some(image_url),
            host_config: Some(host_config),
            entrypoint: Some(vec![CONTAINER_WORKER_BIN.to_string()]),
            cmd: Some(vec![]),
            env: Some(vec![
                format!("JOB_ID={}", job_id),
                format!("ORIGINAL_CMD={}", original_cmd),
            ]),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        let response = self.docker.create_container(Some(options), config).await?;

        info!(
            container_id = %response.id,
            name = %container_name,
            original_cmd = %original_cmd,
            "Container created"
        );

        Ok(ContainerInfo {
            id: response.id,
            name: container_name,
            output_dir: host_output_dir,
        })
    }

    async fn get_image_cmd(&self, image: &str) -> Option<String> {
        let inspect = self.docker.inspect_image(image).await.ok()?;
        let config = inspect.config?;

        // Prefer Cmd over Entrypoint for the command to execute
        if let Some(cmd) = config.cmd {
            if !cmd.is_empty() {
                return Some(cmd.join(" "));
            }
        }

        if let Some(entrypoint) = config.entrypoint {
            if !entrypoint.is_empty() {
                return Some(entrypoint.join(" "));
            }
        }

        None
    }

    /// Detect the architecture of a Docker image
    async fn get_image_arch(&self, image: &str) -> Result<ContainerArch> {
        let inspect = self.docker.inspect_image(image).await?;

        let arch = inspect.architecture.as_deref().unwrap_or("amd64");

        match arch {
            "amd64" | "x86_64" => Ok(ContainerArch::Amd64),
            "arm64" | "aarch64" => Ok(ContainerArch::Arm64),
            other => {
                warn!(arch = %other, "Unknown architecture, defaulting to amd64");
                Ok(ContainerArch::Amd64)
            }
        }
    }

    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await?;

        info!(container_id = %container_id, "Container started");
        Ok(())
    }

    /// Wait for container to exit and return exit code
    pub async fn wait_container(&self, container_id: &str) -> Result<i64> {
        let options = WaitContainerOptions {
            condition: "not-running",
        };

        let mut stream = self.docker.wait_container(container_id, Some(options));

        match stream.next().await {
            Some(Ok(wait_response)) => {
                let exit_code = wait_response.status_code;
                info!(container_id = %container_id, exit_code = exit_code, "Container exited");
                Ok(exit_code)
            }
            Some(Err(e)) => Err(ExecutorError::Internal(format!(
                "wait container failed: {}",
                e
            ))),
            None => Err(ExecutorError::Internal(
                "wait container stream ended unexpectedly".into(),
            )),
        }
    }

    /// Read worker response from output directory
    pub async fn read_response(&self, output_dir: &Path) -> Result<WorkerResponse> {
        let response_path = output_dir.join(RESPONSE_FILENAME);

        let content = tokio::fs::read_to_string(&response_path)
            .await
            .map_err(|e| {
                ExecutorError::WorkerInvalidResponse(format!(
                    "failed to read {}: {}",
                    response_path.display(),
                    e
                ))
            })?;

        let response: WorkerResponse = serde_json::from_str(&content).map_err(|e| {
            ExecutorError::WorkerInvalidResponse(format!("invalid response JSON: {}", e))
        })?;

        Ok(response)
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<()> {
        let options = StopContainerOptions { t: 10 };

        match self
            .docker
            .stop_container(container_id, Some(options))
            .await
        {
            Ok(_) => {
                info!(container_id = %container_id, "Container stopped");
                Ok(())
            }
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 304, ..
            }) => {
                // Container already stopped
                Ok(())
            }
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => {
                // Container not found (maybe auto-removed)
                warn!(container_id = %container_id, "Container not found during stop");
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<()> {
        let options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };

        match self
            .docker
            .remove_container(container_id, Some(options))
            .await
        {
            Ok(_) => {
                info!(container_id = %container_id, "Container removed");
                Ok(())
            }
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => {
                // Container not found (maybe auto-removed)
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn cleanup_job_dir(&self, job_id: &str) {
        let host_base_dir = std::env::temp_dir().join(format!("adi-executor-{}", job_id));
        if let Err(e) = tokio::fs::remove_dir_all(&host_base_dir).await {
            warn!(path = %host_base_dir.display(), error = %e, "Failed to cleanup job dir");
        }
    }
}

impl Default for DockerClient {
    fn default() -> Self {
        Self::new().expect("Failed to connect to Docker")
    }
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub output_dir: PathBuf,
}

fn find_worker_binary(arch: ContainerArch) -> Result<PathBuf> {
    let arch_suffix = arch.binary_suffix();
    let target_triple = arch.target_triple();

    // Binary names to search for, in order of preference
    let binary_names: Vec<String> = if cfg!(target_os = "linux") {
        // On Linux, prefer arch-specific, then generic
        vec![
            format!("adi-worker-{}", arch_suffix),
            "adi-worker".to_string(),
        ]
    } else {
        // On non-Linux hosts, look for arch-specific Linux binaries
        vec![
            format!("adi-worker-{}", arch_suffix),
            "adi-worker-linux".to_string(),
        ]
    };

    // 1. Check arch-specific env var first, then generic
    let env_vars = [
        format!("ADI_WORKER_BINARY_{}", arch_suffix.to_uppercase()),
        "ADI_WORKER_BINARY".to_string(),
    ];
    for env_var in &env_vars {
        if let Ok(path) = std::env::var(env_var) {
            let path = PathBuf::from(path);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    // 2. Check same directory as current executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in &binary_names {
                let worker = dir.join(name);
                if worker.exists() {
                    return Ok(worker);
                }
            }
        }
    }

    // 3. Check target/release and target/debug (for development)
    let cwd = std::env::current_dir().unwrap_or_default();
    for profile in ["release", "debug"] {
        for name in &binary_names {
            let worker = cwd.join("target").join(profile).join(name);
            if worker.exists() {
                return Ok(worker);
            }
        }
    }

    // 4. Check cross-compiled targets directory (most reliable for cross-compilation)
    for profile in ["release", "debug"] {
        let worker = cwd
            .join("target")
            .join(target_triple)
            .join(profile)
            .join("adi-worker");
        if worker.exists() {
            return Ok(worker);
        }
    }

    Err(ExecutorError::Internal(format!(
        "adi-worker binary for {} not found. Build with: cross build -p adi-worker --release --target {}",
        arch_suffix, target_triple
    )))
}

#[derive(Debug, Clone)]
pub struct VerifyInfo {
    pub image_id: Option<String>,
    pub size: Option<u64>,
}
