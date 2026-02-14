//! Configuration for adi-llm-proxy service.

use anyhow::{Context, Result};
use lib_env_parse::{env_opt, env_or, env_vars};

env_vars! {
    Host                   => "HOST",
    Port                   => "PORT",
    DatabaseUrl            => "DATABASE_URL",
    DatabaseMaxConnections => "DATABASE_MAX_CONNECTIONS",
    JwtSecret              => "JWT_SECRET",
    EncryptionKey          => "ENCRYPTION_KEY",
    AnalyticsUrl           => "ANALYTICS_URL",
    UpstreamTimeoutSecs    => "UPSTREAM_TIMEOUT_SECS",
}

/// Service configuration loaded from environment variables.
#[derive(Clone)]
pub struct Config {
    /// Host to bind to (default: 0.0.0.0)
    pub host: String,
    /// Port to listen on (default: 8024)
    pub port: u16,
    /// PostgreSQL database URL
    pub database_url: String,
    /// Maximum database connections (default: 10)
    pub database_max_connections: u32,
    /// JWT secret for platform token validation (shared with adi-auth)
    pub jwt_secret: String,
    /// 32-byte hex-encoded encryption key for API keys
    pub encryption_key: String,
    /// Analytics ingestion service URL (default: http://localhost:8094)
    pub analytics_url: String,
    /// Default timeout for upstream requests in seconds (default: 120)
    pub upstream_timeout_secs: u64,
}

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: env_or(EnvVar::Host.as_str(), "0.0.0.0"),
            port: env_or(EnvVar::Port.as_str(), "8024")
                .parse()
                .context("Invalid PORT")?,
            database_url: env_opt(EnvVar::DatabaseUrl.as_str()).context("DATABASE_URL is required")?,
            database_max_connections: env_or(EnvVar::DatabaseMaxConnections.as_str(), "10")
                .parse()
                .context("Invalid DATABASE_MAX_CONNECTIONS")?,
            jwt_secret: env_opt(EnvVar::JwtSecret.as_str()).context("JWT_SECRET is required")?,
            encryption_key: env_opt(EnvVar::EncryptionKey.as_str())
                .context("ENCRYPTION_KEY is required (64-char hex)")?,
            analytics_url: env_or(EnvVar::AnalyticsUrl.as_str(), "http://localhost:8094"),
            upstream_timeout_secs: env_or(EnvVar::UpstreamTimeoutSecs.as_str(), "120")
                .parse()
                .context("Invalid UPSTREAM_TIMEOUT_SECS")?,
        })
    }

    /// Get encryption key as bytes.
    pub fn encryption_key_bytes(&self) -> Result<[u8; 32]> {
        let bytes =
            hex::decode(&self.encryption_key).context("ENCRYPTION_KEY must be valid hex")?;
        bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("ENCRYPTION_KEY must be exactly 32 bytes (64 hex chars)"))
    }
}
