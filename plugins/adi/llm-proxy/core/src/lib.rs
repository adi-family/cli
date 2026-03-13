pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod models;
pub mod providers;
pub mod service;
pub mod transform;

/// Re-export models as `types` for backward compat with db/provider code.
pub use models as types;

pub mod enums {
    pub use super::models::{KeyMode, ProviderType, RequestStatus};
}

pub use config::Config;
pub use crypto::SecretManager;
pub use db::Database;
pub use error::{ApiError, ApiResult};
pub use models::*;
pub use service::{LlmProxyService, LlmProxyServiceAdi};
