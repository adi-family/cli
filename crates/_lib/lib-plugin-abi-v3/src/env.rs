//! Environment provider plugin trait

use crate::{Plugin, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Environment provider plugin trait
///
/// Environment providers load environment variables from various sources
/// (dotenv files, Vault, 1Password, AWS Secrets Manager, etc.).
#[async_trait]
pub trait EnvProvider: Plugin {
    /// Load environment variables
    async fn load(&self, config: &Value) -> Result<HashMap<String, String>>;

    /// Refresh environment variables (for dynamic secrets)
    async fn refresh(&self, config: &Value) -> Result<HashMap<String, String>>;
}
