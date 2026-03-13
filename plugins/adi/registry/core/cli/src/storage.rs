use anyhow::{Context, Result};
use adi_registry_core_shared::storage::{FileStorage, PublishRequest, MAX_CLI_ARCHIVE_SIZE};
use adi_registry_core_shared::{now_unix, semver_greater};

use crate::types::{CliPluginEntry, CliPluginInfo, CliRegistryIndex};
use crate::validate_platform;

const INDEX_FILE: &str = "index.json";
const PLUGINS_DIR: &str = "plugins";

pub struct CliRegistryStorage {
    storage: FileStorage,
}

impl CliRegistryStorage {
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
        self.storage.ensure_index::<CliRegistryIndex>(INDEX_FILE).await?;
        Ok(())
    }

    pub async fn load_index(&self) -> Result<CliRegistryIndex> {
        self.storage.load_json(INDEX_FILE).await
    }

    pub async fn save_index(&self, index: &CliRegistryIndex) -> Result<()> {
        self.storage.save_json_atomic(INDEX_FILE, index).await
    }

    pub fn artifact_path(&self, id: &str, version: &str, platform: &str) -> std::path::PathBuf {
        self.storage.artifact_path(PLUGINS_DIR, id, version, platform)
    }

    pub async fn get_plugin_info(&self, id: &str, version: &str) -> Result<CliPluginInfo> {
        self.storage.get_artifact_info(PLUGINS_DIR, id, version).await
    }

    pub async fn get_plugin_latest(&self, id: &str) -> Result<CliPluginInfo> {
        adi_registry_core_shared::validation::validate_id(id)?;
        let index = self.load_index().await?;
        let entry = index
            .plugins
            .iter()
            .find(|p| p.id == id)
            .context("Plugin not found")?;
        self.get_plugin_info(id, &entry.latest_version).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn publish_plugin(
        &self,
        id: &str,
        name: &str,
        description: &str,
        version: &str,
        platform: &str,
        data: &[u8],
        author: &str,
        tags: Vec<String>,
        publisher_signature: Option<&str>,
        publisher_public_key: Option<&str>,
        publisher_id: Option<&str>,
        publisher_certificate: Option<&str>,
    ) -> Result<()> {
        let req = PublishRequest {
            id,
            name,
            description,
            version,
            platform,
            data,
            author,
            tags: tags.clone(),
            publisher_signature,
            publisher_public_key,
            publisher_id,
            publisher_certificate,
        };

        self.storage
            .publish_artifact(PLUGINS_DIR, &req, Some(validate_platform), MAX_CLI_ARCHIVE_SIZE, || CliPluginInfo {
                id: id.to_string(),
                version: version.to_string(),
                platforms: Vec::new(),
                published_at: now_unix(),
                changelog: None,
                preview_url: None,
                preview_images: Vec::new(),
            })
            .await?;

        // Update index
        let name = name.to_string();
        let description = description.to_string();
        let version = version.to_string();
        let author = author.to_string();
        let id = id.to_string();
        let mut index = self.load_index().await?;
        if let Some(entry) = index.plugins.iter_mut().find(|p| p.id == id) {
            if semver_greater(&version, &entry.latest_version) {
                entry.latest_version = version;
            }
            entry.name = name;
            entry.description = description;
            entry.author = author;
            entry.tags = tags;
        } else {
            index.plugins.push(CliPluginEntry {
                id,
                name,
                description,
                plugin_types: Vec::new(),
                latest_version: version,
                downloads: 0,
                author,
                tags,
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
