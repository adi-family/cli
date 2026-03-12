//! SSL configuration management

use anyhow::{Context, Result};
use lib_env_parse::{env_require, env_vars, is_truthy};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

env_vars! {
    SslEnabled         => "SSL_ENABLED",
    SslDomains         => "SSL_DOMAINS",
    SslEmail           => "SSL_EMAIL",
    SslCertDir         => "SSL_CERT_DIR",
    SslHttpsPort       => "SSL_HTTPS_PORT",
    SslChallengePort   => "SSL_CHALLENGE_PORT",
    SslChallengeType   => "SSL_CHALLENGE_TYPE",
    SslStaging         => "SSL_STAGING",
    SslAutoRenew       => "SSL_AUTO_RENEW",
    SslRenewBeforeDays => "SSL_RENEW_BEFORE_DAYS",
    SslRedirectHttp    => "SSL_REDIRECT_HTTP",
}

/// ACME challenge type for domain validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AcmeChallengeType {
    /// HTTP-01: Serve challenge on port 80
    Http01,
    /// TLS-ALPN-01: Serve challenge on port 443 via ALPN
    TlsAlpn01,
    /// Auto: Try TLS-ALPN-01 first, fallback to HTTP-01
    #[default]
    Auto,
}

impl std::str::FromStr for AcmeChallengeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "http01" | "http-01" | "http" => Ok(Self::Http01),
            "tls-alpn01" | "tls-alpn-01" | "tlsalpn01" | "alpn" => Ok(Self::TlsAlpn01),
            "auto" => Ok(Self::Auto),
            _ => anyhow::bail!(
                "Invalid challenge type: {}. Expected: http01, tls-alpn01, or auto",
                s
            ),
        }
    }
}

impl std::fmt::Display for AcmeChallengeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http01 => write!(f, "http-01"),
            Self::TlsAlpn01 => write!(f, "tls-alpn-01"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    pub enabled: bool,
    pub domains: Vec<String>,
    /// ACME account email (required by Let's Encrypt)
    pub email: String,
    pub cert_dir: PathBuf,
    pub https_port: u16,
    /// HTTP challenge port for HTTP-01 (default: 80)
    pub http_challenge_port: u16,
    pub challenge_type: AcmeChallengeType,
    /// Use Let's Encrypt staging environment for testing
    pub staging: bool,
    pub auto_renew: bool,
    /// Days before expiry to trigger renewal
    pub renew_before_days: u32,
    /// Redirect HTTP to HTTPS when HTTP-01 challenge is not in progress
    pub redirect_http_to_https: bool,
}

impl Default for SslConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            domains: Vec::new(),
            email: String::new(),
            cert_dir: PathBuf::from("/var/lib/hive/certs"),
            https_port: 443,
            http_challenge_port: 80,
            challenge_type: AcmeChallengeType::Auto,
            staging: false,
            auto_renew: true,
            renew_before_days: 30,
            redirect_http_to_https: true,
        }
    }
}

impl SslConfig {
    /// Load configuration from environment variables (all required).
    pub fn from_env() -> Result<Self> {
        let enabled = is_truthy(&env_require(EnvVar::SslEnabled.as_str())?);

        let domains_raw = env_require(EnvVar::SslDomains.as_str())?;
        let domains: Vec<String> = domains_raw
            .split(',')
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty())
            .collect();

        let email = env_require(EnvVar::SslEmail.as_str())?;

        let cert_dir = PathBuf::from(env_require(EnvVar::SslCertDir.as_str())?);

        let https_port = env_require(EnvVar::SslHttpsPort.as_str())?
            .parse::<u16>()
            .context("SSL_HTTPS_PORT must be a valid port number")?;

        let http_challenge_port = env_require(EnvVar::SslChallengePort.as_str())?
            .parse::<u16>()
            .context("SSL_CHALLENGE_PORT must be a valid port number")?;

