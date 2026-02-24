use anyhow::{bail, Context, Result};
use lib_plugin_registry::{
    PackageEntry, PackageInfo, PlatformBuild, PluginEntry, PluginInfo, PublisherCertificate,
    RegistryIndex, WebUiMeta,
};
use lib_plugin_verify::{generate_keypair, sign_data, Verifier};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncWriteExt;

// === Validation ===

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() {
        bail!("ID must not be empty");
    }
    let valid = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-');
    if !valid {
        bail!("Invalid ID '{id}': only alphanumeric, '.', '_', '-' allowed");
    }
    Ok(())
}

fn validate_version(version: &str) -> Result<()> {
    semver::Version::parse(version)
        .with_context(|| format!("Invalid semver version '{version}'"))?;
    Ok(())
}

fn validate_platform(platform: &str) -> Result<()> {
    if platform.is_empty() {
        bail!("Platform must not be empty");
    }
    let valid = platform
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !valid {
        bail!("Invalid platform '{platform}': only alphanumeric, '_', '-' allowed");
    }
    Ok(())
}

// === Shared types ===

/// Distinguishes package vs plugin artifact paths.
#[derive(Debug, Clone, Copy)]
pub enum ArtifactKind {
    Package,
    Plugin,
}

impl ArtifactKind {
    fn dir_name(self) -> &'static str {
        match self {
            ArtifactKind::Package => "packages",
            ArtifactKind::Plugin => "plugins",
        }
    }
}

// === Publisher registry types ===

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PublisherRecord {
    pub publisher_id: String,
    pub publisher_public_key: String,
    pub registry_signature: String,
    pub created_at: u64,
    #[serde(default)]
    pub revoked: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct PublisherRegistry {
    pub publishers: Vec<PublisherRecord>,
}

fn certificate_signed_payload(publisher_id: &str, public_key: &str) -> Vec<u8> {
    format!("{}:{}", publisher_id, public_key).into_bytes()
}

/// Common fields for publishing a package or plugin.
pub struct PublishRequest<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub version: &'a str,
    pub platform: &'a str,
    pub data: &'a [u8],
    pub author: &'a str,
    pub tags: Vec<String>,
    pub publisher_signature: Option<&'a str>,
    pub publisher_public_key: Option<&'a str>,
    pub publisher_id: Option<&'a str>,
    pub publisher_certificate: Option<&'a str>,
}

/// File-based registry storage.
pub struct RegistryStorage {
    root: PathBuf,
}

