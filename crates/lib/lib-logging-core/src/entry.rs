//! Log entry types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{CorrelationIds, LogLevel, TraceContext};

/// A log entry to be sent to the logging service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,

    /// Log message
    pub message: String,

    /// Trace ID (request chain identifier)
    pub trace_id: Uuid,

    /// Span ID (operation identifier)
    pub span_id: Uuid,

    /// Parent span ID (if this is a child span)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<Uuid>,

    /// Business-level correlation IDs
    #[serde(default, skip_serializing_if = "CorrelationIds::is_empty")]
    pub correlation: CorrelationIds,

    /// Additional structured fields
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, serde_json::Value>,

    /// Error details (for error/fatal logs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,

    /// Source location (file:line)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Target/module path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Error information for error logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Error type/kind
    pub kind: String,

    /// Error message
    pub message: String,

    /// Stack trace (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(level: LogLevel, message: impl Into<String>, ctx: &TraceContext) -> Self {
        Self {
            level,
            message: message.into(),
            trace_id: ctx.trace_id,
            span_id: ctx.span_id,
            parent_span_id: ctx.parent_span_id,
            correlation: ctx.correlation.clone(),
            fields: HashMap::new(),
            error: None,
            source: None,
            target: None,
        }
    }

    /// Add a field to the log entry.
    pub fn with_field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.fields.insert(key.into(), v);
        }
        self
    }

    /// Add error information.
    pub fn with_error(mut self, error: impl std::fmt::Display) -> Self {
        self.error = Some(ErrorInfo {
            kind: std::any::type_name_of_val(&error).to_string(),
            message: error.to_string(),
            stack_trace: None,
        });
        self
    }

    /// Add source location.
    pub fn with_source(mut self, file: &str, line: u32) -> Self {
        self.source = Some(format!("{}:{}", file, line));
        self
    }

    /// Add target/module path.
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Log to console via tracing (backup logging).
    /// This is always called, regardless of whether the logging service is available.
    pub(crate) fn log_to_console(&self, service: &str) {
        let trace_id = &self.trace_id.to_string()[..8]; // Short trace ID for console
        let span_id = &self.span_id.to_string()[..8];

        // Build correlation IDs string
        let mut corr_parts = Vec::new();
        if let Some(ref cocoon_id) = self.correlation.cocoon_id {
            corr_parts.push(format!("cocoon={}", &cocoon_id[..cocoon_id.len().min(8)]));
        }
        if let Some(ref user_id) = self.correlation.user_id {
            corr_parts.push(format!("user={}", &user_id[..user_id.len().min(8)]));
        }
        if let Some(ref session_id) = self.correlation.session_id {
            corr_parts.push(format!(
                "session={}",
                &session_id[..session_id.len().min(8)]
            ));
        }
        let corr_str = if corr_parts.is_empty() {
            String::new()
        } else {
            format!(" {}", corr_parts.join(" "))
        };

        // Build fields string for structured logging
        let fields_str = if self.fields.is_empty() {
            String::new()
        } else {
            format!(
                " {}",
                self.fields
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };

        let error_str = self
            .error
            .as_ref()
            .map(|e| format!(" error=\"{}\"", e.message))
            .unwrap_or_default();

        // Use tracing macros for console output with trace context
        match self.level {
            LogLevel::Trace => tracing::trace!(
                trace_id = trace_id,
                span_id = span_id,
                service = service,
                cocoon_id = self.correlation.cocoon_id.as_deref().unwrap_or(""),
                user_id = self.correlation.user_id.as_deref().unwrap_or(""),
                "[{}] {}{}{}{}",
                service,
                self.message,
                corr_str,
                fields_str,
                error_str
            ),
            LogLevel::Debug => tracing::debug!(
                trace_id = trace_id,
                span_id = span_id,
                service = service,
                cocoon_id = self.correlation.cocoon_id.as_deref().unwrap_or(""),
                user_id = self.correlation.user_id.as_deref().unwrap_or(""),
                "[{}] {}{}{}{}",
                service,
                self.message,
                corr_str,
                fields_str,
                error_str
            ),
            LogLevel::Info => tracing::info!(
                trace_id = trace_id,
                span_id = span_id,
                service = service,
                cocoon_id = self.correlation.cocoon_id.as_deref().unwrap_or(""),
                user_id = self.correlation.user_id.as_deref().unwrap_or(""),
                "[{}] {}{}{}{}",
                service,
                self.message,
                corr_str,
                fields_str,
                error_str
            ),
            LogLevel::Notice => tracing::info!(
                trace_id = trace_id,
                span_id = span_id,
                service = service,
                cocoon_id = self.correlation.cocoon_id.as_deref().unwrap_or(""),
                user_id = self.correlation.user_id.as_deref().unwrap_or(""),
                "[{}] [NOTICE] {}{}{}{}",
                service,
                self.message,
                corr_str,
                fields_str,
                error_str
            ),
            LogLevel::Warn => tracing::warn!(
                trace_id = trace_id,
                span_id = span_id,
                service = service,
                cocoon_id = self.correlation.cocoon_id.as_deref().unwrap_or(""),
                user_id = self.correlation.user_id.as_deref().unwrap_or(""),
                "[{}] {}{}{}{}",
                service,
                self.message,
                corr_str,
                fields_str,
                error_str
            ),
            LogLevel::Error => tracing::error!(
                trace_id = trace_id,
                span_id = span_id,
                service = service,
                cocoon_id = self.correlation.cocoon_id.as_deref().unwrap_or(""),
                user_id = self.correlation.user_id.as_deref().unwrap_or(""),
                "[{}] {}{}{}{}",
                service,
                self.message,
                corr_str,
                fields_str,
                error_str
            ),
            LogLevel::Fatal => tracing::error!(
                trace_id = trace_id,
                span_id = span_id,
                service = service,
                cocoon_id = self.correlation.cocoon_id.as_deref().unwrap_or(""),
                user_id = self.correlation.user_id.as_deref().unwrap_or(""),
                "[{}] [FATAL] {}{}{}{}",
                service,
                self.message,
                corr_str,
                fields_str,
                error_str
            ),
        }
    }
}

