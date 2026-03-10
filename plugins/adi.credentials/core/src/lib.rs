pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod models;

/// Re-export enums for generated AdiService code compatibility.
pub mod enums {
    pub use super::models::CredentialType;
}

pub use config::Config;
pub use crypto::SecretManager;
pub use db::Database;
pub use error::{ApiError, ApiResult};
pub use models::*;
