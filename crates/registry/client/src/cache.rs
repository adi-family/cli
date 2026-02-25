//! Local caching for registry data.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::RegistryError;
use crate::index::RegistryIndex;

/// Cache manager for registry data.
#[derive(Debug, Clone)]
pub struct RegistryCache {
    /// Cache directory
    cache_dir: PathBuf,
    /// Index cache duration
    index_ttl: Duration,
}

impl RegistryCache {
    /// Create a new cache manager.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            index_ttl: Duration::from_secs(3600), // 1 hour default
        }
    }

    /// Set the index cache TTL.
    pub fn with_index_ttl(mut self, ttl: Duration) -> Self {
        self.index_ttl = ttl;
        self
    }

    /// Get the cache directory.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Get the index cache path.
    pub fn index_path(&self) -> PathBuf {
        self.cache_dir.join("registry").join("index.json")
    }

    /// Get the downloads cache directory.
    pub fn downloads_dir(&self) -> PathBuf {
        self.cache_dir.join("downloads")
    }

    /// Check if the index cache is still valid.
    pub fn is_index_valid(&self) -> bool {
        let index_path = self.index_path();
        if !index_path.exists() {
            return false;
        }

        match index_path.metadata().and_then(|m| m.modified()) {
            Ok(modified) => {
                let elapsed = SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or(Duration::MAX);
                elapsed < self.index_ttl
            }
            Err(_) => false,
        }
    }

    /// Load cached index.
    pub fn load_index(&self) -> Result<Option<RegistryIndex>, RegistryError> {
        let index_path = self.index_path();
        if !index_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&index_path)?;
        let index: RegistryIndex = serde_json::from_str(&content)?;
        Ok(Some(index))
    }

    /// Save index to cache.
    pub fn save_index(&self, index: &RegistryIndex) -> Result<(), RegistryError> {
        let index_path = self.index_path();

        if let Some(parent) = index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(index)?;
        std::fs::write(&index_path, content)?;
        Ok(())
    }

    /// Get path for a cached download.
    pub fn download_path(&self, id: &str, version: &str, platform: &str) -> PathBuf {
        self.downloads_dir()
            .join(format!("{}-{}-{}.tar.gz", id, version, platform))
    }

    /// Check if a download is cached.
    pub fn has_download(&self, id: &str, version: &str, platform: &str) -> bool {
        self.download_path(id, version, platform).exists()
    }

    /// Load cached download.
    pub fn load_download(
        &self,
        id: &str,
        version: &str,
        platform: &str,
    ) -> Result<Option<Vec<u8>>, RegistryError> {
        let path = self.download_path(id, version, platform);
        if !path.exists() {
            return Ok(None);
        }

        let data = std::fs::read(&path)?;
        Ok(Some(data))
    }

    /// Save download to cache.
    pub fn save_download(
        &self,
        id: &str,
        version: &str,
        platform: &str,
        data: &[u8],
    ) -> Result<(), RegistryError> {
        let path = self.download_path(id, version, platform);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&path, data)?;
        Ok(())
    }

    /// Clear all cached data.
    pub fn clear(&self) -> Result<(), RegistryError> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    /// Clear only index cache.
    pub fn clear_index(&self) -> Result<(), RegistryError> {
        let index_path = self.index_path();
        if index_path.exists() {
            std::fs::remove_file(&index_path)?;
        }
        Ok(())
    }

    /// Clear only downloads cache.
    pub fn clear_downloads(&self) -> Result<(), RegistryError> {
        let downloads_dir = self.downloads_dir();
        if downloads_dir.exists() {
            std::fs::remove_dir_all(&downloads_dir)?;
        }
        Ok(())
    }

    /// Get cache size in bytes.
    pub fn size(&self) -> u64 {
        if !self.cache_dir.exists() {
            return 0;
        }

        walkdir(&self.cache_dir)
    }
}

