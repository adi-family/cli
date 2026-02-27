use std::path::{Path, PathBuf};

use crate::Result;
use uuid::Uuid;

/// Disk-based storage for captured frames.
pub struct FrameStore {
    base_dir: PathBuf,
}

impl FrameStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub fn job_dir(&self, job_id: Uuid) -> PathBuf {
        self.base_dir.join(job_id.to_string())
    }

    pub fn frame_path(&self, job_id: Uuid, index: u32) -> PathBuf {
        self.job_dir(job_id).join(format!("frame_{index:06}.jpg"))
    }

    pub fn output_path(&self, job_id: Uuid, extension: &str) -> PathBuf {
        self.job_dir(job_id).join(format!("output.{extension}"))
    }

    pub fn ensure_job_dir(&self, job_id: Uuid) -> Result<PathBuf> {
        let dir = self.job_dir(job_id);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn save_frame(&self, job_id: Uuid, index: u32, data: &[u8]) -> Result<PathBuf> {
        self.ensure_job_dir(job_id)?;
        let path = self.frame_path(job_id, index);
        std::fs::write(&path, data)?;
        Ok(path)
    }

    pub fn cleanup(&self, job_id: Uuid) -> Result<()> {
        let dir = self.job_dir(job_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}
