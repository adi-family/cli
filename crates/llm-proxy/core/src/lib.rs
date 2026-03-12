//! ADI API Proxy - Core Library
//!
//! Provides LLM API proxying with BYOK (Bring Your Own Key) and Platform modes,
//! Rhai scripting for request/response transformation, and comprehensive analytics.

pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod providers;
pub mod transform;
pub mod types;

pub use config::Config;
pub use crypto::SecretManager;
pub use db::{Database, UsageSummary};
pub use error::{ApiError, ApiResult};
pub use types::*;
