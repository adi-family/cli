//! TLS configuration management with hot-reload support
//!
//! Builds rustls ServerConfig from certificates and supports
//! reloading certificates without server restart.

use crate::cert_store::{CertStore, StoredCertificate};
use crate::challenge::{AlpnChallengeResolver, ChallengeManager};
use crate::config::SslConfig;
use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    server::ResolvesServerCert,
    sign::CertifiedKey,
    ServerConfig,
};
use std::io::BufReader;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

#[derive(Debug)]
struct StaticCertResolver {
    cert: Arc<CertifiedKey>,
}

impl ResolvesServerCert for StaticCertResolver {
    fn resolve(&self, _client_hello: rustls::server::ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.cert.clone())
    }
}

struct MultiDomainResolver {
    /// Fallback certificate used when SNI doesn't match any domain entry.
    primary: Arc<CertifiedKey>,
    domains: dashmap::DashMap<String, Arc<CertifiedKey>>,
}

impl std::fmt::Debug for MultiDomainResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiDomainResolver")
            .field("primary", &"CertifiedKey")
            .field("domains_count", &self.domains.len())
            .finish()
    }
}

impl MultiDomainResolver {
    fn new(primary: Arc<CertifiedKey>) -> Self {
        Self {
            primary,
            domains: dashmap::DashMap::new(),
        }
    }

    fn add_domain(&self, domain: String, cert: Arc<CertifiedKey>) {
        self.domains.insert(domain, cert);
    }
}

impl ResolvesServerCert for MultiDomainResolver {
    fn resolve(&self, client_hello: rustls::server::ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        if let Some(sni) = client_hello.server_name() {
            if let Some(cert) = self.domains.get(sni) {
                return Some(cert.clone());
            }
        }
        Some(self.primary.clone())
    }
}

pub struct TlsConfigManager {
    config: Arc<ArcSwap<ServerConfig>>,
    cert_store: Arc<CertStore>,
    ssl_config: SslConfig,
    challenge_manager: Option<Arc<ChallengeManager>>,
}

impl TlsConfigManager {
    pub async fn new(
        ssl_config: SslConfig,
        cert_store: Arc<CertStore>,
        challenge_manager: Option<Arc<ChallengeManager>>,
    ) -> Result<Self> {
        let tls_config =
            build_tls_config(&ssl_config, &cert_store, challenge_manager.clone()).await?;

        Ok(Self {
            config: Arc::new(ArcSwap::new(Arc::new(tls_config))),
            cert_store,
            ssl_config,
            challenge_manager,
        })
    }

    pub fn get_config(&self) -> Arc<ServerConfig> {
        self.config.load_full()
    }

    pub fn config_arc_swap(&self) -> Arc<ArcSwap<ServerConfig>> {
        self.config.clone()
    }

    pub async fn reload(&self) -> Result<()> {
        info!("Reloading TLS certificates...");

        let new_config = build_tls_config(
            &self.ssl_config,
            &self.cert_store,
            self.challenge_manager.clone(),
        )
        .await?;

        self.config.store(Arc::new(new_config));

        info!("TLS certificates reloaded successfully");
        Ok(())
    }

