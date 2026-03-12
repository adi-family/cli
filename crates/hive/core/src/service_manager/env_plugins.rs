use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, trace, warn};

use super::environment::EnvPlugin;

pub struct DotenvPlugin {
    base_dir: std::path::PathBuf,
}

impl DotenvPlugin {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }
}

#[async_trait]
impl EnvPlugin for DotenvPlugin {
    fn name(&self) -> &str {
        "dotenv"
    }

    async fn load(&self, config: &serde_json::Value) -> Result<HashMap<String, String>> {
        let mut env = HashMap::new();

        let files = if let Some(files) = config.get("files") {
            files
                .as_array()
                .ok_or_else(|| anyhow!("'files' must be an array"))?
                .iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect::<Vec<_>>()
        } else if let Some(file) = config.get("file").and_then(|v| v.as_str()) {
            vec![file.to_string()]
        } else {
            // Default: .env
            vec![".env".to_string()]
        };

        debug!(files = ?files, base_dir = %self.base_dir.display(), "Loading dotenv files");

        for file_path in files {
            let full_path = if Path::new(&file_path).is_absolute() {
                std::path::PathBuf::from(&file_path)
            } else {
                self.base_dir.join(&file_path)
            };

            if !tokio::fs::try_exists(&full_path).await.unwrap_or(false) {
                // Often optional like .env.local
                tracing::debug!("Dotenv file not found (skipping): {}", full_path.display());
                continue;
            }

            let content = tokio::fs::read_to_string(&full_path)
                .await
                .with_context(|| format!("Failed to read dotenv file: {}", full_path.display()))?;

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

            trace!(path = %full_path.display(), vars = env.len(), "Loaded dotenv file");
        }

        debug!(total_vars = env.len(), "Dotenv loading complete");
        Ok(env)
    }
}

/// Synchronous port resolution for parse-time interpolation via ports-manager CLI.
pub struct PortsParsePlugin {
    prefix: String,
    cache: std::sync::Mutex<HashMap<String, u16>>,
}

impl PortsParsePlugin {
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
            cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            cache: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn get_port_sync(&self, key: &str) -> Result<u16> {
        let full_key = if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}{}", self.prefix, key)
        };

        {
            let cache = self.cache.lock().unwrap();
            if let Some(&port) = cache.get(&full_key) {
                trace!(key = %full_key, port = port, "Port resolved from cache");
                return Ok(port);
            }
        }

        debug!(key = %full_key, "Resolving port via ports-manager");
        let output = std::process::Command::new("ports-manager")
            .args(["get", &full_key])
            .output()
            .context("Failed to run ports-manager")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("ports-manager failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let port: u16 = stdout
            .trim()
            .parse()
            .context("Invalid port number from ports-manager")?;

        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(full_key, port);
        }

        Ok(port)
    }
}

impl Default for PortsParsePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::hive_config::ParsePlugin for PortsParsePlugin {
    fn name(&self) -> &str {
        "ports"
    }

    fn resolve(&self, key: &str) -> Result<Option<String>> {
        match self.get_port_sync(key) {
            Ok(port) => {
                trace!(key = %key, port = port, "Resolved port");
                Ok(Some(port.to_string()))
            }
            Err(e) => {
                warn!(key = %key, error = %e, "Failed to resolve port");
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_dotenv_plugin() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1=value1").unwrap();
        writeln!(file, "KEY2=\"quoted value\"").unwrap();
        writeln!(file, "# comment").unwrap();
        writeln!(file, "KEY3='single quoted'").unwrap();

        let parent = file.path().parent().unwrap();
        let filename = file.path().file_name().unwrap().to_str().unwrap();

        let plugin = DotenvPlugin::new(parent);
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
        let plugin = DotenvPlugin::new("/tmp");
        let config = serde_json::json!({
            "files": ["nonexistent.env"]
        });

        let env = plugin.load(&config).await.unwrap();
        assert!(env.is_empty());
    }
}