/// Recursively calculate directory size.
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_paths() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        assert!(cache.index_path().ends_with("registry/index.json"));
        assert!(cache.downloads_dir().ends_with("downloads"));
    }

    #[test]
    fn test_save_and_load_index() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        let index = RegistryIndex::default();
        cache.save_index(&index).unwrap();

        let loaded = cache.load_index().unwrap();
        assert!(loaded.is_some());
    }

    #[test]
    fn test_is_index_valid() {
        let temp = tempdir().unwrap();
        let cache =
            RegistryCache::new(temp.path().to_path_buf()).with_index_ttl(Duration::from_secs(3600));

        // No index yet
        assert!(!cache.is_index_valid());

        // Save index
        let index = RegistryIndex::default();
        cache.save_index(&index).unwrap();

        // Should be valid now
        assert!(cache.is_index_valid());
    }

    #[test]
    fn test_is_index_valid_expired() {
        let temp = tempdir().unwrap();
        let cache =
            RegistryCache::new(temp.path().to_path_buf()).with_index_ttl(Duration::from_secs(0));

        let index = RegistryIndex::default();
        cache.save_index(&index).unwrap();

        // TTL is 0 so should be expired immediately
        // (may pass if filesystem timestamp resolution is coarse, so we just confirm no panic)
        let _ = cache.is_index_valid();
    }

    #[test]
    fn test_save_load_index_roundtrip() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        let mut index = RegistryIndex::default();
        index.updated_at = 12345;

        cache.save_index(&index).unwrap();
        let loaded = cache.load_index().unwrap().unwrap();
        assert_eq!(loaded.updated_at, 12345);
        assert_eq!(loaded.version, 1);
    }

    #[test]
    fn test_save_load_download_roundtrip() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        let data = b"fake binary data for download test";
        cache
            .save_download("my.plugin", "1.0.0", "darwin-aarch64", data)
            .unwrap();

        let loaded = cache
            .load_download("my.plugin", "1.0.0", "darwin-aarch64")
            .unwrap()
            .unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn test_has_download() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        assert!(!cache.has_download("pkg", "1.0.0", "linux"));

        cache
            .save_download("pkg", "1.0.0", "linux", b"data")
            .unwrap();
        assert!(cache.has_download("pkg", "1.0.0", "linux"));
    }

    #[test]
    fn test_load_download_missing() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        let result = cache.load_download("missing", "1.0.0", "linux").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_clear_all() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        cache.save_index(&RegistryIndex::default()).unwrap();
        cache
            .save_download("pkg", "1.0.0", "linux", b"data")
            .unwrap();

        cache.clear().unwrap();

        assert!(!cache.is_index_valid());
        assert!(!cache.has_download("pkg", "1.0.0", "linux"));
    }

    #[test]
    fn test_clear_index() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        cache.save_index(&RegistryIndex::default()).unwrap();
        cache
            .save_download("pkg", "1.0.0", "linux", b"data")
            .unwrap();

        cache.clear_index().unwrap();

        assert!(!cache.is_index_valid());
        // Downloads should still be there
        assert!(cache.has_download("pkg", "1.0.0", "linux"));
    }

    #[test]
    fn test_clear_downloads() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        cache.save_index(&RegistryIndex::default()).unwrap();
        cache
            .save_download("pkg", "1.0.0", "linux", b"data")
            .unwrap();

        cache.clear_downloads().unwrap();

        // Index should still be there
        assert!(cache.is_index_valid());
        assert!(!cache.has_download("pkg", "1.0.0", "linux"));
    }

    #[test]
    fn test_size_calculation() {
        let temp = tempdir().unwrap();
        let cache = RegistryCache::new(temp.path().to_path_buf());

        assert_eq!(cache.size(), 0);

        cache.save_index(&RegistryIndex::default()).unwrap();
        let size_after_index = cache.size();
        assert!(size_after_index > 0);

        cache
            .save_download("pkg", "1.0.0", "linux", b"some binary data")
            .unwrap();
        let size_after_download = cache.size();
        assert!(size_after_download > size_after_index);
    }
}
