//! Log streaming service trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// A single log line from a plugin
#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub service: String,
    pub message: String,
}

/// Context for creating a log stream
#[derive(Debug, Clone)]
pub struct LogStreamContext {
    pub service: Option<String>,
    pub level: Option<String>,
    pub tail: Option<u32>,
    pub follow: bool,
}

/// Async stream of log lines
#[async_trait]
pub trait LogStream: Send {
    async fn next(&mut self) -> Option<LogLine>;
}

/// Service trait for plugins that provide log streaming
#[async_trait]
pub trait LogProvider: Plugin {
    async fn log_stream(&self, ctx: LogStreamContext) -> Result<Box<dyn LogStream>>;
}