impl RegistryStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Initialize storage directories and ensure registry keypair exists.
    pub async fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.root).await?;
        fs::create_dir_all(self.root.join("packages")).await?;
        fs::create_dir_all(self.root.join("plugins")).await?;

        let index_path = self.root.join("index.json");
        if !index_path.exists() {
            let index = RegistryIndex::default();
            let json = serde_json::to_string_pretty(&index)?;
            fs::write(&index_path, json).await?;
        }

        let publishers_path = self.root.join("publishers.json");
        if !publishers_path.exists() {
            let registry = PublisherRegistry::default();
            let json = serde_json::to_string_pretty(&registry)?;
            fs::write(&publishers_path, json).await?;
        }

        self.ensure_keypair().await?;

        Ok(())
    }

    /// Generate Ed25519 keypair on first run, stored as registry_key.priv / registry_key.pub.
    async fn ensure_keypair(&self) -> Result<()> {
        let priv_path = self.root.join("registry_key.priv");
        let pub_path = self.root.join("registry_key.pub");
        if priv_path.exists() && pub_path.exists() {
            return Ok(());
        }
        let (private_key, public_key) = generate_keypair();
        fs::write(&priv_path, &private_key).await?;
        fs::write(&pub_path, &public_key).await?;
        tracing::info!("Generated new registry Ed25519 keypair");
        Ok(())
    }

    /// Load the registry's public key (base64).
    pub async fn load_public_key(&self) -> Result<String> {
        let path = self.root.join("registry_key.pub");
        fs::read_to_string(&path)
            .await
            .context("Failed to read registry public key")
    }

    /// Load the registry's private key (base64).
    async fn load_private_key(&self) -> Result<String> {
        let path = self.root.join("registry_key.priv");
        fs::read_to_string(&path)
            .await
            .context("Failed to read registry private key")
    }

    /// Sign artifact data with the registry's private key. Returns base64 signature.
    async fn sign_artifact(&self, data: &[u8]) -> Result<String> {
        let private_key = self.load_private_key().await?;
        sign_data(data, &private_key).context("Failed to sign artifact with registry key")
    }

    // === Publisher Operations ===

    /// Load the publisher registry.
    pub async fn load_publishers(&self) -> Result<PublisherRegistry> {
        let path = self.root.join("publishers.json");
        let data = fs::read_to_string(&path)
            .await
            .context("Failed to read publishers.json")?;
        serde_json::from_str(&data).context("Failed to parse publishers.json")
    }

    /// Save the publisher registry atomically.
    async fn save_publishers(&self, registry: &PublisherRegistry) -> Result<()> {
        let path = self.root.join("publishers.json");
        let tmp_path = self.root.join("publishers.json.tmp");
        let json = serde_json::to_string_pretty(registry)?;
        fs::write(&tmp_path, json).await?;
        fs::rename(&tmp_path, &path).await?;
        Ok(())
    }

    /// Register a publisher. Returns a certificate signed by the registry.
    /// Idempotent for same id+key. Rejects duplicate id with different key.
    pub async fn register_publisher(
        &self,
        publisher_id: &str,
        public_key: &str,
    ) -> Result<PublisherCertificate> {
        validate_id(publisher_id)?;

        let mut registry = self.load_publishers().await?;

        // Check for existing registration
        if let Some(existing) = registry
            .publishers
            .iter()
            .find(|p| p.publisher_id == publisher_id)
        {
            if existing.publisher_public_key != public_key {
                bail!(
                    "Publisher '{}' already registered with a different key",
                    publisher_id
                );
            }
            // Idempotent: return existing certificate
            return Ok(PublisherCertificate {
                publisher_id: existing.publisher_id.clone(),
                publisher_public_key: existing.publisher_public_key.clone(),
                registry_signature: existing.registry_signature.clone(),
                created_at: existing.created_at,
            });
        }

        let payload = certificate_signed_payload(publisher_id, public_key);
        let private_key = self.load_private_key().await?;
        let registry_signature =
            sign_data(&payload, &private_key).context("Failed to sign publisher certificate")?;

        let created_at = now_unix();

        registry.publishers.push(PublisherRecord {
            publisher_id: publisher_id.to_string(),
            publisher_public_key: public_key.to_string(),
            registry_signature: registry_signature.clone(),
            created_at,
            revoked: false,
        });

        self.save_publishers(&registry).await?;

        Ok(PublisherCertificate {
            publisher_id: publisher_id.to_string(),
            publisher_public_key: public_key.to_string(),
            registry_signature,
            created_at,
        })
    }

    /// Revoke a publisher by ID.
    pub async fn revoke_publisher(&self, publisher_id: &str) -> Result<()> {
        let mut registry = self.load_publishers().await?;
        let record = registry
            .publishers
            .iter_mut()
            .find(|p| p.publisher_id == publisher_id)
            .context("Publisher not found")?;
        record.revoked = true;
        self.save_publishers(&registry).await
    }

    /// List all non-revoked publishers.
    pub async fn list_publishers(&self) -> Result<Vec<PublisherRecord>> {
        let registry = self.load_publishers().await?;
        Ok(registry
            .publishers
            .into_iter()
            .filter(|p| !p.revoked)
            .collect())
    }

    /// Verify a publisher certificate's registry signature.
    pub async fn verify_certificate(&self, cert: &PublisherCertificate) -> Result<bool> {
        let public_key = self.load_public_key().await?;
        let payload = certificate_signed_payload(&cert.publisher_id, &cert.publisher_public_key);
        let verifier = Verifier::new().with_trusted_key(&public_key);
        let result =
            verifier.verify_signature_base64(&payload, Some(&cert.registry_signature), Some(&public_key));
        Ok(result.is_valid())
    }

    // === Index Operations ===

    /// Load the registry index.
    pub async fn load_index(&self) -> Result<RegistryIndex> {
        let path = self.root.join("index.json");
        let data = fs::read_to_string(&path)
            .await
            .context("Failed to read index.json")?;
        serde_json::from_str(&data).context("Failed to parse index.json")
    }

    /// Save the registry index atomically (write to tmp, then rename).
    pub async fn save_index(&self, index: &RegistryIndex) -> Result<()> {
        let path = self.root.join("index.json");
        let tmp_path = self.root.join("index.json.tmp");
        let json = serde_json::to_string_pretty(index)?;
        fs::write(&tmp_path, json).await?;
        fs::rename(&tmp_path, &path).await?;
        Ok(())
    }

    // === Shared artifact helpers ===

    fn artifact_dir(&self, kind: ArtifactKind, id: &str) -> PathBuf {
        self.root.join(kind.dir_name()).join(id)
    }

    fn artifact_version_dir(&self, kind: ArtifactKind, id: &str, version: &str) -> PathBuf {
        self.artifact_dir(kind, id).join(version)
    }

    /// Get artifact file path for a specific platform build.
    pub fn artifact_path(
        &self,
        kind: ArtifactKind,
        id: &str,
        version: &str,
        platform: &str,
    ) -> PathBuf {
        self.artifact_version_dir(kind, id, version)
            .join(format!("{platform}.tar.gz"))
    }

    /// Load info.json for a given artifact version.
    async fn get_artifact_info<T: DeserializeOwned>(
        &self,
        kind: ArtifactKind,
        id: &str,
        version: &str,
    ) -> Result<T> {
        validate_id(id)?;
        validate_version(version)?;
        let path = self.artifact_version_dir(kind, id, version).join("info.json");
        let data = fs::read_to_string(&path).await?;
        serde_json::from_str(&data).context("Failed to parse info.json")
    }

    /// Get latest artifact version by looking up the index.
    async fn get_artifact_latest<T: DeserializeOwned>(
        &self,
        kind: ArtifactKind,
        id: &str,
        version_lookup: impl FnOnce(&RegistryIndex) -> Option<String>,
    ) -> Result<T> {
        validate_id(id)?;
        let index = self.load_index().await?;
        let version = version_lookup(&index).context("Artifact not found in index")?;
        self.get_artifact_info(kind, id, &version).await
    }

    /// Shared publish logic: checksum, write artifact, create/update info.json.
    async fn publish_artifact<T, F>(
        &self,
        kind: ArtifactKind,
        req: &PublishRequest<'_>,
        create_info: F,
    ) -> Result<()>
    where
        T: serde::Serialize + DeserializeOwned,
        F: FnOnce() -> T,
    {
        validate_id(req.id)?;
        validate_version(req.version)?;
        validate_platform(req.platform)?;

        let version_dir = self.artifact_version_dir(kind, req.id, req.version);
        fs::create_dir_all(&version_dir).await?;

        // Calculate checksum
        let mut hasher = Sha256::new();
        hasher.update(req.data);
        let checksum = hex::encode(hasher.finalize());

        // Verify publisher signature if provided
        if let (Some(sig), Some(key)) = (req.publisher_signature, req.publisher_public_key) {
            let verifier = Verifier::new();
            let result = verifier.verify_signature_base64(req.data, Some(sig), Some(key));
            if !result.is_valid() {
                bail!("Invalid publisher signature");
            }
        }

        // Verify publisher certificate chain if provided
        let parsed_certificate = if let Some(cert_json) = req.publisher_certificate {
            let cert: PublisherCertificate = serde_json::from_str(cert_json)
                .context("Invalid publisher certificate JSON")?;

            // Verify registry's signature on the certificate
            if !self.verify_certificate(&cert).await? {
                bail!("Invalid publisher certificate: registry signature verification failed");
            }

            // Verify certificate's public key matches the signing key
            if let Some(pub_key) = req.publisher_public_key {
                if cert.publisher_public_key != pub_key {
                    bail!("Publisher certificate key does not match signing key");
                }
            }

            // Verify publisher_id matches certificate
            if let Some(pid) = req.publisher_id {
                if cert.publisher_id != pid {
                    bail!("Publisher ID does not match certificate");
                }
            }

            // Check publisher not revoked
            let registry = self.load_publishers().await?;
            if let Some(record) = registry
                .publishers
                .iter()
                .find(|p| p.publisher_id == cert.publisher_id)
            {
                if record.revoked {
                    bail!("Publisher '{}' has been revoked", cert.publisher_id);
                }
            }

            Some(cert)
        } else {
            None
        };

        // Co-sign with registry key
        let registry_signature = self.sign_artifact(req.data).await?;

        // Write artifact
        let artifact_path = version_dir.join(format!("{}.tar.gz", req.platform));
        let mut file = fs::File::create(&artifact_path).await?;
        file.write_all(req.data).await?;

        // Load or create info
        let info_path = version_dir.join("info.json");
        let mut info_value = if info_path.exists() {
            let data = fs::read_to_string(&info_path).await?;
            serde_json::from_str::<serde_json::Value>(&data)?
        } else {
            serde_json::to_value(create_info())?
        };

        // Add/update platform build in the platforms array
        let build = PlatformBuild {
            platform: req.platform.to_string(),
            download_url: format!(
                "/v1/{}/{}/{}/{}.tar.gz",
                kind.dir_name(),
                req.id,
                req.version,
                req.platform
            ),
            size_bytes: req.data.len() as u64,
            checksum,
            publisher_signature: req.publisher_signature.map(String::from),
            publisher_public_key: req.publisher_public_key.map(String::from),
            registry_signature: Some(registry_signature),
            publisher_id: req.publisher_id.map(String::from),
            publisher_certificate: parsed_certificate,
        };

        let build_value = serde_json::to_value(&build)?;
        if let Some(platforms) = info_value.get_mut("platforms").and_then(|v| v.as_array_mut()) {
            if let Some(existing) = platforms
                .iter_mut()
                .find(|p| p.get("platform").and_then(|v| v.as_str()) == Some(req.platform))
            {
                *existing = build_value;
            } else {
                platforms.push(build_value);
            }
        }

        // Save info atomically
        let json = serde_json::to_string_pretty(&info_value)?;
        let tmp_path = version_dir.join("info.json.tmp");
        fs::write(&tmp_path, &json).await?;
        fs::rename(&tmp_path, &info_path).await?;

        Ok(())
    }

    /// Shared index update logic for both packages and plugins.
    async fn update_index_entry<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut RegistryIndex),
    {
        let mut index = self.load_index().await?;
        updater(&mut index);
        index.updated_at = now_unix();
        self.save_index(&index).await
    }

    // === Package Operations ===

    /// Get package info for a specific version.
    pub async fn get_package_info(&self, id: &str, version: &str) -> Result<PackageInfo> {
        self.get_artifact_info(ArtifactKind::Package, id, version)
            .await
    }

    /// Get latest package version.
    pub async fn get_package_latest(&self, id: &str) -> Result<PackageInfo> {
        self.get_artifact_latest(ArtifactKind::Package, id, |index| {
            index
                .packages
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.latest_version.clone())
        })
        .await
    }

    /// Get package artifact path.
    pub fn package_artifact_path(&self, id: &str, version: &str, platform: &str) -> PathBuf {
        self.artifact_path(ArtifactKind::Package, id, version, platform)
    }

    /// Publish a package version.
    #[allow(clippy::too_many_arguments)]
    pub async fn publish_package(
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

        self.publish_artifact(ArtifactKind::Package, &req, || PackageInfo {
            id: id.to_string(),
            version: version.to_string(),
            platforms: Vec::new(),
            published_at: now_unix(),
            changelog: None,
        })
        .await?;

        let name = name.to_string();
        let description = description.to_string();
        let version = version.to_string();
        let author = author.to_string();
        let id = id.to_string();
        self.update_index_entry(move |index| {
            if let Some(entry) = index.packages.iter_mut().find(|p| p.id == id) {
                if semver_greater(&version, &entry.latest_version) {
                    entry.latest_version = version;
                }
                entry.name = name;
                entry.description = description;
                entry.author = author;
                entry.tags = tags;
            } else {
                index.packages.push(PackageEntry {
                    id,
                    name,
                    description,
                    plugin_count: 0,
                    plugin_ids: Vec::new(),
                    latest_version: version,
                    downloads: 0,
                    author,
                    tags,
                });
            }
        })
        .await
    }

    // === Plugin Operations ===

    /// Get plugin info for a specific version.
    pub async fn get_plugin_info(&self, id: &str, version: &str) -> Result<PluginInfo> {
        let mut info: PluginInfo = self
            .get_artifact_info(ArtifactKind::Plugin, id, version)
            .await?;
        info.web_ui = self.web_ui_meta(id, version);
        Ok(info)
    }

    /// Get latest plugin version.
    pub async fn get_plugin_latest(&self, id: &str) -> Result<PluginInfo> {
        validate_id(id)?;
        let index = self.load_index().await?;
        let entry = index
            .plugins
            .iter()
            .find(|p| p.id == id)
            .context("Plugin not found")?;
        self.get_plugin_info(id, &entry.latest_version).await
    }

    /// Get plugin artifact path.
    pub fn plugin_artifact_path(&self, id: &str, version: &str, platform: &str) -> PathBuf {
        self.artifact_path(ArtifactKind::Plugin, id, version, platform)
    }

    /// Publish a plugin version.
    ///
    /// Automatically extracts `web.js` from the tar.gz archive (if present)
    /// and adds `"web"` to `plugin_types`.
    #[allow(clippy::too_many_arguments)]
    pub async fn publish_plugin(
        &self,
        id: &str,
        name: &str,
        description: &str,
        plugin_types: &[String],
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

        self.publish_artifact(ArtifactKind::Plugin, &req, || PluginInfo {
            id: id.to_string(),
            version: version.to_string(),
            platforms: Vec::new(),
            published_at: now_unix(),
            web_ui: None,
        })
        .await?;

        // Auto-extract web.js from the archive and publish it
        let mut plugin_types = plugin_types.to_vec();
        if let Some(web_js_bytes) = extract_web_js_from_tar_gz(data) {
            tracing::info!("Auto-extracted web.js ({} bytes) from archive for {id}", web_js_bytes.len());
            self.publish_plugin_web_ui(id, version, &web_js_bytes).await?;
            if !plugin_types.iter().any(|t| t == "web") {
                plugin_types.push("web".to_string());
            }
        }

        let name = name.to_string();
        let description = description.to_string();
        let version = version.to_string();
        let author = author.to_string();
        let id = id.to_string();
        self.update_index_entry(move |index| {
            if let Some(entry) = index.plugins.iter_mut().find(|p| p.id == id) {
                if semver_greater(&version, &entry.latest_version) {
                    entry.latest_version = version;
                }
                entry.name = name;
                entry.description = description;
                entry.plugin_types = plugin_types;
                entry.author = author;
                entry.tags = tags;
            } else {
                index.plugins.push(PluginEntry {
                    id,
                    name,
                    description,
                    plugin_types,
                    package_id: None,
                    latest_version: version,
                    downloads: 0,
                    author,
                    tags,
                });
            }
        })
        .await
    }

    // === Web UI Operations ===

    /// Store the single JS entry point for a plugin's web UI.
    pub async fn publish_plugin_web_ui(
        &self,
        id: &str,
        version: &str,
        data: &[u8],
    ) -> Result<()> {
        let version_dir = self.artifact_version_dir(ArtifactKind::Plugin, id, version);
        fs::create_dir_all(&version_dir).await?;

        let js_path = version_dir.join("web.js");
        let mut file = fs::File::create(&js_path).await?;
        file.write_all(data).await?;

        let meta = serde_json::json!({ "size_bytes": data.len() });
        let meta_path = version_dir.join("web_meta.json");
        fs::write(&meta_path, serde_json::to_string_pretty(&meta)?).await?;

        Ok(())
    }

    /// Get the filesystem path to a plugin's web UI JS file.
    pub fn get_plugin_web_ui_path(&self, id: &str, version: &str) -> PathBuf {
        self.artifact_version_dir(ArtifactKind::Plugin, id, version)
            .join("web.js")
    }

    /// Check if a plugin version has a web UI.
    pub fn has_plugin_web_ui(&self, id: &str, version: &str) -> bool {
        self.get_plugin_web_ui_path(id, version).exists()
    }

    /// Build WebUiMeta for a plugin version if web.js exists.
    fn web_ui_meta(&self, id: &str, version: &str) -> Option<WebUiMeta> {
        let js_path = self.get_plugin_web_ui_path(id, version);
        if !js_path.exists() {
            return None;
        }
        let size_bytes = std::fs::metadata(&js_path).map(|m| m.len()).unwrap_or(0);
        Some(WebUiMeta {
            entry_url: format!("/v1/plugins/{}/{}/web.js", id, version),
            size_bytes,
        })
    }

    // === Version Listing ===

    /// List all published versions for an artifact.
    pub async fn list_artifact_versions(
        &self,
        kind: ArtifactKind,
        id: &str,
    ) -> Result<Vec<String>> {
        validate_id(id)?;
        let dir = self.artifact_dir(kind, id);
        let mut versions = Vec::new();
        if dir.exists() {
            let mut entries = fs::read_dir(&dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        if semver::Version::parse(name).is_ok() {
                            versions.push(name.to_string());
                        }
                    }
                }
            }
        }
        versions.sort_by(|a, b| {
            semver::Version::parse(a)
                .unwrap()
                .cmp(&semver::Version::parse(b).unwrap())
        });
        Ok(versions)
    }

    // === Download counter ===

    /// Increment download counter.
    pub async fn increment_downloads(&self, kind: ArtifactKind, id: &str) -> Result<()> {
        let mut index = self.load_index().await?;

        match kind {
            ArtifactKind::Package => {
                if let Some(entry) = index.packages.iter_mut().find(|p| p.id == id) {
                    entry.downloads += 1;
                }
            }
            ArtifactKind::Plugin => {
                if let Some(entry) = index.plugins.iter_mut().find(|p| p.id == id) {
                    entry.downloads += 1;
                }
            }
        }

        self.save_index(&index).await
    }
}

