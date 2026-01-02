//! Analytics core library for ADI
//!
//! Provides event tracking client that sends events to analytics ingestion service.
//!
//! # Usage
//!
//! ```rust,no_run
//! use lib_analytics_core::{AnalyticsClient, AnalyticsEvent};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create client (points to analytics ingestion service)
//!     let analytics_url = std::env::var("ANALYTICS_URL")
//!         .unwrap_or_else(|_| "http://localhost:8094".to_string());
//!
//!     let client = AnalyticsClient::new(analytics_url);
//!
//!     // Track events (non-blocking, batched automatically)
//!     client.track(AnalyticsEvent::AuthLoginAttempt {
//!         user_id: None,
//!         email: "user@example.com".to_string(),
//!         success: true,
//!         error: None,
//!     });
//! }
//! ```

mod client;
mod error;
mod events;

pub use client::AnalyticsClient;
pub use error::{AnalyticsError, Result};
pub use events::{AnalyticsEvent, EnrichedEvent};
