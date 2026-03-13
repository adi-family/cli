use anyhow::{bail, Context, Result};
use lib_plugin_verify::Verifier;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::publisher::PublisherStore;
use crate::signing::RegistryKeyPair;
use crate::types::{PlatformBuild, PublisherCertificate};
use crate::validation::{validate_id, validate_version};

/// Maximum archive size for CLI plugins (native binaries): 256 MB.
pub const MAX_CLI_ARCHIVE_SIZE: usize = 256 * 1024 * 1024;

/// Maximum archive size for web plugins (JS/CSS bundles): 32 MB.
pub const MAX_WEB_ARCHIVE_SIZE: usize = 32 * 1024 * 1024;

/// Common fields for publishing an artifact.
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

/// Generic file-based registry storage.
pub struct FileStorage {
    root: PathBuf,
    keypair: RegistryKeyPair,
    publishers: PublisherStore,
}

impl FileStorage {
    pub fn new(root: PathBuf) -> Self {
        let keypair = RegistryKeyPair::new(root.clone());
        let publishers = PublisherStore::new(root.clone());
        Self {
            root,
            keypair,
            publishers,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn keypair(&self) -> &RegistryKeyPair {
        &self.keypair
    }

    pub fn publishers(&self) -> &PublisherStore {
        &self.publishers
    }

    /// Initialize storage directory and infrastructure.
    pub async fn init(&self, subdirs: &[&str]) -> Result<()> {
        fs::create_dir_all(&self.root).await?;
        for dir in subdirs {
            fs::create_dir_all(self.root.join(dir)).await?;
        }
        self.publishers.init().await?;
        self.keypair.ensure_exists().await?;
        Ok(())
    }

    /// Load a JSON file from the root.
    pub async fn load_json<T: DeserializeOwned>(&self, filename: &str) -> Result<T> {
        let path = self.root.join(filename);
        let data = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read {filename}"))?;
        serde_json::from_str(&data).with_context(|| format!("Failed to parse {filename}"))
    }

    /// Save a JSON file atomically (write tmp, rename).
    pub async fn save_json_atomic<T: serde::Serialize>(
        &self,
        filename: &str,
        value: &T,
    ) -> Result<()> {
        let path = self.root.join(filename);
        let tmp_path = self.root.join(format!("{filename}.tmp"));
        let json = serde_json::to_string_pretty(value)?;
        fs::write(&tmp_path, json).await?;
        fs::rename(&tmp_path, &path).await?;
        Ok(())
    }

    /// Ensure the index file exists with a default value.
    pub async fn ensure_index<T: serde::Serialize + Default>(&self, filename: &str) -> Result<()> {
        let path = self.root.join(filename);
        if !path.exists() {
            let value = T::default();
            let json = serde_json::to_string_pretty(&value)?;
            fs::write(&path, json).await?;
        }
        Ok(())
    }

    // === Artifact path helpers ===

    pub fn artifact_dir(&self, kind: &str, id: &str) -> PathBuf {
        self.root.join(kind).join(id)
    }

    pub fn artifact_version_dir(&self, kind: &str, id: &str, version: &str) -> PathBuf {
        self.artifact_dir(kind, id).join(version)
    }

    pub fn artifact_path(&self, kind: &str, id: &str, version: &str, platform: &str) -> PathBuf {
        self.artifact_version_dir(kind, id, version)
            .join(format!("{platform}.tar.gz"))
    }

    // === Artifact info ===

    /// Load info.json for a given artifact version.
    pub async fn get_artifact_info<T: DeserializeOwned>(
        &self,
        kind: &str,
        id: &str,
        version: &str,
    ) -> Result<T> {
        validate_id(id)?;
        validate_version(version)?;
        let path = self
            .artifact_version_dir(kind, id, version)
            .join("info.json");
        let data = fs::read_to_string(&path).await?;
        serde_json::from_str(&data).context("Failed to parse info.json")
    }

    // === Publish artifact ===

    /// Publish an artifact: validate, checksum, verify signature, write file, update info.json.
    pub async fn publish_artifact<T, F>(
        &self,
        kind: &str,
        req: &PublishRequest<'_>,
        validate_platform_fn: Option<fn(&str) -> Result<()>>,
        max_archive_size: usize,
        create_info: F,
    ) -> Result<()>
    where
        T: serde::Serialize + DeserializeOwned,
        F: FnOnce() -> T,
    {
        if req.data.len() > max_archive_size {
            bail!(
                "Archive size {} bytes exceeds maximum allowed {} bytes",
                req.data.len(),
                max_archive_size
            );
        }
        validate_id(req.id)?;
        validate_version(req.version)?;
        if let Some(validate_platform) = validate_platform_fn {
            validate_platform(req.platform)?;
        }

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
            let cert: PublisherCertificate =
                serde_json::from_str(cert_json).context("Invalid publisher certificate JSON")?;

            if !self
                .publishers
                .verify_certificate(&self.keypair, &cert)
                .await?
            {
                bail!("Invalid publisher certificate: registry signature verification failed");
            }

            if let Some(pub_key) = req.publisher_public_key {
                if cert.publisher_public_key != pub_key {
                    bail!("Publisher certificate key does not match signing key");
                }
            }

            if let Some(pid) = req.publisher_id {
                if cert.publisher_id != pid {
                    bail!("Publisher ID does not match certificate");
                }
            }

            let registry = self.publishers.load().await?;
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
        let registry_signature = self.keypair.sign(req.data).await?;

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
                kind, req.id, req.version, req.platform
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
        if let Some(platforms) = info_value
            .get_mut("platforms")
            .and_then(|v| v.as_array_mut())
        {
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

    // === Version listing ===

    pub async fn list_artifact_versions(&self, kind: &str, id: &str) -> Result<Vec<String>> {
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
}
