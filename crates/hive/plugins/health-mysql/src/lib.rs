//! MySQL Health Check Plugin for Hive
//!
//! Checks service health by connecting to MySQL.
//!
//! ## Configuration
//!
//! ```yaml
//! healthcheck:
//!   type: mysql
//!   mysql:
//!     port: "{{runtime.port.db}}"
//!     host: localhost
//!     user: root
//!     database: mydb
//!     timeout: 5s
//! ```

use async_trait::async_trait;
use lib_plugin_abi_v3::{
    health::{HealthCheck, HealthResult},
    runner::RuntimeContext,
    utils::parse_duration,
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_HEALTH_CHECK,
};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::Row;
use std::time::{Duration, Instant};
use tracing::{debug, trace, warn};

pub struct MysqlHealthPlugin {
    default_timeout: Duration,
}

impl Default for MysqlHealthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MysqlHealthPlugin {
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(5),
        }
    }
}

#[async_trait]
impl Plugin for MysqlHealthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.health.mysql".to_string(),
            name: "MySQL Health Check".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("MySQL connection health check".to_string()),
            category: Some(PluginCategory::Health),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(timeout_str) = ctx.config.get("timeout").and_then(|v| v.as_str()) {
            if let Some(duration) = parse_duration(timeout_str) {
                self.default_timeout = duration;
            }
        }
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_HEALTH_CHECK]
    }
}

#[async_trait]
impl HealthCheck for MysqlHealthPlugin {
    async fn check(&self, config: &serde_json::Value, ctx: &RuntimeContext) -> PluginResult<HealthResult> {
        let mysql_config: MysqlHealthConfig = config
            .get("mysql")
            .ok_or_else(|| lib_plugin_abi_v3::PluginError::Config("Missing 'mysql' configuration".to_string()))?
            .clone()
            .try_into()
            .map_err(|e| lib_plugin_abi_v3::PluginError::Config(format!("Invalid MySQL health config: {}", e)))?;

        let port_str = ctx.interpolate(&mysql_config.port)?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| lib_plugin_abi_v3::PluginError::Config(format!("Invalid port: {}", port_str)))?;

        let host = mysql_config.host.as_deref().unwrap_or("127.0.0.1");
        let user = mysql_config.user.as_deref().unwrap_or("root");
        let database = mysql_config.database.as_deref().unwrap_or("mysql");

        let url = if let Some(ref password) = mysql_config.password {
            format!(
                "mysql://{}:{}@{}:{}/{}",
                user, password, host, port, database
            )
        } else {
            format!("mysql://{}@{}:{}/{}", user, host, port, database)
        };

        let check_timeout = mysql_config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(self.default_timeout);

        debug!(
            host = %host,
            port = port,
            database = %database,
            user = %user,
            timeout_ms = check_timeout.as_millis() as u64,
            "Starting MySQL health check"
        );

        let start = Instant::now();

        let result = tokio::time::timeout(
            check_timeout,
            MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(check_timeout)
                .connect(&url),
        )
        .await;

        let elapsed = start.elapsed();

        match result {
            Ok(Ok(pool)) => {
                let query_result = sqlx::query("SELECT 1 as health")
                    .fetch_one(&pool)
                    .await;

                match query_result {
                    Ok(row) => {
                        let _: i32 = row.get("health");
                        trace!(
                            host = %host, port = port, database = %database,
                            elapsed_ms = elapsed.as_millis() as u64,
                            "MySQL health check passed"
                        );
                        Ok(HealthResult::healthy()
                            .with_response_time(elapsed.as_millis() as u64)
                            .with_message(format!(
                                "MySQL connected: {}:{}/{}",
                                host, port, database
                            )))
                    }
                    Err(e) => {
                        warn!(host = %host, port = port, error = %e, "MySQL health check query failed");
                        Ok(HealthResult::unhealthy(format!("Query failed: {}", e))
                            .with_response_time(elapsed.as_millis() as u64))
                    }
                }
            }
            Ok(Err(e)) => {
                warn!(host = %host, port = port, error = %e, "MySQL health check connection failed");
                Ok(HealthResult::unhealthy(format!("Connection failed: {}", e))
                    .with_response_time(elapsed.as_millis() as u64))
            }
            Err(_) => {
                warn!(host = %host, port = port, timeout_s = check_timeout.as_secs(), "MySQL health check timed out");
                Ok(HealthResult::unhealthy(format!(
                    "Connection timed out after {}s",
                    check_timeout.as_secs()
                )))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MysqlHealthConfig {
    /// Port (can be interpolated with {{runtime.port.X}})
    pub port: String,
    /// Default: 127.0.0.1
    pub host: Option<String>,
    /// Default: root
    pub user: Option<String>,
    pub password: Option<String>,
    /// Default: mysql
    pub database: Option<String>,
    pub timeout: Option<String>,
}

impl TryFrom<serde_json::Value> for MysqlHealthConfig {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(MysqlHealthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = MysqlHealthPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.health.mysql");
        assert_eq!(meta.name, "MySQL Health Check");
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "port": "3306",
            "host": "localhost",
            "user": "root",
            "database": "mydb"
        });

        let mysql_config: MysqlHealthConfig = config.try_into().unwrap();
        assert_eq!(mysql_config.port, "3306");
        assert_eq!(mysql_config.host, Some("localhost".to_string()));
    }
}