/// Enriched log entry with metadata (sent to ingestion service).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedLogEntry {
    /// Timestamp when the log was created
    pub timestamp: DateTime<Utc>,

    /// Service that generated the log
    pub service: String,

    /// The log entry
    #[serde(flatten)]
    pub entry: LogEntry,

    /// Hostname of the service instance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    /// Environment (dev, staging, production)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,

    /// Service version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl EnrichedLogEntry {
    /// Create an enriched log entry.
    pub fn new(entry: LogEntry, service: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            service: service.to_string(),
            entry,
            hostname: crate::env::hostname(),
            environment: crate::env::environment(),
            version: crate::env::service_version(),
        }
    }
}

/// Builder for creating log entries fluently.
///
/// All operations are non-blocking. When `send()` is called:
/// 1. Log is immediately written to console (via tracing) - always works
/// 2. Log is queued for async delivery to logging service - never blocks
pub struct LogEntryBuilder {
    entry: LogEntry,
    sender: mpsc::UnboundedSender<EnrichedLogEntry>,
    service: Arc<str>,
}

impl LogEntryBuilder {
    pub(crate) fn new(
        level: LogLevel,
        message: impl Into<String>,
        ctx: &TraceContext,
        sender: mpsc::UnboundedSender<EnrichedLogEntry>,
        service: Arc<str>,
    ) -> Self {
        Self {
            entry: LogEntry::new(level, message, ctx),
            sender,
            service,
        }
    }

    /// Add a field to the log entry.
    pub fn with_field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.entry = self.entry.with_field(key, value);
        self
    }

    /// Add multiple fields at once.
    pub fn with_fields(
        mut self,
        fields: impl IntoIterator<Item = (impl Into<String>, impl Serialize)>,
    ) -> Self {
        for (key, value) in fields {
            self.entry = self.entry.with_field(key, value);
        }
        self
    }

    /// Add error information.
    pub fn with_error(mut self, error: impl std::fmt::Display) -> Self {
        self.entry = self.entry.with_error(error);
        self
    }

    /// Add source location.
    pub fn with_source(mut self, file: &str, line: u32) -> Self {
        self.entry = self.entry.with_source(file, line);
        self
    }

    /// Add target/module path.
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.entry = self.entry.with_target(target);
        self
    }

    /// Send the log entry (non-blocking).
    ///
    /// This method:
    /// 1. Immediately logs to console via tracing (backup, always works)
    /// 2. Queues the log for async delivery to the logging service (never blocks)
    ///
    /// If the logging service is unavailable, logs still appear in console.
    pub fn send(self) {
        // 1. Always log to console first (backup)
        self.entry.log_to_console(&self.service);

        // 2. Queue for async delivery to logging service (non-blocking)
        let enriched = EnrichedLogEntry::new(self.entry, &self.service);
        // Using unbounded channel - send never blocks
        // Ignoring error if receiver is dropped (service shutting down)
        let _ = self.sender.send(enriched);
    }
}

/// Macro for logging with source location.
#[macro_export]
macro_rules! log_entry {
    ($client:expr, $level:expr, $msg:expr, $ctx:expr) => {
        $client
            .log($level, $msg, $ctx)
            .with_source(file!(), line!())
    };
}