    /// Returns true if any domain cert is within 7 days of expiry, or was renewed within the last
    /// hour (so the hot-swap picks up the new cert promptly).
    pub async fn check_reload_needed(&self) -> bool {
        for domain in &self.ssl_config.domains {
            match self.cert_store.needs_renewal(domain, 7).await {
                Ok(true) => {
                    debug!("Certificate for {} needs renewal, reload recommended", domain);
                    return true;
                }
                Ok(false) => {
                    if let Ok(Some(cert)) = self.cert_store.load_certificate(domain).await {
                        if let Some(renewed_at) = cert.metadata.renewed_at {
                            let now = chrono::Utc::now();
                            if now.signed_duration_since(renewed_at).num_minutes() < 60 {
                                debug!("Certificate for {} was recently renewed, reload needed", domain);
                                return true;
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("Error checking renewal status for {}: {}", domain, e);
                }
            }
        }
        false
    }
}

async fn build_tls_config(
    ssl_config: &SslConfig,
    cert_store: &CertStore,
    challenge_manager: Option<Arc<ChallengeManager>>,
) -> Result<ServerConfig> {
    let mut loaded_certs = Vec::new();

    for domain in &ssl_config.domains {
        match cert_store.load_certificate(domain).await {
            Ok(Some(cert)) => {
                debug!("Loaded certificate for domain: {}", domain);
                loaded_certs.push((domain.clone(), cert));
            }
            Ok(None) => {
                warn!("No certificate found for domain: {}", domain);
            }
            Err(e) => {
                error!("Failed to load certificate for {}: {}", domain, e);
            }
        }
    }

    // No certs yet — generate self-signed so the server can start; ACME will replace it.
    let resolver: Arc<dyn ResolvesServerCert> = if loaded_certs.is_empty() {
        warn!("No certificates loaded, generating self-signed certificate");
        let primary_domain = ssl_config.primary_domain().unwrap_or("localhost");
        let cert = generate_self_signed_cert(primary_domain)?;
        Arc::new(StaticCertResolver {
            cert: Arc::new(cert),
        })
    } else {
        let primary = &loaded_certs[0];
        let cert_key = parse_certificate(&primary.1)?;
        let resolver = MultiDomainResolver::new(Arc::new(cert_key));

        for (domain, stored_cert) in &loaded_certs {
            let cert_key = parse_certificate(stored_cert)?;
            resolver.add_domain(domain.clone(), Arc::new(cert_key));
        }

        Arc::new(resolver)
    };

    let final_resolver: Arc<dyn ResolvesServerCert> = match challenge_manager {
        Some(cm) => Arc::new(AlpnChallengeResolver::new(cm, resolver)),
        None => resolver,
    };

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(final_resolver);

    // acme-tls/1 allows TLS-ALPN-01 challenges on the same port as HTTPS.
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"acme-tls/1".to_vec()];

    Ok(config)
}

fn parse_certificate(stored: &StoredCertificate) -> Result<CertifiedKey> {
    let cert_chain = parse_pem_certs(&stored.cert_pem)?;
    let private_key = parse_pem_private_key(&stored.privkey_pem)?;
    let signing_key = rustls::crypto::aws_lc_rs::sign::any_supported_type(&private_key)
        .map_err(|e| anyhow::anyhow!("Failed to create signing key: {}", e))?;

    Ok(CertifiedKey::new(cert_chain, signing_key))
}

fn parse_pem_certs(pem: &str) -> Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(pem.as_bytes());
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse PEM certificates")?;

    Ok(certs)
}

fn parse_pem_private_key(pem: &str) -> Result<PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(pem.as_bytes());
    let keys = rustls_pemfile::private_key(&mut reader)
        .context("Failed to parse PEM private key")?
        .ok_or_else(|| anyhow::anyhow!("No private key found in PEM"))?;

    Ok(keys)
}

fn generate_self_signed_cert(domain: &str) -> Result<CertifiedKey> {
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};

    let mut params = CertificateParams::default();
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::CommonName, domain.to_string());
    params.subject_alt_names = vec![rcgen::SanType::DnsName(domain.try_into().unwrap())];

    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after = time::OffsetDateTime::now_utc() + time::Duration::days(30);

    let key_pair = KeyPair::generate().context("Failed to generate key pair")?;
    let cert = params
        .self_signed(&key_pair)
        .context("Failed to generate self-signed certificate")?;

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
        key_pair.serialize_der(),
    ));

    let signing_key = rustls::crypto::aws_lc_rs::sign::any_supported_type(&key_der)
        .map_err(|e| anyhow::anyhow!("Failed to create signing key: {}", e))?;

    Ok(CertifiedKey::new(vec![cert_der], signing_key))
}

pub fn create_rustls_config(
    tls_manager: &TlsConfigManager,
) -> axum_server::tls_rustls::RustlsConfig {
    axum_server::tls_rustls::RustlsConfig::from_config(tls_manager.get_config())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed() {
        let cert = generate_self_signed_cert("test.example.com").unwrap();
        assert!(!cert.cert.is_empty());
    }
}