/// Scan a tar.gz archive in memory for a `web.js` entry and return its contents.
fn extract_web_js_from_tar_gz(data: &[u8]) -> Option<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let decoder = GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive.entries().ok()?;

    for entry in entries {
        let mut entry = entry.ok()?;
        let is_web_js = entry
            .path()
            .ok()
            .and_then(|p| p.file_name().map(|f| f == "web.js"))
            .unwrap_or(false);
        if !is_web_js {
            continue;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).ok()?;
        return Some(buf);
    }
    None
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn semver_greater(a: &str, b: &str) -> bool {
    match (semver::Version::parse(a), semver::Version::parse(b)) {
        (Ok(va), Ok(vb)) => va > vb,
        _ => a > b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup() -> (RegistryStorage, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();
        storage
            .publish_plugin(
                "adi.tasks",
                "Tasks",
                "Task management",
                &["core".to_string()],
                "1.0.0",
                "darwin-aarch64",
                b"fake binary",
                "ADI Team",
                vec![],
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        (storage, tmp)
    }

    // === Web UI tests ===

    #[tokio::test]
    async fn test_publish_web_ui_creates_file() {
        let (storage, _tmp) = setup().await;
        let js = b"console.log('hello');";
        storage
            .publish_plugin_web_ui("adi.tasks", "1.0.0", js)
            .await
            .unwrap();
        let path = storage.get_plugin_web_ui_path("adi.tasks", "1.0.0");
        assert!(path.exists());
        let content = std::fs::read(&path).unwrap();
        assert_eq!(content, js);
    }

    #[tokio::test]
    async fn test_publish_web_ui_size_metadata() {
        let (storage, _tmp) = setup().await;
        let js = b"export default class {}";
        storage
            .publish_plugin_web_ui("adi.tasks", "1.0.0", js)
            .await
            .unwrap();
        let meta_path = storage
            .artifact_version_dir(ArtifactKind::Plugin, "adi.tasks", "1.0.0")
            .join("web_meta.json");
        let meta: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(meta_path).unwrap()).unwrap();
        assert_eq!(meta["size_bytes"], js.len() as u64);
    }

    #[tokio::test]
    async fn test_has_web_ui_true() {
        let (storage, _tmp) = setup().await;
        storage
            .publish_plugin_web_ui("adi.tasks", "1.0.0", b"js code")
            .await
            .unwrap();
        assert!(storage.has_plugin_web_ui("adi.tasks", "1.0.0"));
    }

    #[tokio::test]
    async fn test_has_web_ui_false() {
        let (storage, _tmp) = setup().await;
        assert!(!storage.has_plugin_web_ui("adi.tasks", "1.0.0"));
    }

    #[tokio::test]
    async fn test_publish_web_ui_overwrite() {
        let (storage, _tmp) = setup().await;
        storage
            .publish_plugin_web_ui("adi.tasks", "1.0.0", b"first")
            .await
            .unwrap();
        storage
            .publish_plugin_web_ui("adi.tasks", "1.0.0", b"second")
            .await
            .unwrap();
        let path = storage.get_plugin_web_ui_path("adi.tasks", "1.0.0");
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "second");
    }

    #[tokio::test]
    async fn test_plugin_info_includes_web_ui() {
        let (storage, _tmp) = setup().await;
        let js = b"export default class MyPlugin {}";
        storage
            .publish_plugin_web_ui("adi.tasks", "1.0.0", js)
            .await
            .unwrap();
        let info = storage.get_plugin_info("adi.tasks", "1.0.0").await.unwrap();
        let web_ui = info.web_ui.unwrap();
        assert_eq!(web_ui.entry_url, "/v1/plugins/adi.tasks/1.0.0/web.js");
        assert_eq!(web_ui.size_bytes, js.len() as u64);
    }

    #[tokio::test]
    async fn test_plugin_info_without_web_ui() {
        let (storage, _tmp) = setup().await;
        let info = storage.get_plugin_info("adi.tasks", "1.0.0").await.unwrap();
        assert!(info.web_ui.is_none());
    }

    // === Phase 4a: Core storage tests ===

    #[tokio::test]
    async fn test_publish_package_and_get_info() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        storage
            .publish_package(
                "my.pkg",
                "My Package",
                "A test package",
                "1.0.0",
                "linux-x86_64",
                b"binary data",
                "tester",
                vec!["test".to_string()],
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let info = storage.get_package_info("my.pkg", "1.0.0").await.unwrap();
        assert_eq!(info.id, "my.pkg");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.platforms.len(), 1);
        assert_eq!(info.platforms[0].platform, "linux-x86_64");
        assert_eq!(info.platforms[0].size_bytes, 11);
        assert!(info.published_at > 0);
    }

    #[tokio::test]
    async fn test_publish_plugin_and_get_info() {
        let (storage, _tmp) = setup().await;
        let info = storage.get_plugin_info("adi.tasks", "1.0.0").await.unwrap();
        assert_eq!(info.id, "adi.tasks");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.platforms.len(), 1);
        assert_eq!(info.platforms[0].platform, "darwin-aarch64");
    }

    #[tokio::test]
    async fn test_get_package_latest() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        storage
            .publish_package("my.pkg", "Pkg", "desc", "1.0.0", "linux-x86_64", b"data", "author", vec![], None, None, None, None)
            .await
            .unwrap();

        let info = storage.get_package_latest("my.pkg").await.unwrap();
        assert_eq!(info.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_get_plugin_latest() {
        let (storage, _tmp) = setup().await;
        let info = storage.get_plugin_latest("adi.tasks").await.unwrap();
        assert_eq!(info.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_increment_downloads() {
        let (storage, _tmp) = setup().await;
        let before = storage.load_index().await.unwrap();
        let before_count = before.plugins.iter().find(|p| p.id == "adi.tasks").unwrap().downloads;

        storage.increment_downloads(ArtifactKind::Plugin, "adi.tasks").await.unwrap();

        let after = storage.load_index().await.unwrap();
        let after_count = after.plugins.iter().find(|p| p.id == "adi.tasks").unwrap().downloads;
        assert_eq!(after_count, before_count + 1);
    }

    #[tokio::test]
    async fn test_semver_greater_edge_cases() {
        assert!(semver_greater("2.0.0", "1.0.0"));
        assert!(semver_greater("1.1.0", "1.0.0"));
        assert!(semver_greater("1.0.1", "1.0.0"));
        assert!(!semver_greater("1.0.0", "1.0.0"));
        assert!(!semver_greater("0.9.0", "1.0.0"));
        assert!(semver_greater("1.0.0", "1.0.0-alpha"));
    }

    #[tokio::test]
    async fn test_index_update_on_republish() {
        let (storage, _tmp) = setup().await;

        // Publish v2.0.0
        storage
            .publish_plugin(
                "adi.tasks",
                "Tasks v2",
                "Updated",
                &["core".to_string()],
                "2.0.0",
                "darwin-aarch64",
                b"new binary",
                "ADI Team",
                vec![],
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let index = storage.load_index().await.unwrap();
        let entry = index.plugins.iter().find(|p| p.id == "adi.tasks").unwrap();
        assert_eq!(entry.latest_version, "2.0.0");
    }

    #[tokio::test]
    async fn test_invalid_id_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let result = storage
            .publish_package("../evil", "Evil", "desc", "1.0.0", "linux-x86_64", b"data", "hacker", vec![], None, None, None, None)
            .await;
        assert!(result.is_err());

        let result = storage
            .publish_package("foo/bar", "Evil", "desc", "1.0.0", "linux-x86_64", b"data", "hacker", vec![], None, None, None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_version_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let result = storage
            .publish_package("my.pkg", "Pkg", "desc", "not-semver", "linux-x86_64", b"data", "author", vec![], None, None, None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_artifact_versions() {
        let (storage, _tmp) = setup().await;

        // Publish v2.0.0
        storage
            .publish_plugin(
                "adi.tasks",
                "Tasks",
                "desc",
                &["core".to_string()],
                "2.0.0",
                "darwin-aarch64",
                b"data",
                "ADI Team",
                vec![],
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let versions = storage
            .list_artifact_versions(ArtifactKind::Plugin, "adi.tasks")
            .await
            .unwrap();
        assert_eq!(versions, vec!["1.0.0", "2.0.0"]);
    }

    #[tokio::test]
    async fn test_list_versions_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let versions = storage
            .list_artifact_versions(ArtifactKind::Plugin, "nonexistent")
            .await
            .unwrap();
        assert!(versions.is_empty());
    }

    /// Build a tar.gz archive in memory containing the given files.
    fn make_tar_gz(files: &[(&str, &[u8])]) -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let buf = Vec::new();
        let enc = GzEncoder::new(buf, Compression::fast());
        let mut builder = tar::Builder::new(enc);
        for (name, data) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, name, &data[..]).unwrap();
        }
        let enc = builder.into_inner().unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn test_extract_web_js_found() {
        let archive = make_tar_gz(&[
            ("plugin.toml", b"[plugin]\nid = \"test\""),
            ("web.js", b"console.log('hello');"),
        ]);
        let result = extract_web_js_from_tar_gz(&archive);
        assert_eq!(result.as_deref(), Some(b"console.log('hello');" as &[u8]));
    }

    #[test]
    fn test_extract_web_js_not_found() {
        let archive = make_tar_gz(&[("plugin.toml", b"[plugin]\nid = \"test\"")]);
        assert!(extract_web_js_from_tar_gz(&archive).is_none());
    }

    #[tokio::test]
    async fn test_publish_plugin_auto_extracts_web_js() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let web_content = b"export default class WebUI {}";
        let archive = make_tar_gz(&[
            ("plugin.dylib", b"fake binary"),
            ("plugin.toml", b"[plugin]\nid = \"adi.tasks\""),
            ("web.js", web_content),
        ]);

        storage
            .publish_plugin(
                "adi.tasks", "Tasks", "Task management",
                &["core".to_string()],
                "1.0.0", "darwin-aarch64", &archive,
                "ADI Team", vec![],
                None, None, None, None,
            )
            .await
            .unwrap();

        // web.js should have been auto-extracted
        assert!(storage.has_plugin_web_ui("adi.tasks", "1.0.0"));
        let path = storage.get_plugin_web_ui_path("adi.tasks", "1.0.0");
        assert_eq!(std::fs::read(&path).unwrap(), web_content);

        // "web" should have been auto-added to plugin_types
        let index = storage.load_index().await.unwrap();
        let entry = index.plugins.iter().find(|p| p.id == "adi.tasks").unwrap();
        assert!(entry.plugin_types.contains(&"web".to_string()));
        assert!(entry.plugin_types.contains(&"core".to_string()));
    }

    #[tokio::test]
    async fn test_publish_plugin_no_web_js_no_auto_type() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let archive = make_tar_gz(&[
            ("plugin.dylib", b"fake binary"),
            ("plugin.toml", b"[plugin]\nid = \"adi.tasks\""),
        ]);

        storage
            .publish_plugin(
                "adi.tasks", "Tasks", "Task management",
                &["core".to_string()],
                "1.0.0", "darwin-aarch64", &archive,
                "ADI Team", vec![],
                None, None, None, None,
            )
            .await
            .unwrap();

        assert!(!storage.has_plugin_web_ui("adi.tasks", "1.0.0"));

        let index = storage.load_index().await.unwrap();
        let entry = index.plugins.iter().find(|p| p.id == "adi.tasks").unwrap();
        assert_eq!(entry.plugin_types, vec!["core"]);
    }

    // === Signature verification tests ===

    #[tokio::test]
    async fn test_publish_with_valid_publisher_signature() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let data = b"signed plugin binary";
        let (private_key, public_key) = generate_keypair();
        let signature = sign_data(data, &private_key).unwrap();

        storage
            .publish_plugin(
                "adi.signed",
                "Signed Plugin",
                "A signed plugin",
                &["core".to_string()],
                "1.0.0",
                "darwin-aarch64",
                data,
                "ADI Team",
                vec![],
                Some(&signature),
                Some(&public_key),
                None,
                None,
            )
            .await
            .unwrap();

        let info = storage.get_plugin_info("adi.signed", "1.0.0").await.unwrap();
        let build = &info.platforms[0];
        assert_eq!(build.publisher_signature.as_deref(), Some(signature.as_str()));
        assert_eq!(build.publisher_public_key.as_deref(), Some(public_key.as_str()));
        assert!(build.registry_signature.is_some());
    }

    #[tokio::test]
    async fn test_publish_with_invalid_publisher_signature_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let data = b"plugin binary";
        let (_, public_key) = generate_keypair();
        let (other_private, _) = generate_keypair();
        let bad_signature = sign_data(data, &other_private).unwrap();

        let result = storage
            .publish_plugin(
                "adi.bad",
                "Bad Plugin",
                "Invalid sig",
                &["core".to_string()],
                "1.0.0",
                "darwin-aarch64",
                data,
                "ADI Team",
                vec![],
                Some(&bad_signature),
                Some(&public_key),
                None,
                None,
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("signature"));
    }

    #[tokio::test]
    async fn test_publish_without_signature_still_gets_registry_signature() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        storage
            .publish_plugin(
                "adi.unsigned",
                "Unsigned Plugin",
                "No publisher sig",
                &["core".to_string()],
                "1.0.0",
                "darwin-aarch64",
                b"unsigned data",
                "ADI Team",
                vec![],
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let info = storage.get_plugin_info("adi.unsigned", "1.0.0").await.unwrap();
        let build = &info.platforms[0];
        assert!(build.publisher_signature.is_none());
        assert!(build.publisher_public_key.is_none());
        assert!(build.registry_signature.is_some());
    }

    // === Publisher certificate tests ===

    #[tokio::test]
    async fn test_register_publisher_creates_certificate() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let (_, public_key) = generate_keypair();
        let cert = storage.register_publisher("acme", &public_key).await.unwrap();

        assert_eq!(cert.publisher_id, "acme");
        assert_eq!(cert.publisher_public_key, public_key);
        assert!(cert.created_at > 0);

        // Verify certificate signature
        assert!(storage.verify_certificate(&cert).await.unwrap());
    }

    #[tokio::test]
    async fn test_register_publisher_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let (_, public_key) = generate_keypair();
        let cert1 = storage.register_publisher("acme", &public_key).await.unwrap();
        let cert2 = storage.register_publisher("acme", &public_key).await.unwrap();

        assert_eq!(cert1.registry_signature, cert2.registry_signature);
        assert_eq!(cert1.created_at, cert2.created_at);
    }

    #[tokio::test]
    async fn test_register_publisher_rejects_different_key() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let (_, key1) = generate_keypair();
        let (_, key2) = generate_keypair();
        storage.register_publisher("acme", &key1).await.unwrap();

        let result = storage.register_publisher("acme", &key2).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("different key"));
    }

    #[tokio::test]
    async fn test_revoke_publisher() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let (_, public_key) = generate_keypair();
        storage.register_publisher("acme", &public_key).await.unwrap();

        storage.revoke_publisher("acme").await.unwrap();

        let publishers = storage.list_publishers().await.unwrap();
        assert!(publishers.is_empty());

        // Full registry still has the record (just revoked)
        let registry = storage.load_publishers().await.unwrap();
        assert_eq!(registry.publishers.len(), 1);
        assert!(registry.publishers[0].revoked);
    }

    #[tokio::test]
    async fn test_publish_with_valid_certificate_chain() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let data = b"certified plugin binary";
        let (private_key, public_key) = generate_keypair();
        let signature = sign_data(data, &private_key).unwrap();

        // Register publisher and get certificate
        let cert = storage.register_publisher("acme", &public_key).await.unwrap();
        let cert_json = serde_json::to_string(&cert).unwrap();

        // Publish with full certificate chain
        storage
            .publish_plugin(
                "acme.plugin",
                "Acme Plugin",
                "Certified plugin",
                &["core".to_string()],
                "1.0.0",
                "darwin-aarch64",
                data,
                "Acme Corp",
                vec![],
                Some(&signature),
                Some(&public_key),
                Some("acme"),
                Some(&cert_json),
            )
            .await
            .unwrap();

        let info = storage.get_plugin_info("acme.plugin", "1.0.0").await.unwrap();
        let build = &info.platforms[0];
        assert_eq!(build.publisher_id.as_deref(), Some("acme"));
        assert!(build.publisher_certificate.is_some());
        assert!(build.registry_signature.is_some());
    }

    #[tokio::test]
    async fn test_publish_with_invalid_certificate_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let data = b"plugin binary";
        let (private_key, public_key) = generate_keypair();
        let signature = sign_data(data, &private_key).unwrap();

        // Forge a certificate with a fake registry signature
        let forged_cert = PublisherCertificate {
            publisher_id: "evil".to_string(),
            publisher_public_key: public_key.clone(),
            registry_signature: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            created_at: 12345,
        };
        let cert_json = serde_json::to_string(&forged_cert).unwrap();

        let result = storage
            .publish_plugin(
                "evil.plugin",
                "Evil Plugin",
                "Forged cert",
                &["core".to_string()],
                "1.0.0",
                "darwin-aarch64",
                data,
                "Evil Corp",
                vec![],
                Some(&signature),
                Some(&public_key),
                Some("evil"),
                Some(&cert_json),
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("certificate"));
    }

    #[tokio::test]
    async fn test_publish_with_mismatched_key_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let data = b"plugin binary";
        let (_private_key, public_key) = generate_keypair();
        let (other_private, other_public) = generate_keypair();
        let signature = sign_data(data, &other_private).unwrap();

        // Register with one key, sign with another
        let cert = storage.register_publisher("acme", &public_key).await.unwrap();
        let cert_json = serde_json::to_string(&cert).unwrap();

        let result = storage
            .publish_plugin(
                "acme.plugin",
                "Acme Plugin",
                "Mismatched key",
                &["core".to_string()],
                "1.0.0",
                "darwin-aarch64",
                data,
                "Acme Corp",
                vec![],
                Some(&signature),
                Some(&other_public),
                Some("acme"),
                Some(&cert_json),
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not match"));
    }

    #[tokio::test]
    async fn test_publish_with_revoked_publisher_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        let data = b"plugin binary";
        let (private_key, public_key) = generate_keypair();
        let signature = sign_data(data, &private_key).unwrap();

        let cert = storage.register_publisher("acme", &public_key).await.unwrap();
        let cert_json = serde_json::to_string(&cert).unwrap();

        // Revoke the publisher
        storage.revoke_publisher("acme").await.unwrap();

        let result = storage
            .publish_plugin(
                "acme.plugin",
                "Acme Plugin",
                "Revoked",
                &["core".to_string()],
                "1.0.0",
                "darwin-aarch64",
                data,
                "Acme Corp",
                vec![],
                Some(&signature),
                Some(&public_key),
                Some("acme"),
                Some(&cert_json),
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("revoked"));
    }

    #[tokio::test]
    async fn test_publish_without_certificate_still_works() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = RegistryStorage::new(tmp.path().to_path_buf());
        storage.init().await.unwrap();

        // Backward compat: no certificate required
        storage
            .publish_plugin(
                "adi.basic",
                "Basic Plugin",
                "No cert",
                &["core".to_string()],
                "1.0.0",
                "darwin-aarch64",
                b"basic data",
                "ADI Team",
                vec![],
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let info = storage.get_plugin_info("adi.basic", "1.0.0").await.unwrap();
        let build = &info.platforms[0];
        assert!(build.publisher_id.is_none());
        assert!(build.publisher_certificate.is_none());
        assert!(build.registry_signature.is_some());
    }
}
