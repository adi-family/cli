use anyhow::{Context, Result};
use lib_env_parse::{env_opt, env_or, env_vars};

env_vars! {
    Host                   => "HOST",
    Port                   => "PORT",
    DatabaseUrl            => "DATABASE_URL",
    DatabaseMaxConnections => "DATABASE_MAX_CONNECTIONS",
    JwtSecret              => "JWT_SECRET",
    CorsOrigin             => "CORS_ORIGIN",
    CoinbaseApiKey         => "COINBASE_API_KEY",
    CoinbaseWebhookSecret  => "COINBASE_WEBHOOK_SECRET",
    PaddleApiKey           => "PADDLE_API_KEY",
    PaddleWebhookSecret    => "PADDLE_WEBHOOK_SECRET",
    PaddleEnvironment      => "PADDLE_ENVIRONMENT",
    BalanceApiUrl          => "BALANCE_API_URL",
}

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub database_max_connections: u32,
    pub jwt_secret: String,
    pub cors_origin: String,
    pub balance_api_url: Option<String>,
    pub coinbase: Option<CoinbaseConfig>,
    pub paddle: Option<PaddleConfig>,
}

#[derive(Clone)]
pub struct CoinbaseConfig {
    pub api_key: String,
    pub webhook_secret: String,
}

#[derive(Clone)]
pub struct PaddleConfig {
    pub api_key: String,
    pub webhook_secret: String,
    pub sandbox: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let coinbase = match (
            env_opt(EnvVar::CoinbaseApiKey.as_str()),
            env_opt(EnvVar::CoinbaseWebhookSecret.as_str()),
        ) {
            (Some(api_key), Some(webhook_secret)) => Some(CoinbaseConfig {
                api_key,
                webhook_secret,
            }),
            _ => None,
        };

        let paddle = match (
            env_opt(EnvVar::PaddleApiKey.as_str()),
            env_opt(EnvVar::PaddleWebhookSecret.as_str()),
        ) {
            (Some(api_key), Some(webhook_secret)) => {
                let env = env_or(EnvVar::PaddleEnvironment.as_str(), "sandbox");
                Some(PaddleConfig {
                    api_key,
                    webhook_secret,
                    sandbox: env != "production",
                })
            }
            _ => None,
        };

        Ok(Self {
            host: env_or(EnvVar::Host.as_str(), "0.0.0.0"),
            port: env_or(EnvVar::Port.as_str(), "8040")
                .parse()
                .context("Invalid PORT")?,
            database_url: env_opt(EnvVar::DatabaseUrl.as_str())
                .context("DATABASE_URL is required")?,
            database_max_connections: env_or(EnvVar::DatabaseMaxConnections.as_str(), "10")
                .parse()
                .context("Invalid DATABASE_MAX_CONNECTIONS")?,
            jwt_secret: env_opt(EnvVar::JwtSecret.as_str())
                .context("JWT_SECRET is required")?,
            cors_origin: env_or(EnvVar::CorsOrigin.as_str(), "http://localhost:8013"),
            balance_api_url: env_opt(EnvVar::BalanceApiUrl.as_str()),
            coinbase,
            paddle,
        })
    }
}
