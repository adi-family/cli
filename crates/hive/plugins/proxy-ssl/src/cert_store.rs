//! Certificate storage and persistence
//!
//! Stores certificates and ACME account credentials on the filesystem.
//!
//! Directory structure:
//! ```text
//! {cert_dir}/
//! ├── account.json          # ACME account credentials
//! ├── {domain}/
//! │   ├── cert.pem         # Full chain certificate
//! │   ├── privkey.pem      # Private key
//! │   └── meta.json        # Certificate metadata (expiry, etc.)
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCredentials {
    pub account_url: String,
    /// Serialized `instant_acme::AccountCredentials` (not a raw PEM key despite the field name)
    pub private_key_pem: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateMetadata {
    pub domains: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
    pub renewed_at: Option<DateTime<Utc>>,
    pub issuer: String,
    pub serial: String,
}

#[derive(Debug, Clone)]
pub struct StoredCertificate {
    pub cert_pem: String,
    pub privkey_pem: String,
    pub metadata: CertificateMetadata,
}

pub struct CertStore {
    cert_dir: PathBuf,
}

impl CertStore {
    pub fn new(cert_dir: impl Into<PathBuf>) -> Self {
        Self {
            cert_dir: cert_dir.into(),
        }
    }

    pub async fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.cert_dir)
            .await
            .context("Failed to create certificate directory")?;
        Ok(())
    }

    fn domain_dir(&self, domain: &str) -> PathBuf {
        self.cert_dir.join(sanitize_domain(domain))
    }

    fn account_path(&self) -> PathBuf {
        self.cert_dir.join("account.json")
    }

    pub async fn load_account(&self) -> Result<Option<AccountCredentials>> {
        let path = self.account_path();

        if !path.exists() {
            debug!("No account credentials found at {:?}", path);
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .await
            .context("Failed to read account credentials")?;

        let account: AccountCredentials =
            serde_json::from_str(&content).context("Failed to parse account credentials")?;

        info!("Loaded ACME account: {}", account.email);
        Ok(Some(account))
    }

    pub async fn save_account(&self, account: &AccountCredentials) -> Result<()> {
        self.ensure_dir().await?;

        let path = self.account_path();
        let content = serde_json::to_string_pretty(account)
            .context("Failed to serialize account credentials")?;

        fs::write(&path, &content)
            .await
            .context("Failed to write account credentials")?;

        // Restrict permissions on account file (contains private key)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, perms).ok();
        }

        info!("Saved ACME account credentials to {:?}", path);
        Ok(())
    }

    pub async fn load_certificate(&self, domain: &str) -> Result<Option<StoredCertificate>> {
        let dir = self.domain_dir(domain);

        let cert_path = dir.join("cert.pem");
        let key_path = dir.join("privkey.pem");
        let meta_path = dir.join("meta.json");

        if !cert_path.exists() || !key_path.exists() || !meta_path.exists() {
            debug!("Certificate not found for domain: {}", domain);
            return Ok(None);
        }

        let cert_pem = fs::read_to_string(&cert_path)
            .await
            .context("Failed to read certificate")?;

        let privkey_pem = fs::read_to_string(&key_path)
            .await
            .context("Failed to read private key")?;

        let meta_content = fs::read_to_string(&meta_path)
            .await
            .context("Failed to read certificate metadata")?;

        let metadata: CertificateMetadata =
            serde_json::from_str(&meta_content).context("Failed to parse certificate metadata")?;

        info!(
            "Loaded certificate for {}: expires {}",
            domain, metadata.expires_at
        );

        Ok(Some(StoredCertificate {
            cert_pem,
            privkey_pem,
            metadata,
        }))
    }

    pub async fn save_certificate(
        &self,
        domain: &str,
        cert_pem: &str,
        privkey_pem: &str,
        metadata: &CertificateMetadata,
    ) -> Result<()> {
        let dir = self.domain_dir(domain);
        fs::create_dir_all(&dir)
            .await
            .context("Failed to create domain directory")?;

        let cert_path = dir.join("cert.pem");
        let key_path = dir.join("privkey.pem");
        let meta_path = dir.join("meta.json");

        fs::write(&cert_path, cert_pem)
            .await
            .context("Failed to write certificate")?;

        fs::write(&key_path, privkey_pem)
            .await
            .context("Failed to write private key")?;

        // Restrict permissions on private key
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&key_path, perms).ok();
        }

        let meta_content = serde_json::to_string_pretty(metadata)
            .context("Failed to serialize certificate metadata")?;
        fs::write(&meta_path, &meta_content)
            .await
            .context("Failed to write certificate metadata")?;

        info!(
            "Saved certificate for {}: expires {}",
            domain, metadata.expires_at
        );

        Ok(())
    }

    pub async fn needs_renewal(&self, domain: &str, threshold_days: u32) -> Result<bool> {
        let cert = match self.load_certificate(domain).await? {
            Some(c) => c,
            None => {
                debug!("No certificate found for {}, needs issuance", domain);
                return Ok(true);
            }
        };

        let now = Utc::now();
        let threshold = chrono::Duration::days(threshold_days as i64);
        let renewal_time = cert.metadata.expires_at - threshold;

        if now >= renewal_time {
            info!(
                "Certificate for {} expires {} (threshold: {} days), needs renewal",
                domain, cert.metadata.expires_at, threshold_days
            );
            Ok(true)
        } else {
            let days_until_renewal = (renewal_time - now).num_days();
            debug!(
                "Certificate for {} valid until {}, renewal in {} days",
                domain, cert.metadata.expires_at, days_until_renewal
            );
            Ok(false)
        }
    }

    pub async fn list_domains(&self) -> Result<Vec<String>> {
        let mut domains = Vec::new();

        if !self.cert_dir.exists() {
            return Ok(domains);
        }

        let mut entries = fs::read_dir(&self.cert_dir)
            .await
            .context("Failed to read certificate directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !name.starts_with('.') && path.join("cert.pem").exists() {
                        domains.push(name.to_string());
                    }
                }
            }
        }

        Ok(domains)
    }

    pub async fn delete_certificate(&self, domain: &str) -> Result<()> {
        let dir = self.domain_dir(domain);

        if dir.exists() {
            fs::remove_dir_all(&dir)
                .await
                .context("Failed to delete certificate directory")?;
            info!("Deleted certificate for {}", domain);
        } else {
            warn!("No certificate found to delete for {}", domain);
        }

        Ok(())
    }

    pub async fn get_status(&self, domain: &str) -> Result<CertificateStatus> {
        match self.load_certificate(domain).await? {
            Some(cert) => {
                let now = Utc::now();
                let days_until_expiry = (cert.metadata.expires_at - now).num_days();

                let status = if now >= cert.metadata.expires_at {
                    CertificateHealth::Expired
                } else if days_until_expiry <= 7 {
                    CertificateHealth::Critical
                } else if days_until_expiry <= 30 {
                    CertificateHealth::Warning
                } else {
                    CertificateHealth::Good
                };

                Ok(CertificateStatus {
                    domain: domain.to_string(),
                    exists: true,
                    health: status,
                    expires_at: Some(cert.metadata.expires_at),
                    days_until_expiry: Some(days_until_expiry),
                    issuer: Some(cert.metadata.issuer),
                })
            }
            None => Ok(CertificateStatus {
                domain: domain.to_string(),
                exists: false,
                health: CertificateHealth::Missing,
                expires_at: None,
                days_until_expiry: None,
                issuer: None,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertificateHealth {
    /// > 30 days until expiry
    Good,
    /// <= 30 days until expiry
    Warning,
    /// <= 7 days until expiry
    Critical,
    Expired,
    Missing,
}

impl std::fmt::Display for CertificateHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Good => write!(f, "good"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
            Self::Expired => write!(f, "expired"),
            Self::Missing => write!(f, "missing"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateStatus {
    pub domain: String,
    pub exists: bool,
    pub health: CertificateHealth,
    pub expires_at: Option<DateTime<Utc>>,
    pub days_until_expiry: Option<i64>,
    pub issuer: Option<String>,
}

fn sanitize_domain(domain: &str) -> String {
    domain
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn parse_certificate_expiry(cert_pem: &str) -> Result<DateTime<Utc>> {
    use x509_parser::prelude::*;

    let parsed =
        ::pem::parse(cert_pem).map_err(|e| anyhow::anyhow!("Failed to parse PEM: {}", e))?;

    let (_, cert) = X509Certificate::from_der(parsed.contents())
        .map_err(|e| anyhow::anyhow!("Failed to parse X.509 certificate: {}", e))?;

    let not_after = cert.validity().not_after;
    let timestamp = not_after.timestamp();

    DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid certificate expiry timestamp"))
}

pub fn parse_certificate_issuer(cert_pem: &str) -> Result<String> {
    use x509_parser::prelude::*;

    let parsed =
        ::pem::parse(cert_pem).map_err(|e| anyhow::anyhow!("Failed to parse PEM: {}", e))?;

    let (_, cert) = X509Certificate::from_der(parsed.contents())
        .map_err(|e| anyhow::anyhow!("Failed to parse X.509 certificate: {}", e))?;

    Ok(cert.issuer().to_string())
}

pub fn parse_certificate_serial(cert_pem: &str) -> Result<String> {
    use x509_parser::prelude::*;

    let parsed =
        ::pem::parse(cert_pem).map_err(|e| anyhow::anyhow!("Failed to parse PEM: {}", e))?;

    let (_, cert) = X509Certificate::from_der(parsed.contents())
        .map_err(|e| anyhow::anyhow!("Failed to parse X.509 certificate: {}", e))?;

    Ok(cert.serial.to_str_radix(16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_domain() {
        assert_eq!(sanitize_domain("example.com"), "example.com");
        assert_eq!(sanitize_domain("sub.example.com"), "sub.example.com");
        assert_eq!(sanitize_domain("my-domain.co.uk"), "my-domain.co.uk");
        assert_eq!(sanitize_domain("weird/domain"), "weird_domain");
    }
}
