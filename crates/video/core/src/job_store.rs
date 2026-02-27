use dashmap::DashMap;
use uuid::Uuid;

use crate::{RenderConfig, RenderJob, RenderPhase, Result, VideoError};

/// In-memory store for render jobs.
pub struct JobStore {
    jobs: DashMap<Uuid, RenderJob>,
}

impl Default for JobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl JobStore {
    pub fn new() -> Self {
        Self {
            jobs: DashMap::new(),
        }
    }

    pub fn create(&self, config: RenderConfig) -> RenderJob {
        let job = RenderJob::new(config);
        self.jobs.insert(job.id, job.clone());
        job
    }

    pub fn get(&self, id: Uuid) -> Result<RenderJob> {
        self.jobs
            .get(&id)
            .map(|r| r.clone())
            .ok_or(VideoError::JobNotFound(id))
    }

    pub fn update<F>(&self, id: Uuid, f: F) -> Result<RenderJob>
    where
        F: FnOnce(&mut RenderJob),
    {
        let mut entry = self
            .jobs
            .get_mut(&id)
            .ok_or(VideoError::JobNotFound(id))?;
        f(&mut entry);
        entry.updated_at = chrono::Utc::now();
        Ok(entry.clone())
    }

    pub fn list(&self) -> Vec<RenderJob> {
        self.jobs.iter().map(|r| r.clone()).collect()
    }

    pub fn increment_frames(&self, id: Uuid) -> Result<u32> {
        let mut entry = self
            .jobs
            .get_mut(&id)
            .ok_or(VideoError::JobNotFound(id))?;
        entry.frames_received += 1;
        entry.phase = RenderPhase::Capturing;
        entry.progress = entry.frames_received as f64 / entry.config.total_frames as f64;
        entry.updated_at = chrono::Utc::now();
        Ok(entry.frames_received)
    }
}
