//! Logging client that sends logs to the central logging service.
//!
//! This client is designed to be completely non-blocking:
//! - All log calls return immediately
//! - Logs are queued via unbounded channel (never blocks)
//! - Background task handles async delivery
//! - Console output (via tracing) provides backup logging

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::entry::{EnrichedLogEntry, LogEntryBuilder};
use crate::{LogLevel, TraceContext};

/// Configuration for the logging client.
#[derive(Clone)]
pub struct LoggingClientConfig {
    /// URL of the logging service
    pub logging_url: String,
    /// Service name for log attribution
    pub service: String,
    /// Batch size before auto-flush (default: 100)
    pub batch_size: usize,
    /// Flush interval in seconds (default: 5)
    pub flush_interval_secs: u64,
    /// HTTP timeout for sending batches (default: 10s)
    pub http_timeout_secs: u64,
    /// Whether to send to logging service (can be disabled, console-only mode)
    pub send_to_service: bool,
}

impl LoggingClientConfig {
    /// Create config with defaults.
    pub fn new(logging_url: impl Into<String>, service: impl Into<String>) -> Self {
        Self {
            logging_url: logging_url.into(),
            service: service.into(),
            batch_size: 100,
            flush_interval_secs: 5,
            http_timeout_secs: 10,
            send_to_service: true,
        }
    }

    /// Create config for console-only mode (no service delivery).
    pub fn console_only(service: impl Into<String>) -> Self {
        Self {
            logging_url: String::new(),
            service: service.into(),
            batch_size: 100,
            flush_interval_secs: 5,
            http_timeout_secs: 10,
            send_to_service: false,
        }
    }

    /// Set batch size.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set flush interval.
    pub fn with_flush_interval(mut self, secs: u64) -> Self {
        self.flush_interval_secs = secs;
        self
    }
}

/// Client for sending logs to the central logging service.
///
/// All operations are non-blocking:
/// - `log()` and related methods return immediately
/// - Uses unbounded channel (send never blocks)
/// - Background task handles async HTTP delivery
/// - Console logging (via tracing) always works as backup
///
/// # Example
///
/// ```rust,no_run
/// use lib_logging_core::{LoggingClient, TraceContext};
///
/// let client = LoggingClient::new("http://localhost:8040", "my-service");
/// let ctx = TraceContext::new();
///
/// // Non-blocking - returns immediately
/// client.info("Request received", &ctx)
///     .with_field("path", "/api/users")
///     .send();
/// ```
#[derive(Clone)]
pub struct LoggingClient {
    sender: mpsc::UnboundedSender<EnrichedLogEntry>,
    service: Arc<str>,
}

impl LoggingClient {
    /// Create a new logging client.
    ///
    /// # Arguments
    /// * `logging_url` - Base URL of logging service (e.g., "http://localhost:8040")
    /// * `service` - Name of the service using this client
    ///
    /// Logs are:
    /// 1. Immediately written to console (via tracing)
    /// 2. Batched and sent asynchronously to the logging service
    pub fn new(logging_url: impl Into<String>, service: impl Into<String>) -> Self {
        Self::with_config(LoggingClientConfig::new(logging_url, service))
    }

    /// Create a client with custom configuration.
    pub fn with_config(config: LoggingClientConfig) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let service: Arc<str> = config.service.clone().into();

        if config.send_to_service && !config.logging_url.is_empty() {
            // Spawn background sender task
            let http_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(config.http_timeout_secs))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let logging_url: Arc<str> = config.logging_url.into();
            let batch_size = config.batch_size;
            let flush_interval = Duration::from_secs(config.flush_interval_secs);

