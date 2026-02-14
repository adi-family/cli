//! Environment variable configuration for logging.
//!
//! Standard environment variables:
//! - `LOGGING_URL` - URL of the logging service (e.g., "http://localhost:8040")
//! - `LOGGING_ENABLED` - Whether to send logs to service ("true"/"false", default: "true")
//!
//! If `LOGGING_URL` is not set or `LOGGING_ENABLED` is "false", the client
//! operates in console-only mode.

use lib_env_parse::{env_bool_default_true, env_opt, env_vars};

use crate::{LoggingClient, LoggingClientConfig};

env_vars! {
    LoggingUrl     => "LOGGING_URL",
    LoggingEnabled => "LOGGING_ENABLED",
    Hostname       => "HOSTNAME",
    Environment    => "ENVIRONMENT",
    ServiceVersion => "SERVICE_VERSION",
    DatabaseUrl    => "DATABASE_URL",
}

/// Standard environment variable name for logging service URL.
pub const LOGGING_URL_ENV: &str = EnvVar::LoggingUrl.as_str();

/// Standard environment variable name to enable/disable logging service delivery.
pub const LOGGING_ENABLED_ENV: &str = EnvVar::LoggingEnabled.as_str();

/// Hostname of the service instance ($HOSTNAME).
pub fn hostname() -> Option<String> {
    env_opt(EnvVar::Hostname.as_str())
}

/// Environment name ($ENVIRONMENT).
pub fn environment() -> Option<String> {
    env_opt(EnvVar::Environment.as_str())
}

/// Service version string ($SERVICE_VERSION).
pub fn service_version() -> Option<String> {
    env_opt(EnvVar::ServiceVersion.as_str())
}

/// Database URL ($DATABASE_URL).
pub fn database_url() -> Option<String> {
    env_opt(EnvVar::DatabaseUrl.as_str())
}

/// Create a logging client from environment variables.
///
/// Uses these environment variables:
/// - `LOGGING_URL` - URL of logging service (required for service delivery)
/// - `LOGGING_ENABLED` - Set to "false" to disable service delivery (default: "true")
///
/// # Arguments
/// * `service` - Name of the service using this client
///
/// # Returns
/// A `LoggingClient` configured based on environment:
/// - If `LOGGING_URL` is set and `LOGGING_ENABLED` != "false": full mode
/// - Otherwise: console-only mode (logs still appear via tracing)
///
/// # Example
///
/// ```rust,no_run
/// use lib_logging_core::from_env;
///
/// let client = from_env("my-service");
/// ```
pub fn from_env(service: impl Into<String>) -> LoggingClient {
    let service = service.into();

    let enabled = env_bool_default_true(EnvVar::LoggingEnabled.as_str());
    let logging_url = env_opt(EnvVar::LoggingUrl.as_str());

    match (enabled, logging_url) {
        (true, Some(url)) if !url.is_empty() => {
            tracing::info!(
                target: "lib_logging_core",
                "Logging client initialized: service={}, url={}",
                service,
                url
            );
            LoggingClient::new(url, service)
        }
        _ => {
            tracing::info!(
                target: "lib_logging_core",
                "Logging client initialized: service={}, mode=console-only",
                service
            );
            LoggingClient::console_only(service)
        }
    }
}

/// Create a logging client from environment with custom config overrides.
///
/// # Arguments
/// * `service` - Name of the service
/// * `configure` - Function to customize the config
///
/// # Example
///
/// ```rust,no_run
/// use lib_logging_core::from_env_with;
///
/// let client = from_env_with("my-service", |config| {
///     config.with_batch_size(50).with_flush_interval(10)
/// });
/// ```
pub fn from_env_with<F>(service: impl Into<String>, configure: F) -> LoggingClient
where
    F: FnOnce(LoggingClientConfig) -> LoggingClientConfig,
{
    let service = service.into();

    let enabled = env_bool_default_true(EnvVar::LoggingEnabled.as_str());
    let logging_url = env_opt(EnvVar::LoggingUrl.as_str());

    let config = match (enabled, logging_url) {
        (true, Some(url)) if !url.is_empty() => LoggingClientConfig::new(url, &service),
        _ => LoggingClientConfig::console_only(&service),
    };

    LoggingClient::with_config(configure(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_from_env_no_url() {
        // Without LOGGING_URL, should create console-only client
        std::env::remove_var(LOGGING_URL_ENV);
        std::env::remove_var(LOGGING_ENABLED_ENV);

        // This won't panic - creates console-only client
        let _client = from_env("test-service");
    }
}
