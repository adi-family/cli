//! HTTPS server with automatic certificate management
//!
//! Provides the main entry point for running Hive with SSL/TLS support.

use crate::acme_client::AcmeClient;
use crate::cert_store::CertStore;
use crate::challenge::ChallengeManager;
use crate::config::SslConfig;
use crate::tls_config::{create_rustls_config, TlsConfigManager};
use anyhow::{Context, Result};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{error, info};

pub struct HttpsServer {
    config: SslConfig,
    cert_store: Arc<CertStore>,
    challenge_manager: Arc<ChallengeManager>,
    tls_manager: Arc<TlsConfigManager>,
    acme_client: Arc<AcmeClient>,
    app_router: Router,
}

impl HttpsServer {
    pub async fn new(config: SslConfig, app_router: Router) -> Result<Self> {
        config.validate()?;

        let cert_store = Arc::new(CertStore::new(&config.cert_dir));
        cert_store.ensure_dir().await?;

        let challenge_manager = Arc::new(ChallengeManager::new(
            config.redirect_http_to_https,
            config.https_port,
        ));

        let tls_manager = Arc::new(
            TlsConfigManager::new(
                config.clone(),
                cert_store.clone(),
                Some(challenge_manager.clone()),
            )
            .await?,
        );

        let acme_client = Arc::new(AcmeClient::new(config.clone(), cert_store.clone()).await?);

        Ok(Self {
            config,
            cert_store,
            challenge_manager,
            tls_manager,
            acme_client,
            app_router,
        })
    }

    pub async fn ensure_certificates(&self) -> Result<()> {
        info!("Checking certificate status...");

        if self.acme_client.needs_renewal().await? {
            info!("Certificate renewal needed, starting ACME process...");

            self.acme_client
                .obtain_certificate(&self.challenge_manager)
                .await?;

            self.tls_manager.reload().await?;

            info!("Certificates obtained and loaded successfully");
        } else {
            info!("Certificates are valid, no renewal needed");
        }

        Ok(())
    }

    pub async fn start(self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let http_handle = self.start_http_challenge_server(shutdown_rx.clone());

        let renewal_handle = if self.config.auto_renew {
            Some(self.start_renewal_task(shutdown_rx.clone()))
        } else {
            None
        };

        let https_result = self.start_https_server().await;

        let _ = shutdown_tx.send(true);

        if let Some(handle) = renewal_handle {
            handle.abort();
        }
        http_handle.abort();

        https_result
    }

    fn start_http_challenge_server(
        &self,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> tokio::task::JoinHandle<()> {
        let challenge_manager = self.challenge_manager.clone();
        let port = self.config.http_challenge_port;

        tokio::spawn(async move {
            let app = challenge_manager.http_router();
            let addr = SocketAddr::from(([0, 0, 0, 0], port));

            info!("HTTP challenge server listening on {}", addr);

            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to bind HTTP challenge server: {}", e);
                    return;
                }
            };

            tokio::select! {
                result = axum::serve(listener, app) => {
                    if let Err(e) = result {
                        error!("HTTP challenge server error: {}", e);
                    }
                }
                _ = shutdown_rx.changed() => {
                    info!("HTTP challenge server shutting down");
                }
            }
        })
    }

    async fn start_https_server(&self) -> Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.https_port));
        info!("HTTPS server listening on {}", addr);

        let tls_config = create_rustls_config(&self.tls_manager);

        axum_server::bind_rustls(addr, tls_config)
            .serve(self.app_router.clone().into_make_service())
            .await
            .context("HTTPS server error")?;

        Ok(())
    }

    fn start_renewal_task(
        &self,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> tokio::task::JoinHandle<()> {
        let acme_client = self.acme_client.clone();
        let challenge_manager = self.challenge_manager.clone();
        let tls_manager = self.tls_manager.clone();
        let renew_before_days = self.config.renew_before_days;

        tokio::spawn(async move {
            let interval = Duration::from_secs(12 * 60 * 60);

            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        info!("Running periodic certificate renewal check...");

                        match acme_client.needs_renewal().await {
                            Ok(true) => {
                                info!("Certificate renewal needed (expires within {} days)", renew_before_days);

                                match acme_client.obtain_certificate(&challenge_manager).await {
                                    Ok(_) => {
                                        if let Err(e) = tls_manager.reload().await {
                                            error!("Failed to reload TLS config after renewal: {}", e);
                                        } else {
                                            info!("Certificate renewed and loaded successfully");
                                        }
                                    }
                                    Err(e) => {
                                        error!("Certificate renewal failed: {}", e);
                                    }
                                }
                            }
                            Ok(false) => {
                                info!("Certificate still valid, no renewal needed");
                            }
                            Err(e) => {
                                error!("Failed to check certificate renewal status: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        info!("Certificate renewal task shutting down");
                        break;
                    }
                }
            }
        })
    }

    pub fn cert_store(&self) -> &Arc<CertStore> {
        &self.cert_store
    }

    pub fn tls_manager(&self) -> &Arc<TlsConfigManager> {
        &self.tls_manager
    }

    pub async fn force_renew(&self) -> Result<()> {
        info!("Forcing certificate renewal...");

        self.acme_client
            .obtain_certificate(&self.challenge_manager)
            .await?;

        self.tls_manager.reload().await?;

        info!("Certificate force-renewed successfully");
        Ok(())
    }
}

