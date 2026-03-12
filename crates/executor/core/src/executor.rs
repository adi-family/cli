use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::docker::DockerClient;
use crate::error::{ApiError, ErrorCode, Result};
use crate::output::handle_output;
use crate::store::JobStore;
use crate::types::{Job, JobStatus, OutputConfig, Package, VerifyResult, WorkerRequest};

const MAX_CONCURRENT_JOBS: usize = 10;

pub struct Executor {
    docker: DockerClient,
    store: JobStore,
    semaphore: Arc<Semaphore>,
}

impl Executor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            docker: DockerClient::new()?,
            store: JobStore::new(),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_JOBS)),
        })
    }

    pub async fn verify_package(&self, package: &Package) -> Result<VerifyResult> {
        match self.docker.verify_package(package).await {
            Ok(info) => Ok(VerifyResult {
                valid: true,
                error: None,
                image_id: info.image_id,
                size: info.size,
            }),
            Err(e) => Ok(VerifyResult {
                valid: false,
                error: Some(e.to_api_error()),
                image_id: None,
                size: None,
            }),
        }
    }

    pub async fn submit_job(
        &self,
        package: Package,
        request: WorkerRequest,
        output: OutputConfig,
    ) -> Job {
        let job = self.store.create_job(package, request, output);
        let job_id = job.id;

        // Spawn background task to execute the job
        let executor = self.clone_for_task();
        tokio::spawn(async move {
            executor.execute_job(job_id).await;
        });

        job
    }

    pub fn get_job(&self, id: Uuid) -> Result<Job> {
        self.store.get_job(id)
    }

    pub fn list_jobs(&self) -> Vec<Job> {
        self.store.list_jobs()
    }

    fn clone_for_task(&self) -> ExecutorTask {
        ExecutorTask {
            docker: DockerClient::new().expect("Failed to create Docker client"),
            store: self.store.clone(),
            semaphore: self.semaphore.clone(),
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new().expect("Failed to create Executor")
    }
}

// Clone of Executor for background tasks (JobStore uses DashMap which is thread-safe)
struct ExecutorTask {
    docker: DockerClient,
    store: JobStore,
    semaphore: Arc<Semaphore>,
}

impl ExecutorTask {
    async fn execute_job(&self, job_id: Uuid) {
        let _permit = match self.semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => {
                let _ = self.store.set_error(
                    job_id,
                    ApiError::with_details(ErrorCode::Internal, "semaphore closed"),
                );
                return;
            }
        };

        let job = match self.store.get_job(job_id) {
            Ok(job) => job,
            Err(e) => {
                error!(job_id = %job_id, error = %e, "Failed to get job");
                return;
            }
        };

        info!(job_id = %job_id, "Starting job execution");

        // Pull image
        let _ = self.store.update_status(job_id, JobStatus::Pulling);
        if let Err(e) = self.docker.pull_image(&job.package).await {
            error!(job_id = %job_id, error = %e, "Failed to pull image");
            let _ = self.store.set_error(job_id, e.to_api_error());
            return;
        }

        // Create container
        let _ = self.store.update_status(job_id, JobStatus::Running);
        let container = match self
            .docker
            .create_container(&job.package, &job_id.to_string(), &job.request)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                error!(job_id = %job_id, error = %e, "Failed to create container");
                let _ = self.store.set_error(job_id, e.to_api_error());
                return;
            }
        };

        let _ = self.store.set_container_id(job_id, container.id.clone());

        // Start container
        if let Err(e) = self.docker.start_container(&container.id).await {
            error!(job_id = %job_id, error = %e, "Failed to start container");
            let _ = self.docker.remove_container(&container.id).await;
            self.docker.cleanup_job_dir(&job_id.to_string()).await;
            let _ = self.store.set_error(job_id, e.to_api_error());
            return;
        }

        // Wait for container to exit
        let exit_code = match self.docker.wait_container(&container.id).await {
            Ok(code) => code,
            Err(e) => {
                error!(job_id = %job_id, error = %e, "Failed waiting for container");
                let _ = self.docker.remove_container(&container.id).await;
                self.docker.cleanup_job_dir(&job_id.to_string()).await;
                let _ = self.store.set_error(job_id, e.to_api_error());
                return;
            }
        };

        // Read response from output directory
        let response = match self.docker.read_response(&container.output_dir).await {
            Ok(r) => r,
            Err(e) => {
                error!(job_id = %job_id, exit_code = exit_code, error = %e, "Failed to read worker response");
                let _ = self.docker.remove_container(&container.id).await;
                self.docker.cleanup_job_dir(&job_id.to_string()).await;
                let _ = self.store.set_error(job_id, e.to_api_error());
                return;
            }
        };

        // Clean up container and job directory
        if let Err(e) = self.docker.remove_container(&container.id).await {
            warn!(job_id = %job_id, error = %e, "Failed to remove container");
        }
        self.docker.cleanup_job_dir(&job_id.to_string()).await;

        // Check if worker reported failure
        if !response.success {
            let details = response
                .error
                .map(|e| format!("{}: {}", e.code, e.details.unwrap_or_default()))
                .unwrap_or_else(|| "unknown error".into());
            error!(job_id = %job_id, details = %details, "Worker execution failed");
            let _ = self.store.set_error(
                job_id,
                ApiError::with_details(ErrorCode::ExecutionFailed, details),
            );
            return;
        }

        // Process output files
        let _ = self
            .store
            .update_status(job_id, JobStatus::ProcessingOutput);
        if !response.files.is_empty() {
            if let Err(e) = handle_output(&job.output, &response.files).await {
                error!(job_id = %job_id, error = %e, "Failed to handle output");
                let _ = self.store.set_error(job_id, e.to_api_error());
                return;
            }
        }

        // Store result
        if let Some(data) = response.data {
            let _ = self.store.set_result(job_id, data);
        }
        let _ = self.store.update_status(job_id, JobStatus::Completed);

        info!(job_id = %job_id, "Job completed successfully");
    }
}
