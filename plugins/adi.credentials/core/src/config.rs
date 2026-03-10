use anyhow::{Context, Result};
use lib_env_parse::{env_opt, env_or, env_vars};

env_vars! {
    Host                   => "HOST",
    Port                   => "PORT",
    DatabaseUrl            => "DATABASE_URL",
    DatabaseMaxConnections => "DATABASE_MAX_CONNECTIONS",
    JwtSecret              => "JWT_SECRET",
    EncryptionKey          => "ENCRYPTION_KEY",
}

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub database_max_connections: u32,
    pub jwt_secret: String,
    pub encryption_key: String, // 64-char hex (32 bytes)
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: env_or(EnvVar::Host.as_str(), "0.0.0.0"),
            port: env_or(EnvVar::Port.as_str(), "8032")
                .parse()
                .context("Invalid PORT")?,
            database_url: env_opt(EnvVar::DatabaseUrl.as_str()).context("DATABASE_URL is required")?,
            database_max_connections: env_or(EnvVar::DatabaseMaxConnections.as_str(), "10")
                .parse()
                .context("Invalid DATABASE_MAX_CONNECTIONS")?,
            jwt_secret: env_opt(EnvVar::JwtSecret.as_str()).context("JWT_SECRET is required")?,
            encryption_key: env_opt(EnvVar::EncryptionKey.as_str())
                .context("ENCRYPTION_KEY is required (64-char hex for 32 bytes)")?,
        })
    }
}