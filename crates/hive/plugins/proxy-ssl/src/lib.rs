//! ADI Hive SSL Module
//!
//! Provides automatic SSL/TLS certificate management using Let's Encrypt (ACME protocol).
//!
//! Features:
//! - Automatic certificate issuance and renewal via Let's Encrypt
//! - HTTP-01 and TLS-ALPN-01 challenge support
//! - Certificate persistence to filesystem
//! - Hot-reload of certificates without restart
//! - Staging environment support for testing
//!
//! # Usage
//!
//! ```rust,ignore
//! use hive_ssl::{SslConfig, SslManager};
//!
//! let config = SslConfig::from_env()?;
//! let ssl_manager = SslManager::new(config).await?;
//!
//! // Obtain initial certificates
//! ssl_manager.ensure_certificates().await?;
//!
//! // Get TLS config for HTTPS server
//! let tls_config = ssl_manager.tls_config();
//!
//! // Start renewal background task
//! ssl_manager.start_renewal_task();
//! ```

pub mod acme_client;
pub mod cert_store;
pub mod challenge;
pub mod config;
pub mod server;
pub mod tls_config;

pub use acme_client::AcmeClient;
pub use cert_store::CertStore;
pub use challenge::{ChallengeManager, ChallengeType};
pub use config::SslConfig;
pub use server::{start_https_server, CertificateIssueResult, HttpsServer, SslManager};
pub use tls_config::TlsConfigManager;

pub use instant_acme;
pub use rustls;
pub use tokio_rustls;
