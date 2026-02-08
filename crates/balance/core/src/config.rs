use anyhow::{Context, Result};
use std::env;

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub database_max_connections: u32,
    pub jwt_secret: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8030".to_string())
                .parse()
                .context("Invalid PORT")?,
            database_url: env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("Invalid DATABASE_MAX_CONNECTIONS")?,
            jwt_secret: env::var("JWT_SECRET").context("JWT_SECRET is required")?,
        })
    }
}
