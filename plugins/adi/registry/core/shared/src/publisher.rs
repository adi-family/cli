use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::now_unix;
use crate::signing::RegistryKeyPair;
use crate::types::PublisherCertificate;
use crate::validation::validate_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherRecord {
    pub publisher_id: String,
    pub publisher_public_key: String,
    pub registry_signature: String,
    pub created_at: u64,
    #[serde(default)]
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PublisherRegistry {
    pub publishers: Vec<PublisherRecord>,
}

fn certificate_signed_payload(publisher_id: &str, public_key: &str) -> Vec<u8> {
    format!("{publisher_id}:{public_key}").into_bytes()
}

pub struct PublisherStore {
    root: std::path::PathBuf,
}

impl PublisherStore {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self { root }
    }

    fn publishers_path(&self) -> std::path::PathBuf {
        self.root.join("publishers.json")
    }

    pub async fn init(&self) -> Result<()> {
        let path = self.publishers_path();
        if !path.exists() {
            let registry = PublisherRegistry::default();
            let json = serde_json::to_string_pretty(&registry)?;
            fs::write(&path, json).await?;
        }
        Ok(())
    }

    pub async fn load(&self) -> Result<PublisherRegistry> {
        let path = self.publishers_path();
        let data = fs::read_to_string(&path)
            .await
            .context("Failed to read publishers.json")?;
        serde_json::from_str(&data).context("Failed to parse publishers.json")
    }

    async fn save(&self, registry: &PublisherRegistry) -> Result<()> {
        let path = self.publishers_path();
        let tmp_path = self.root.join("publishers.json.tmp");
        let json = serde_json::to_string_pretty(registry)?;
        fs::write(&tmp_path, json).await?;
        fs::rename(&tmp_path, &path).await?;
        Ok(())
    }

    pub async fn register(
        &self,
        keypair: &RegistryKeyPair,
        publisher_id: &str,
        public_key: &str,
    ) -> Result<PublisherCertificate> {
        validate_id(publisher_id)?;
        let mut registry = self.load().await?;

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
            return Ok(PublisherCertificate {
                publisher_id: existing.publisher_id.clone(),
                publisher_public_key: existing.publisher_public_key.clone(),
                registry_signature: existing.registry_signature.clone(),
                created_at: existing.created_at,
            });
        }

        let payload = certificate_signed_payload(publisher_id, public_key);
        let registry_signature = keypair.sign(&payload).await?;
        let created_at = now_unix();

        registry.publishers.push(PublisherRecord {
            publisher_id: publisher_id.to_string(),
            publisher_public_key: public_key.to_string(),
            registry_signature: registry_signature.clone(),
            created_at,
            revoked: false,
        });

        self.save(&registry).await?;

        Ok(PublisherCertificate {
            publisher_id: publisher_id.to_string(),
            publisher_public_key: public_key.to_string(),
            registry_signature,
            created_at,
        })
    }

    pub async fn revoke(&self, publisher_id: &str) -> Result<()> {
        let mut registry = self.load().await?;
        let record = registry
            .publishers
            .iter_mut()
            .find(|p| p.publisher_id == publisher_id)
            .context("Publisher not found")?;
        record.revoked = true;
        self.save(&registry).await
    }

    pub async fn list_active(&self) -> Result<Vec<PublisherRecord>> {
        let registry = self.load().await?;
        Ok(registry
            .publishers
            .into_iter()
            .filter(|p| !p.revoked)
            .collect())
    }

    pub async fn verify_certificate(
        &self,
        keypair: &RegistryKeyPair,
        cert: &PublisherCertificate,
    ) -> Result<bool> {
        let public_key = keypair.load_public_key().await?;
        let payload = certificate_signed_payload(&cert.publisher_id, &cert.publisher_public_key);
        let verifier = lib_plugin_verify::Verifier::new().with_trusted_key(&public_key);
        let result = verifier.verify_signature_base64(
            &payload,
            Some(&cert.registry_signature),
            Some(&public_key),
        );
        Ok(result.is_valid())
    }
}