pub async fn start_https_server(config: SslConfig, app_router: Router) -> Result<()> {
    let server = HttpsServer::new(config, app_router).await?;
    server.ensure_certificates().await?;
    server.start().await
}

pub struct SslManager {
    config: SslConfig,
    cert_store: Arc<CertStore>,
    challenge_manager: Arc<ChallengeManager>,
    tls_manager: Arc<TlsConfigManager>,
    acme_client: Arc<AcmeClient>,
}

impl SslManager {
    pub async fn new(config: SslConfig) -> Result<Self> {
        config.validate()?;

        let cert_store = Arc::new(CertStore::new(&config.cert_dir));
        cert_store.ensure_dir().await?;

        let challenge_manager = Arc::new(ChallengeManager::new(
            config.redirect_http_to_https,
            config.https_port,
        ));

        let tls_manager = Arc::new(
            TlsConfigManager::new(
                config.clone(),
                cert_store.clone(),
                Some(challenge_manager.clone()),
            )
            .await?,
        );

        let acme_client = Arc::new(AcmeClient::new(config.clone(), cert_store.clone()).await?);

        Ok(Self {
            config,
            cert_store,
            challenge_manager,
            tls_manager,
            acme_client,
        })
    }

    pub fn tls_config(&self) -> axum_server::tls_rustls::RustlsConfig {
        create_rustls_config(&self.tls_manager)
    }

    pub fn http_router(&self) -> Router {
        self.challenge_manager.clone().http_router()
    }

    pub async fn ensure_certificates(&self) -> Result<()> {
        if self.acme_client.needs_renewal().await? {
            self.acme_client
                .obtain_certificate(&self.challenge_manager)
                .await?;
            self.tls_manager.reload().await?;
        }
        Ok(())
    }

    pub async fn force_renew(&self) -> Result<()> {
        self.acme_client
            .obtain_certificate(&self.challenge_manager)
            .await?;
        self.tls_manager.reload().await?;
        Ok(())
    }

    pub async fn reload_tls(&self) -> Result<()> {
        self.tls_manager.reload().await
    }

    pub async fn get_status(&self) -> Result<Vec<crate::cert_store::CertificateStatus>> {
        let mut statuses = Vec::new();
        for domain in &self.config.domains {
            let status = self.cert_store.get_status(domain).await?;
            statuses.push(status);
        }
        Ok(statuses)
    }

    pub fn challenge_manager(&self) -> &Arc<ChallengeManager> {
        &self.challenge_manager
    }

    pub fn config(&self) -> &SslConfig {
        &self.config
    }

    /// Issue a certificate for arbitrary domains (on-demand issuance).
    ///
    /// Creates a temporary ACME client with the specified domains and issues a certificate.
    /// The certificate is stored and can be loaded later.
    pub async fn issue_certificate_for_domains(
        &self,
        domains: &[String],
        email: &str,
        staging: bool,
        challenge_type: Option<&str>,
    ) -> Result<CertificateIssueResult> {
        use crate::config::AcmeChallengeType;

        if domains.is_empty() {
            anyhow::bail!("At least one domain is required");
        }

        if email.is_empty() || !email.contains('@') {
            anyhow::bail!("Valid email is required for ACME registration");
        }

        info!(
            "Issuing certificate for domains: {:?} (staging: {})",
            domains, staging
        );

        let mut temp_config = self.config.clone();
        temp_config.domains = domains.to_vec();
        temp_config.email = email.to_string();
        temp_config.staging = staging;
        temp_config.challenge_type = challenge_type
            .map(|s| s.parse())
            .transpose()?
            .unwrap_or(AcmeChallengeType::Auto);

        let temp_acme = AcmeClient::new(temp_config, self.cert_store.clone()).await?;

        temp_acme
            .obtain_certificate(&self.challenge_manager)
            .await?;

        let status = self.cert_store.get_status(&domains[0]).await?;

        info!(
            "Certificate issued successfully for {} (expires: {:?})",
            domains[0], status.expires_at
        );

        Ok(CertificateIssueResult {
            domain: domains[0].clone(),
            expires_at: status.expires_at,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CertificateIssueResult {
    pub domain: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}
