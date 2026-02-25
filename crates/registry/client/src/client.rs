//! Registry HTTP client.

use std::path::PathBuf;

use futures::StreamExt;
use reqwest::Client;

use crate::cache::RegistryCache;
use crate::error::RegistryError;
use crate::index::{
    PackageEntry, PackageInfo, PluginEntry, PluginInfo, RegistryIndex, SearchKind, SearchResults,
};

/// HTTP client for plugin registry.
pub struct RegistryClient {
    /// Base URL of the registry
    base_url: String,
    /// HTTP client
    http: Client,
    /// Local cache (optional)
    cache: Option<RegistryCache>,
}

impl RegistryClient {
    /// Create a new registry client.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: Client::builder()
                .user_agent("lib-plugin-registry")
                .build()
                .expect("Failed to create HTTP client"),
            cache: None,
        }
    }

    /// Enable caching with the given directory.
    pub fn with_cache(mut self, cache_dir: PathBuf) -> Self {
        self.cache = Some(RegistryCache::new(cache_dir));
        self
    }

    /// Enable caching with a custom cache instance.
    pub fn with_cache_instance(mut self, cache: RegistryCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the cache instance.
    pub fn cache(&self) -> Option<&RegistryCache> {
        self.cache.as_ref()
    }

    // === Index Operations ===

    /// Fetch the full registry index. Uses cache if available and not expired.
    pub async fn fetch_index(&self) -> Result<RegistryIndex, RegistryError> {
        if let Some(cache) = &self.cache {
            if cache.is_index_valid() {
                if let Ok(Some(index)) = cache.load_index() {
                    return Ok(index);
                }
            }
        }

        let url = format!("{}/v1/index", self.base_url);
        let response = self.http.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(RegistryError::InvalidResponse(format!(
                "Index fetch failed: {}",
                response.status()
            )));
        }

        let index: RegistryIndex = response.json().await?;

        if let Some(cache) = &self.cache {
            let _ = cache.save_index(&index);
        }

        Ok(index)
    }

    // === Search Operations ===

    /// Search for packages and plugins.
    pub async fn search(
        &self,
        query: &str,
        kind: SearchKind,
    ) -> Result<SearchResults, RegistryError> {
        let url = format!(
            "{}/v1/search?q={}&kind={}",
            self.base_url,
            urlencoding::encode(query),
            kind.as_str()
        );

        let response = self.http.get(&url).send().await?;

        if !response.status().is_success() {
            return self.search_local(query, kind).await;
        }

        let results: SearchResults = response.json().await?;
        Ok(results)
    }

    /// Search locally using cached index.
    async fn search_local(
        &self,
        query: &str,
        kind: SearchKind,
    ) -> Result<SearchResults, RegistryError> {
        let index = self.fetch_index().await?;
        let query_lower = query.to_lowercase();

        let mut results = SearchResults::default();

        if !matches!(kind, SearchKind::PluginsOnly) {
            results.packages = index
                .packages
                .into_iter()
                .filter(|p| {
                    p.id.to_lowercase().contains(&query_lower)
                        || p.name.to_lowercase().contains(&query_lower)
                        || p.description.to_lowercase().contains(&query_lower)
                })
                .collect();
        }

        if !matches!(kind, SearchKind::PackagesOnly) {
            results.plugins = index
                .plugins
                .into_iter()
                .filter(|p| {
                    p.id.to_lowercase().contains(&query_lower)
                        || p.name.to_lowercase().contains(&query_lower)
                        || p.description.to_lowercase().contains(&query_lower)
                })
                .collect();
        }

        Ok(results)
    }

    // === Shared version info / download helpers ===

    /// Fetch version info for a given segment ("packages" or "plugins").
    async fn get_version_info<T: serde::de::DeserializeOwned>(
        &self,
        segment: &str,
        id: &str,
        version: &str,
    ) -> Result<T, RegistryError> {
        let url = format!("{}/v1/{}/{}/{}.json", self.base_url, segment, id, version);
        let response = self.http.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(RegistryError::VersionNotFound(
                id.to_string(),
                version.to_string(),
            ));
        }

        if !response.status().is_success() {
            return Err(RegistryError::InvalidResponse(format!(
                "Fetch failed: {}",
                response.status()
            )));
        }

        let info: T = response.json().await?;
        Ok(info)
    }

    /// Fetch latest version info for a given segment.
    async fn get_latest_info<T: serde::de::DeserializeOwned>(
        &self,
        segment: &str,
        id: &str,
    ) -> Result<T, RegistryError> {
        let url = format!("{}/v1/{}/{}/latest", self.base_url, segment, id);
        let response = self.http.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(RegistryError::NotFound(id.to_string()));
        }

        if !response.status().is_success() {
            return Err(RegistryError::InvalidResponse(format!(
                "Fetch failed: {}",
                response.status()
            )));
        }

        let info: T = response.json().await?;
        Ok(info)
    }

    /// Download an artifact (package or plugin) with progress reporting.
    async fn download_artifact<F>(
        &self,
        segment: &str,
        id: &str,
        version: &str,
        platform: &str,
        progress: F,
    ) -> Result<Vec<u8>, RegistryError>
    where
        F: Fn(u64, u64),
    {
        // Check cache first
        if let Some(cache) = &self.cache {
            if let Ok(Some(data)) = cache.load_download(id, version, platform) {
                let len = data.len() as u64;
                progress(len, len);
                return Ok(data);
            }
        }

        let url = format!(
            "{}/v1/{}/{}/{}/{}.tar.gz",
            self.base_url, segment, id, version, platform
        );

        let response = self.http.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(RegistryError::PlatformNotSupported(platform.to_string()));
        }

        if !response.status().is_success() {
            return Err(RegistryError::InvalidResponse(format!(
                "Download failed: {}",
                response.status()
            )));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut bytes = Vec::new();
        let mut downloaded = 0u64;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            bytes.extend_from_slice(&chunk);
            progress(downloaded, total_size);
        }

        // Save to cache
        if let Some(cache) = &self.cache {
            let _ = cache.save_download(id, version, platform, &bytes);
        }

        Ok(bytes)
    }

    // === Package Operations ===

    /// Get package version information.
    pub async fn get_package_version(
        &self,
        id: &str,
        version: &str,
    ) -> Result<PackageInfo, RegistryError> {
        self.get_version_info("packages", id, version).await
    }

    /// Get latest package version info.
    pub async fn get_package_latest(&self, id: &str) -> Result<PackageInfo, RegistryError> {
        self.get_latest_info("packages", id).await
    }

    /// Download a package. Progress callback receives (bytes_downloaded, total_bytes).
    pub async fn download_package<F>(
        &self,
        id: &str,
        version: &str,
        platform: &str,
        progress: F,
    ) -> Result<Vec<u8>, RegistryError>
    where
        F: Fn(u64, u64),
    {
        self.download_artifact("packages", id, version, platform, progress)
            .await
    }

    // === Plugin Operations ===

    /// Get plugin version information.
    pub async fn get_plugin_version(
        &self,
        id: &str,
        version: &str,
    ) -> Result<PluginInfo, RegistryError> {
        self.get_version_info("plugins", id, version).await
    }

    /// Get latest plugin version info.
    pub async fn get_plugin_latest(&self, id: &str) -> Result<PluginInfo, RegistryError> {
        self.get_latest_info("plugins", id).await
    }

    /// Download a plugin.
    pub async fn download_plugin<F>(
        &self,
        id: &str,
        version: &str,
        platform: &str,
        progress: F,
    ) -> Result<Vec<u8>, RegistryError>
    where
        F: Fn(u64, u64),
    {
        self.download_artifact("plugins", id, version, platform, progress)
            .await
    }

    // === Version Listing ===

    /// List all published versions for a plugin.
    pub async fn list_plugin_versions(&self, id: &str) -> Result<Vec<String>, RegistryError> {
        self.list_versions("plugins", id).await
    }

    /// List all published versions for a package.
    pub async fn list_package_versions(&self, id: &str) -> Result<Vec<String>, RegistryError> {
        self.list_versions("packages", id).await
    }

    async fn list_versions(
        &self,
        segment: &str,
        id: &str,
    ) -> Result<Vec<String>, RegistryError> {
        let url = format!("{}/v1/{}/{}/versions", self.base_url, segment, id);
        let response = self.http.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(RegistryError::NotFound(id.to_string()));
        }

        if !response.status().is_success() {
            return Err(RegistryError::InvalidResponse(format!(
                "Version listing failed: {}",
                response.status()
            )));
        }

        #[derive(serde::Deserialize)]
        struct VersionsResponse {
            versions: Vec<String>,
        }
        let resp: VersionsResponse = response.json().await?;
        Ok(resp.versions)
    }

    // === Convenience Methods ===

    /// List all packages in the registry.
    pub async fn list_packages(&self) -> Result<Vec<PackageEntry>, RegistryError> {
        let index = self.fetch_index().await?;
        Ok(index.packages)
    }

    /// List all plugins in the registry.
    pub async fn list_plugins(&self) -> Result<Vec<PluginEntry>, RegistryError> {
        let index = self.fetch_index().await?;
        Ok(index.plugins)
    }

    /// Clear the local cache.
    pub fn clear_cache(&self) -> Result<(), RegistryError> {
        if let Some(cache) = &self.cache {
            cache.clear()?;
        }
        Ok(())
    }
}
