//! Dotenv Environment Plugin for Hive
//!
//! Loads environment variables from .env files.
//!
//! ## Configuration
//!
//! ```yaml
//! env:
//!   - type: dotenv
//!     dotenv:
//!       files:
//!         - .env
//!         - .env.local
//! ```

use lib_plugin_abi_v3::{
    async_trait, env::EnvProvider, Plugin, PluginCategory, PluginContext, PluginMetadata,
    PluginType, Result as PluginResult, SERVICE_ENV_PROVIDER,
};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

pub struct DotenvPlugin {
    base_dir: std::path::PathBuf,
}

impl Default for DotenvPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DotenvPlugin {
    pub fn new() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_default(),
        }
    }

    pub fn with_base_dir(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    fn load_env_files(&self, config: &serde_json::Value) -> PluginResult<HashMap<String, String>> {
        let mut env = HashMap::new();

        let files = if let Some(files) = config.get("files") {
            files
                .as_array()
                .ok_or_else(|| lib_plugin_abi_v3::PluginError::Config("'files' must be an array".to_string()))?
                .iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect::<Vec<_>>()
        } else if let Some(file) = config.get("file").and_then(|v| v.as_str()) {
            vec![file.to_string()]
        } else {
            vec![".env".to_string()]
        };

        for file_path in files {
            let full_path = if Path::new(&file_path).is_absolute() {
                std::path::PathBuf::from(&file_path)
            } else {
                self.base_dir.join(&file_path)
            };

            if !full_path.exists() {
                debug!("Dotenv file not found (skipping): {}", full_path.display());
                continue;
            }

            let content = std::fs::read_to_string(&full_path).map_err(|e| {
                lib_plugin_abi_v3::PluginError::Other(anyhow::anyhow!(
                    "Failed to read dotenv file {}: {}",
                    full_path.display(),
                    e
                ))
            })?;

            for line in content.lines() {
                let line = line.trim();

                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();

                    let value = if (value.starts_with('"') && value.ends_with('"'))
                        || (value.starts_with('\'') && value.ends_with('\''))
                    {
                        &value[1..value.len() - 1]
                    } else {
                        value
                    };

                    env.insert(key.to_string(), value.to_string());
                }
            }
        }

        Ok(env)
    }
}

#[async_trait]
impl Plugin for DotenvPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.env.dotenv".to_string(),
            name: "Dotenv Environment".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Load environment from .env files".to_string()),
            category: Some(PluginCategory::Env),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_ENV_PROVIDER]
    }
}

#[async_trait]
impl EnvProvider for DotenvPlugin {
    async fn load(&self, config: &serde_json::Value) -> PluginResult<HashMap<String, String>> {
        self.load_env_files(config)
    }

    async fn refresh(&self, config: &serde_json::Value) -> PluginResult<HashMap<String, String>> {
        self.load_env_files(config)
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(DotenvPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_dotenv_plugin() {
        // Create a temporary .env file
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1=value1").unwrap();
        writeln!(file, "KEY2=\"quoted value\"").unwrap();
        writeln!(file, "# comment").unwrap();
        writeln!(file, "KEY3='single quoted'").unwrap();

        let parent = file.path().parent().unwrap();
        let filename = file.path().file_name().unwrap().to_str().unwrap();

        let plugin = DotenvPlugin::with_base_dir(parent);
        let config = serde_json::json!({
            "files": [filename]
        });

        let env = plugin.load(&config).await.unwrap();

        assert_eq!(env.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env.get("KEY2"), Some(&"quoted value".to_string()));
        assert_eq!(env.get("KEY3"), Some(&"single quoted".to_string()));
    }

    #[tokio::test]
    async fn test_dotenv_missing_file() {
        let plugin = DotenvPlugin::with_base_dir("/tmp");
        let config = serde_json::json!({
            "files": ["nonexistent.env"]
        });

        // Should succeed with empty env (missing files are skipped)
        let env = plugin.load(&config).await.unwrap();
        assert!(env.is_empty());
    }
}