        let challenge_type: AcmeChallengeType = env_require(EnvVar::SslChallengeType.as_str())?
            .parse()
            .context("Invalid SSL_CHALLENGE_TYPE")?;

        let staging = is_truthy(&env_require(EnvVar::SslStaging.as_str())?);
        let auto_renew = is_truthy(&env_require(EnvVar::SslAutoRenew.as_str())?);

        let renew_before_days = env_require(EnvVar::SslRenewBeforeDays.as_str())?
            .parse::<u32>()
            .context("SSL_RENEW_BEFORE_DAYS must be a valid number")?;

        let redirect_http_to_https = is_truthy(&env_require(EnvVar::SslRedirectHttp.as_str())?);

        Ok(Self {
            enabled,
            domains,
            email,
            cert_dir,
            https_port,
            http_challenge_port,
            challenge_type,
            staging,
            auto_renew,
            renew_before_days,
            redirect_http_to_https,
        })
    }

    /// Validate configuration
    ///
    /// Note: SSL_DOMAINS and SSL_EMAIL are only required if you want to issue
    /// certificates at startup. For WebSocket-only on-demand issuance, these
    /// can be empty - the domain and email will come from the RequestCertificate message.
    pub fn validate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // If domains are configured, validate them
        if !self.domains.is_empty() {
            // Email is required when pre-configuring domains
            if self.email.is_empty() {
                anyhow::bail!("SSL_EMAIL must be set when SSL_DOMAINS is configured");
            }

            // Validate email format (basic check)
            if !self.email.contains('@') || !self.email.contains('.') {
                anyhow::bail!("SSL_EMAIL must be a valid email address");
            }

            // Validate domains (basic check)
            for domain in &self.domains {
                if domain.is_empty() || domain.starts_with('.') || domain.ends_with('.') {
                    anyhow::bail!("Invalid domain: {}", domain);
                }
            }
        }

        // If no domains configured, SSL is enabled for on-demand WebSocket issuance only
        // No validation needed - domains/email come from RequestCertificate message

        Ok(())
    }

    pub fn has_preconfigured_domains(&self) -> bool {
        !self.domains.is_empty()
    }

    pub fn primary_domain(&self) -> Option<&str> {
        self.domains.first().map(|s| s.as_str())
    }

    pub fn supports_challenge(&self, challenge_type: AcmeChallengeType) -> bool {
        match self.challenge_type {
            AcmeChallengeType::Auto => true,
            other => other == challenge_type,
        }
    }

    pub fn acme_directory_url(&self) -> &'static str {
        if self.staging {
            "https://acme-staging-v02.api.letsencrypt.org/directory"
        } else {
            "https://acme-v02.api.letsencrypt.org/directory"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_type_parse() {
        assert_eq!(
            "http01".parse::<AcmeChallengeType>().unwrap(),
            AcmeChallengeType::Http01
        );
        assert_eq!(
            "http-01".parse::<AcmeChallengeType>().unwrap(),
            AcmeChallengeType::Http01
        );
        assert_eq!(
            "tls-alpn01".parse::<AcmeChallengeType>().unwrap(),
            AcmeChallengeType::TlsAlpn01
        );
        assert_eq!(
            "auto".parse::<AcmeChallengeType>().unwrap(),
            AcmeChallengeType::Auto
        );
    }

    #[test]
    fn test_config_validation() {
        let mut config = SslConfig::default();

        // Disabled config should pass
        assert!(config.validate().is_ok());

        // Enabled without domains is valid (on-demand WebSocket issuance)
        config.enabled = true;
        assert!(config.validate().is_ok());

        // With domains but no email should fail
        config.domains = vec!["example.com".to_string()];
        assert!(config.validate().is_err());

        // With valid email should pass
        config.email = "admin@example.com".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_acme_directory_url() {
        let mut config = SslConfig::default();

        assert!(config
            .acme_directory_url()
            .contains("acme-v02.api.letsencrypt.org"));

        config.staging = true;
        assert!(config.acme_directory_url().contains("staging"));
    }
}
