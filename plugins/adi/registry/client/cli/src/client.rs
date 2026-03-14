use std::path::PathBuf;

use futures::StreamExt;
use reqwest::Client;

use crate::cache::RegistryCache;
use crate::error::RegistryError;
use adi_registry_core_cli::{CliPluginEntry, CliPluginInfo, CliRegistryIndex, CliSearchResults};

pub struct CliRegistryClient {
    base_url: String,
    http: Client,
    cache: Option<RegistryCache>,
}

impl CliRegistryClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: Client::builder()
                .user_agent("adi-cli-registry-client")
                .build()
                .expect("Failed to create HTTP client"),
            cache: None,
        }
    }

    pub fn with_cache(mut self, cache_dir: PathBuf) -> Self {
        self.cache = Some(RegistryCache::new(cache_dir));
        self
    }

    pub fn with_cache_instance(mut self, cache: RegistryCache) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn cache(&self) -> Option<&RegistryCache> {
        self.cache.as_ref()
    }

    pub async fn fetch_index(&self) -> Result<CliRegistryIndex, RegistryError> {
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
            return Err(RegistryError::InvalidResponse(format!("Index fetch failed: {}", response.status())));
        }

        let index: CliRegistryIndex = response.json().await?;

        if let Some(cache) = &self.cache {
            let _ = cache.save_index(&index);
        }

        Ok(index)
    }

    pub async fn search(&self, query: &str) -> Result<CliSearchResults, RegistryError> {
        let url = format!("{}/v1/search?q={}", self.base_url, urlencoding::encode(query));
        let response = self.http.get(&url).send().await?;

        if !response.status().is_success() {
            return self.search_local(query).await;
        }

        let results: CliSearchResults = response.json().await?;
        Ok(results)
    }

    async fn search_local(&self, query: &str) -> Result<CliSearchResults, RegistryError> {
        let index = self.fetch_index().await?;
        let query_lower = query.to_lowercase();

        let plugins = index
            .plugins
            .into_iter()
            .filter(|p| {
                p.id.to_lowercase().contains(&query_lower)
                    || p.name.to_lowercase().contains(&query_lower)
                    || p.description.to_lowercase().contains(&query_lower)
            })
            .collect();

        Ok(CliSearchResults { plugins })
    }

    pub async fn get_plugin_version(&self, id: &str, version: &str) -> Result<CliPluginInfo, RegistryError> {
        let url = format!("{}/v1/{}/{}", self.base_url, id, version);
        let response = self.http.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(RegistryError::VersionNotFound(id.to_string(), version.to_string()));
        }
        if !response.status().is_success() {
            return Err(RegistryError::InvalidResponse(format!("Fetch failed: {}", response.status())));
        }

        Ok(response.json().await?)
    }

    pub async fn get_plugin_latest(&self, id: &str) -> Result<CliPluginInfo, RegistryError> {
        let url = format!("{}/v1/{}/latest", self.base_url, id);
        let response = self.http.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(RegistryError::NotFound(id.to_string()));
        }
        if !response.status().is_success() {
            return Err(RegistryError::InvalidResponse(format!("Fetch failed: {}", response.status())));
        }

        Ok(response.json().await?)
    }

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
        if let Some(cache) = &self.cache {
            if let Ok(Some(data)) = cache.load_download(id, version, platform) {
                let len = data.len() as u64;
                progress(len, len);
                return Ok(data);
            }
        }

        let url = format!("{}/v1/{}/{}/{}.tar.gz", self.base_url, id, version, platform);
        let response = self.http.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(RegistryError::PlatformNotSupported(platform.to_string()));
        }
        if !response.status().is_success() {
            return Err(RegistryError::InvalidResponse(format!("Download failed: {}", response.status())));
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

        if let Some(cache) = &self.cache {
            let _ = cache.save_download(id, version, platform, &bytes);
        }

        Ok(bytes)
    }

    pub async fn list_plugin_versions(&self, id: &str) -> Result<Vec<String>, RegistryError> {
        let url = format!("{}/v1/{}/versions", self.base_url, id);
        let response = self.http.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(RegistryError::NotFound(id.to_string()));
        }
        if !response.status().is_success() {
            return Err(RegistryError::InvalidResponse(format!("Version listing failed: {}", response.status())));
        }

        #[derive(serde::Deserialize)]
        struct VersionsResponse { versions: Vec<String> }
        let resp: VersionsResponse = response.json().await?;
        Ok(resp.versions)
    }

    pub async fn list_plugins(&self) -> Result<Vec<CliPluginEntry>, RegistryError> {
        let index = self.fetch_index().await?;
        Ok(index.plugins)
    }

    pub fn clear_cache(&self) -> Result<(), RegistryError> {
        if let Some(cache) = &self.cache {
            cache.clear()?;
        }
        Ok(())
    }
}
