use anyhow::{Context, Result};
use lib_plugin_verify::{generate_keypair, sign_data};
use std::path::PathBuf;
use tokio::fs;

pub struct RegistryKeyPair {
    root: PathBuf,
}

impl RegistryKeyPair {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn priv_path(&self) -> PathBuf {
        self.root.join("registry_key.priv")
    }

    fn pub_path(&self) -> PathBuf {
        self.root.join("registry_key.pub")
    }

    pub async fn ensure_exists(&self) -> Result<()> {
        let priv_path = self.priv_path();
        let pub_path = self.pub_path();
        if priv_path.exists() && pub_path.exists() {
            return Ok(());
        }
        let (private_key, public_key) = generate_keypair();
        fs::write(&priv_path, &private_key).await?;
        fs::write(&pub_path, &public_key).await?;
        tracing::info!("Generated new registry Ed25519 keypair");
        Ok(())
    }

    pub async fn load_public_key(&self) -> Result<String> {
        fs::read_to_string(&self.pub_path())
            .await
            .context("Failed to read registry public key")
    }

    async fn load_private_key(&self) -> Result<String> {
        fs::read_to_string(&self.priv_path())
            .await
            .context("Failed to read registry private key")
    }

    pub async fn sign(&self, data: &[u8]) -> Result<String> {
        let private_key = self.load_private_key().await?;
        sign_data(data, &private_key).context("Failed to sign with registry key")
    }
}
