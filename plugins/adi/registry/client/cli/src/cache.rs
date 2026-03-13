use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::RegistryError;
use adi_registry_core_cli::CliRegistryIndex;

#[derive(Debug, Clone)]
pub struct RegistryCache {
    cache_dir: PathBuf,
    index_ttl: Duration,
}

impl RegistryCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir, index_ttl: Duration::from_secs(3600) }
    }

    pub fn with_index_ttl(mut self, ttl: Duration) -> Self {
        self.index_ttl = ttl;
        self
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn index_path(&self) -> PathBuf {
        self.cache_dir.join("cli-registry").join("index.json")
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.cache_dir.join("downloads")
    }

    pub fn is_index_valid(&self) -> bool {
        let index_path = self.index_path();
        if !index_path.exists() { return false; }
        match index_path.metadata().and_then(|m| m.modified()) {
            Ok(modified) => {
                let elapsed = SystemTime::now().duration_since(modified).unwrap_or(Duration::MAX);
                elapsed < self.index_ttl
            }
            Err(_) => false,
        }
    }

    pub fn load_index(&self) -> Result<Option<CliRegistryIndex>, RegistryError> {
        let index_path = self.index_path();
        if !index_path.exists() { return Ok(None); }
        let content = std::fs::read_to_string(&index_path)?;
        let index: CliRegistryIndex = serde_json::from_str(&content)?;
        Ok(Some(index))
    }

    pub fn save_index(&self, index: &CliRegistryIndex) -> Result<(), RegistryError> {
        let index_path = self.index_path();
        if let Some(parent) = index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(index)?;
        std::fs::write(&index_path, content)?;
        Ok(())
    }

    pub fn download_path(&self, id: &str, version: &str, platform: &str) -> PathBuf {
        self.downloads_dir().join(format!("{id}-{version}-{platform}.tar.gz"))
    }

    pub fn has_download(&self, id: &str, version: &str, platform: &str) -> bool {
        self.download_path(id, version, platform).exists()
    }

    pub fn load_download(&self, id: &str, version: &str, platform: &str) -> Result<Option<Vec<u8>>, RegistryError> {
        let path = self.download_path(id, version, platform);
        if !path.exists() { return Ok(None); }
        let data = std::fs::read(&path)?;
        Ok(Some(data))
    }

    pub fn save_download(&self, id: &str, version: &str, platform: &str, data: &[u8]) -> Result<(), RegistryError> {
        let path = self.download_path(id, version, platform);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, data)?;
        Ok(())
    }

    pub fn clear(&self) -> Result<(), RegistryError> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    pub fn clear_index(&self) -> Result<(), RegistryError> {
        let index_path = self.index_path();
        if index_path.exists() { std::fs::remove_file(&index_path)?; }
        Ok(())
    }

    pub fn clear_downloads(&self) -> Result<(), RegistryError> {
        let downloads_dir = self.downloads_dir();
        if downloads_dir.exists() { std::fs::remove_dir_all(&downloads_dir)?; }
        Ok(())
    }

    pub fn size(&self) -> u64 {
        if !self.cache_dir.exists() { return 0; }
        walkdir(&self.cache_dir)
    }
}

fn walkdir(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                size += path.metadata().map(|m| m.len()).unwrap_or(0);
            } else if path.is_dir() {
                size += walkdir(&path);
            }
        }
    }
    size
}