            tokio::spawn(async move {
                Self::send_loop(receiver, http_client, logging_url, batch_size, flush_interval).await;
            });
        } else {
            // Console-only mode: just drain the channel
            tokio::spawn(async move {
                let mut receiver = receiver;
                while receiver.recv().await.is_some() {
                    // Entries already logged to console in send(), just drain
                }
            });
        }

        Self { sender, service }
    }

    /// Create a console-only client (no service delivery).
    ///
    /// Useful for:
    /// - Testing
    /// - Development without logging service
    /// - Fallback when service URL is not configured
    pub fn console_only(service: impl Into<String>) -> Self {
        Self::with_config(LoggingClientConfig::console_only(service))
    }

    /// Create a no-op client for testing.
    #[deprecated(note = "Use console_only() instead")]
    pub fn noop(service: impl Into<String>) -> Self {
        Self::console_only(service)
    }

    /// Create a log entry builder.
    ///
    /// Call `.send()` on the builder to log. This is non-blocking.
    pub fn log(&self, level: LogLevel, message: impl Into<String>, ctx: &TraceContext) -> LogEntryBuilder {
        LogEntryBuilder::new(level, message, ctx, self.sender.clone(), self.service.clone())
    }

    /// Log at TRACE level.
    pub fn trace(&self, message: impl Into<String>, ctx: &TraceContext) -> LogEntryBuilder {
        self.log(LogLevel::Trace, message, ctx)
    }

    /// Log at DEBUG level.
    pub fn debug(&self, message: impl Into<String>, ctx: &TraceContext) -> LogEntryBuilder {
        self.log(LogLevel::Debug, message, ctx)
    }

    /// Log at INFO level.
    pub fn info(&self, message: impl Into<String>, ctx: &TraceContext) -> LogEntryBuilder {
        self.log(LogLevel::Info, message, ctx)
    }

    /// Log at NOTICE level.
    pub fn notice(&self, message: impl Into<String>, ctx: &TraceContext) -> LogEntryBuilder {
        self.log(LogLevel::Notice, message, ctx)
    }

    /// Log at WARN level.
    pub fn warn(&self, message: impl Into<String>, ctx: &TraceContext) -> LogEntryBuilder {
        self.log(LogLevel::Warn, message, ctx)
    }

    /// Log at ERROR level.
    pub fn error(&self, message: impl Into<String>, ctx: &TraceContext) -> LogEntryBuilder {
        self.log(LogLevel::Error, message, ctx)
    }

    /// Log at FATAL level.
    pub fn fatal(&self, message: impl Into<String>, ctx: &TraceContext) -> LogEntryBuilder {
        self.log(LogLevel::Fatal, message, ctx)
    }

    /// Background task that batches and sends logs.
    ///
    /// This runs in a separate task and never blocks the main application.
    async fn send_loop(
        mut receiver: mpsc::UnboundedReceiver<EnrichedLogEntry>,
        client: reqwest::Client,
        logging_url: Arc<str>,
        batch_size: usize,
        flush_interval: Duration,
    ) {
        let mut batch = Vec::with_capacity(batch_size);
        let mut interval = tokio::time::interval(flush_interval);

        // Skip first tick (happens immediately)
        interval.tick().await;

        loop {
            tokio::select! {
                // Biased towards receiving to prevent starvation
                biased;

                Some(entry) = receiver.recv() => {
                    batch.push(entry);

                    // Send if batch is full
                    if batch.len() >= batch_size {
                        Self::send_batch(&client, &logging_url, &mut batch).await;
                    }
                }

                _ = interval.tick() => {
                    if !batch.is_empty() {
                        Self::send_batch(&client, &logging_url, &mut batch).await;
                    }
                }

                else => {
                    // Channel closed, flush remaining and exit
                    if !batch.is_empty() {
                        Self::send_batch(&client, &logging_url, &mut batch).await;
                    }
                    break;
                }
            }
        }
    }

    /// Send a batch of logs to the logging service.
    ///
    /// This is fire-and-forget - failures are logged to console but don't block.
    async fn send_batch(
        client: &reqwest::Client,
        logging_url: &str,
        batch: &mut Vec<EnrichedLogEntry>,
    ) {
        let count = batch.len();
        if count == 0 {
            return;
        }

        let url = format!("{}/logs/batch", logging_url);

        // Take ownership of batch data for sending
        let entries = std::mem::take(batch);
        *batch = Vec::with_capacity(100); // Reset with capacity

        match client.post(&url).json(&entries).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    tracing::trace!(
                        target: "lib_logging_core::sender",
                        "Delivered {} log entries to service",
                        count
                    );
                } else {
                    // Log failure but don't retry - logs already in console
                    tracing::debug!(
                        target: "lib_logging_core::sender",
                        "Failed to deliver logs to service: HTTP {} (logs preserved in console)",
                        response.status()
                    );
                }
            }
            Err(e) => {
                // Log failure but don't retry - logs already in console
                tracing::debug!(
                    target: "lib_logging_core::sender",
                    "Failed to deliver logs to service: {} (logs preserved in console)",
                    e
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = LoggingClientConfig::new("http://localhost:8040", "test");
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.flush_interval_secs, 5);
        assert!(config.send_to_service);
    }

    #[test]
    fn test_console_only_config() {
        let config = LoggingClientConfig::console_only("test");
        assert!(!config.send_to_service);
        assert!(config.logging_url.is_empty());
    }
}
