use anyhow::{bail, Context, Result};
use adi_registry_core_shared::archive::extract_files_from_tar_gz;
use adi_registry_core_shared::manifest::Manifest;
use adi_registry_core_shared::storage::{FileStorage, MAX_WEB_ARCHIVE_SIZE};
use adi_registry_core_shared::validation::{validate_id, validate_version};
use adi_registry_core_shared::{now_unix, semver_greater};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::types::{WebPluginEntry, WebPluginInfo, WebRegistryIndex};

const INDEX_FILE: &str = "index.json";
const PLUGINS_DIR: &str = "plugins";

pub struct WebRegistryStorage {
    storage: FileStorage,
}

impl WebRegistryStorage {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self {
            storage: FileStorage::new(root),
        }
    }

    pub fn inner(&self) -> &FileStorage {
        &self.storage
    }

    pub async fn init(&self) -> Result<()> {
        self.storage.init(&[PLUGINS_DIR]).await?;
        self.storage.ensure_index::<WebRegistryIndex>(INDEX_FILE).await?;
        Ok(())
    }

    pub async fn load_index(&self) -> Result<WebRegistryIndex> {
        self.storage.load_json(INDEX_FILE).await
    }

    pub async fn save_index(&self, index: &WebRegistryIndex) -> Result<()> {
        self.storage.save_json_atomic(INDEX_FILE, index).await
    }

    pub fn plugin_dir(&self, id: &str, version: &str) -> std::path::PathBuf {
        self.storage.artifact_version_dir(PLUGINS_DIR, id, version)
    }

    pub fn js_path(&self, id: &str, version: &str) -> std::path::PathBuf {
        self.plugin_dir(id, version).join("main.js")
    }

    pub fn css_path(&self, id: &str, version: &str) -> std::path::PathBuf {
        self.plugin_dir(id, version).join("main.css")
    }

    pub fn preview_html_path(&self, id: &str, version: &str) -> std::path::PathBuf {
        self.plugin_dir(id, version).join("preview.html")
    }

    pub fn preview_image_path(&self, id: &str, version: &str, index: u8) -> std::path::PathBuf {
        self.plugin_dir(id, version).join(format!("preview_{index}.webp"))
    }

    pub async fn get_plugin_info(&self, id: &str, version: &str) -> Result<WebPluginInfo> {
        validate_id(id)?;
        validate_version(version)?;
        let info_path = self.plugin_dir(id, version).join("info.json");
        let data = fs::read_to_string(&info_path).await?;
        serde_json::from_str(&data).context("Failed to parse info.json")
    }

    pub async fn get_plugin_latest(&self, id: &str) -> Result<WebPluginInfo> {
        validate_id(id)?;
        let index = self.load_index().await?;
        let entry = index
            .plugins
            .iter()
            .find(|p| p.id == id)
            .context("Plugin not found")?;
        self.get_plugin_info(id, &entry.latest_version).await
    }

    /// Publish a web plugin from a .tar.gz archive containing manifest.json + main.js + optional assets.
    pub async fn publish_web_plugin(&self, id: &str, version: &str, data: &[u8]) -> Result<()> {
        if data.len() > MAX_WEB_ARCHIVE_SIZE {
            bail!(
                "Archive size {} bytes exceeds maximum allowed {} bytes",
                data.len(),
                MAX_WEB_ARCHIVE_SIZE
            );
        }
        validate_id(id)?;
        validate_version(version)?;

        // Extract required files from archive
        let files = extract_files_from_tar_gz(
            data,
            &["manifest.json", "main.js", "main.css", "preview.html"],
        );

        let manifest_bytes = files
            .iter()
            .find(|(name, _)| name == "manifest.json")
            .map(|(_, data)| data.clone())
            .context("Archive must contain manifest.json")?;

        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
            .context("Failed to parse manifest.json")?;

        if manifest.id != id {
            bail!("Manifest ID '{}' does not match path ID '{id}'", manifest.id);
        }
        if manifest.version != version {
            bail!(
                "Manifest version '{}' does not match path version '{version}'",
                manifest.version
            );
        }

        let js_bytes = files
            .iter()
            .find(|(name, _)| name == "main.js")
            .map(|(_, data)| data.clone())
            .context("Archive must contain main.js")?;

        let css_bytes = files
            .iter()
            .find(|(name, _)| name == "main.css")
            .map(|(_, data)| data.clone());

        let preview_html_bytes = files
            .iter()
            .find(|(name, _)| name == "preview.html")
            .map(|(_, data)| data.clone());

        // Extract preview images (preview_0.webp through preview_9.webp)
        let preview_names: Vec<String> = (0..10)
            .map(|i| format!("preview_{i}.webp"))
            .collect();
        let preview_name_refs: Vec<&str> = preview_names.iter().map(|s| s.as_str()).collect();
        let preview_files = extract_files_from_tar_gz(data, &preview_name_refs);

        // Create version directory
        let version_dir = self.plugin_dir(id, version);
        fs::create_dir_all(&version_dir).await?;

        // Write main.js
        let mut js_file = fs::File::create(self.js_path(id, version)).await?;
        js_file.write_all(&js_bytes).await?;

        // Write main.css if present
        let css_url = if let Some(css_data) = &css_bytes {
            let mut css_file = fs::File::create(self.css_path(id, version)).await?;
            css_file.write_all(css_data).await?;
            Some(format!("/v1/{id}/{version}/main.css"))
        } else {
            None
        };

        // Write preview.html if present
        let preview_url = if let Some(preview_data) = &preview_html_bytes {
            let mut preview_file = fs::File::create(self.preview_html_path(id, version)).await?;
            preview_file.write_all(preview_data).await?;
            Some(format!("/v1/{id}/{version}/preview.html"))
        } else {
            None
        };

        // Write preview images
        let mut preview_images = Vec::new();
        for (name, img_data) in &preview_files {
            let index: u8 = name
                .strip_prefix("preview_")
                .and_then(|s| s.strip_suffix(".webp"))
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let img_path = self.preview_image_path(id, version, index);
            let mut img_file = fs::File::create(&img_path).await?;
            img_file.write_all(img_data).await?;
            preview_images.push(format!("/v1/{id}/{version}/preview/{index}.webp"));
        }

        // Create info.json
        let info = WebPluginInfo {
            id: id.to_string(),
            version: version.to_string(),
            js_url: format!("/v1/{id}/{version}/main.js"),
            css_url,
            size_bytes: js_bytes.len() as u64,
            published_at: now_unix(),
            changelog: None,
            preview_url,
            preview_images,
        };

        let info_path = version_dir.join("info.json");
        let tmp_path = version_dir.join("info.json.tmp");
        let json = serde_json::to_string_pretty(&info)?;
        fs::write(&tmp_path, json).await?;
        fs::rename(&tmp_path, &info_path).await?;

        // Update index
        let mut index = self.load_index().await?;
        if let Some(entry) = index.plugins.iter_mut().find(|p| p.id == id) {
            if semver_greater(version, &entry.latest_version) {
                entry.latest_version = version.to_string();
            }
            entry.name = manifest.name;
            entry.description = manifest.description;
            entry.author = manifest.author;
            entry.tags = manifest.tags;
        } else {
            index.plugins.push(WebPluginEntry {
                id: id.to_string(),
                name: manifest.name,
                description: manifest.description,
                latest_version: version.to_string(),
                downloads: 0,
                author: manifest.author,
                tags: manifest.tags,
            });
        }
        index.updated_at = now_unix();
        self.save_index(&index).await
    }

    pub async fn list_plugin_versions(&self, id: &str) -> Result<Vec<String>> {
        self.storage.list_artifact_versions(PLUGINS_DIR, id).await
    }

    pub async fn increment_downloads(&self, id: &str) -> Result<()> {
        let mut index = self.load_index().await?;
        if let Some(entry) = index.plugins.iter_mut().find(|p| p.id == id) {
            entry.downloads += 1;
        }
        self.save_index(&index).await
    }
}
