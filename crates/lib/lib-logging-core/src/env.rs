//! Environment variable configuration for logging.
//!
//! Standard environment variables:
//! - `LOGGING_URL` - URL of the logging service (e.g., "http://localhost:8040")
//! - `LOGGING_ENABLED` - Whether to send logs to service ("true"/"false", default: "true")
//!
//! If `LOGGING_URL` is not set or `LOGGING_ENABLED` is "false", the client
//! operates in console-only mode.

use crate::{LoggingClient, LoggingClientConfig};

/// Standard environment variable for logging service URL.
pub const LOGGING_URL_ENV: &str = "LOGGING_URL";

/// Standard environment variable to enable/disable logging service delivery.
pub const LOGGING_ENABLED_ENV: &str = "LOGGING_ENABLED";

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

    // Check if logging to service is enabled
    let enabled = std::env::var(LOGGING_ENABLED_ENV)
        .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
        .unwrap_or(true);

    // Get logging URL
    let logging_url = std::env::var(LOGGING_URL_ENV).ok();

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

    let enabled = std::env::var(LOGGING_ENABLED_ENV)
        .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
        .unwrap_or(true);

    let logging_url = std::env::var(LOGGING_URL_ENV).ok();

    let config = match (enabled, logging_url) {
        (true, Some(url)) if !url.is_empty() => LoggingClientConfig::new(url, &service),
        _ => LoggingClientConfig::console_only(&service),
    };

    LoggingClient::with_config(configure(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env_no_url() {
        // Without LOGGING_URL, should create console-only client
        std::env::remove_var(LOGGING_URL_ENV);
        std::env::remove_var(LOGGING_ENABLED_ENV);

        // This won't panic
        let _client = from_env("test-service");
    }
}
