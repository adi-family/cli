//! Centralized logging library with distributed tracing for ADI services.
//!
//! This library provides:
//! - A logging client that sends logs to the central logging service
//! - Trace context (correlation ID + span ID) for distributed tracing
//! - Axum middleware for automatic trace propagation
//!
//! # Architecture
//!
//! ```text
//! Request → Service A [trace: abc, span: 001] → Service B [trace: abc, span: 002]
//!                ↓                                    ↓
//!           LoggingClient                       LoggingClient
//!                ↓                                    ↓
//!           adi-logging-service (TimescaleDB)
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use lib_logging_core::{LoggingClient, LogLevel, TraceContext};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create client (points to logging service)
//!     let logging_url = std::env::var("LOGGING_URL")
//!         .unwrap_or_else(|_| "http://localhost:8040".to_string());
//!
//!     let client = LoggingClient::new(logging_url, "my-service");
//!
//!     // Create trace context for a request
//!     let ctx = TraceContext::new();
//!
//!     // Log with context
//!     client.log(LogLevel::Info, "User logged in", &ctx)
//!         .with_field("user_id", "123")
//!         .with_field("email", "user@example.com")
//!         .send();
//! }
//! ```

mod client;
mod context;
mod env;
mod error;
mod level;
mod entry;

#[cfg(any(feature = "axum", feature = "axum-08-compat"))]
mod middleware;

pub use client::{LoggingClient, LoggingClientConfig};
pub use context::{TraceContext, SpanContext, CorrelationIds};
pub use env::{from_env, from_env_with, LOGGING_URL_ENV, LOGGING_ENABLED_ENV};
pub use error::{LoggingError, Result};
pub use level::LogLevel;
pub use entry::{LogEntry, LogEntryBuilder, EnrichedLogEntry};

#[cfg(any(feature = "axum", feature = "axum-08-compat"))]
pub use middleware::{
    TraceLayer, TraceLayerConfig,
    trace_layer, TraceContextExt,
};

/// Header name for trace ID propagation
pub const TRACE_ID_HEADER: &str = "X-Trace-ID";

/// Header name for span ID propagation
pub const SPAN_ID_HEADER: &str = "X-Span-ID";

/// Header name for parent span ID propagation
pub const PARENT_SPAN_ID_HEADER: &str = "X-Parent-Span-ID";

/// Header name for cocoon ID correlation
pub const COCOON_ID_HEADER: &str = "X-Cocoon-ID";

/// Header name for user ID correlation
pub const USER_ID_HEADER: &str = "X-User-ID";

/// Header name for session ID correlation
pub const SESSION_ID_HEADER: &str = "X-Session-ID";

/// Header name for hive ID correlation
pub const HIVE_ID_HEADER: &str = "X-Hive-ID";
