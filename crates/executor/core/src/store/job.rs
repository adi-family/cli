use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ExecutorError, Result};
use crate::types::{Job, JobStatus, OutputConfig, Package, WorkerRequest};

#[derive(Clone)]
pub struct JobStore {
    jobs: Arc<DashMap<Uuid, Job>>,
}

impl JobStore {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(DashMap::new()),
        }
    }

    pub fn create_job(
        &self,
        package: Package,
        request: WorkerRequest,
        output: OutputConfig,
    ) -> Job {
        let job = Job::new(package, request, output);
        let id = job.id;
        self.jobs.insert(id, job.clone());
        job
    }

    pub fn get_job(&self, id: Uuid) -> Result<Job> {
        self.jobs
            .get(&id)
            .map(|j| j.clone())
            .ok_or(ExecutorError::JobNotFound(id))
    }

    pub fn update_status(&self, id: Uuid, status: JobStatus) -> Result<()> {
        let mut job = self
            .jobs
            .get_mut(&id)
            .ok_or(ExecutorError::JobNotFound(id))?;

        job.status = status;

        match status {
            JobStatus::Running => {
                job.started_at = Some(Utc::now());
            }
            JobStatus::Completed | JobStatus::Failed => {
                job.completed_at = Some(Utc::now());
            }
            _ => {}
        }

        Ok(())
    }

    pub fn set_container_id(&self, id: Uuid, container_id: String) -> Result<()> {
        let mut job = self
            .jobs
            .get_mut(&id)
            .ok_or(ExecutorError::JobNotFound(id))?;
        job.container_id = Some(container_id);
        Ok(())
    }

    pub fn set_result(&self, id: Uuid, result: serde_json::Value) -> Result<()> {
        let mut job = self
            .jobs
            .get_mut(&id)
            .ok_or(ExecutorError::JobNotFound(id))?;
        job.result = Some(result);
        Ok(())
    }

    pub fn set_error(&self, id: Uuid, error: ApiError) -> Result<()> {
        let mut job = self
            .jobs
            .get_mut(&id)
            .ok_or(ExecutorError::JobNotFound(id))?;
        job.error = Some(error);
        job.status = JobStatus::Failed;
        job.completed_at = Some(Utc::now());
        Ok(())
    }

    pub fn list_jobs(&self) -> Vec<Job> {
        self.jobs
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn cleanup_old_jobs(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        self.jobs.retain(|_, job| {
            job.completed_at.map(|t| t > cutoff).unwrap_or(true) // Keep running jobs
        });
    }
}

impl Default for JobStore {
    fn default() -> Self {
        Self::new()
    }
}
