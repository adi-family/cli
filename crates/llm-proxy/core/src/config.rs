//! Configuration for adi-llm-proxy service.

use anyhow::{Context, Result};
use std::env;

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
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8024".to_string())
                .parse()
                .context("Invalid PORT")?,
            database_url: env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("Invalid DATABASE_MAX_CONNECTIONS")?,
            jwt_secret: env::var("JWT_SECRET").context("JWT_SECRET is required")?,
            encryption_key: env::var("ENCRYPTION_KEY")
                .context("ENCRYPTION_KEY is required (64-char hex)")?,
            analytics_url: env::var("ANALYTICS_URL")
                .unwrap_or_else(|_| "http://localhost:8094".to_string()),
            upstream_timeout_secs: env::var("UPSTREAM_TIMEOUT_SECS")
                .unwrap_or_else(|_| "120".to_string())
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
