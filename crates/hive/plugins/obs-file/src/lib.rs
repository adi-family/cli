//! File Observability Plugin for Hive
//!
//! Writes logs and events to files with rotation support.
//!
//! ## Configuration
//!
//! ```yaml
//! observability:
//!   - type: file
//!     file:
//!       dir: ~/.adi/hive/logs
//!       rotate: true
//!       max_size: 10MB
//!       level: debug
//! ```

use async_trait::async_trait;
use chrono::Utc;
use lib_plugin_abi_v3::{
    obs::{LogLevel, ObservabilityEvent, ObservabilitySink},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_OBSERVABILITY_SINK,
};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct FileObsPlugin {
    dir: PathBuf,
    rotate: bool,
    max_size: u64,
    files: Arc<Mutex<HashMap<String, File>>>,
    min_level: LogLevel,
}

impl Default for FileObsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl FileObsPlugin {
    pub fn new() -> Self {
        let dir = dirs::home_dir()
            .map(|h| h.join(".adi/hive/logs"))
            .unwrap_or_else(|| PathBuf::from(".adi/hive/logs"));
        Self {
            dir,
            rotate: true,
            max_size: 10 * 1024 * 1024, // 10MB default
            files: Arc::new(Mutex::new(HashMap::new())),
            min_level: LogLevel::Debug,
        }
    }

    async fn get_or_create_file(&self, service: &str) -> anyhow::Result<()> {
        let mut files = self.files.lock().await;

        if !files.contains_key(service) {
            std::fs::create_dir_all(&self.dir)?;

            let file_path = self.dir.join(format!("{}.log", service));
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)?;

            files.insert(service.to_string(), file);
        }

        Ok(())
    }

    async fn write_event(&self, service: &str, line: &str) -> anyhow::Result<()> {
        self.get_or_create_file(service).await?;

        let mut files = self.files.lock().await;
        if let Some(file) = files.get_mut(service) {
            writeln!(file, "{}", line)?;

            if self.rotate {
                let metadata = file.metadata()?;
                if metadata.len() > self.max_size {
                    drop(files);
                    self.rotate_file(service).await?;
                }
            }
        }

        Ok(())
    }

    async fn rotate_file(&self, service: &str) -> anyhow::Result<()> {
        let file_path = self.dir.join(format!("{}.log", service));
        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let rotated_path = self.dir.join(format!("{}.{}.log", service, timestamp));

        {
            let mut files = self.files.lock().await;
            files.remove(service);
        }

        std::fs::rename(&file_path, &rotated_path)?;

        info!(
            "Rotated log file for {} to {}",
            service,
            rotated_path.display()
        );
        Ok(())
    }
}

#[async_trait]
impl Plugin for FileObsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.obs.file".to_string(),
            name: "file".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("File-based log storage".to_string()),
            category: Some(PluginCategory::Obs),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        let config = &ctx.config;

        if let Some(dir) = config.get("dir").and_then(|v| v.as_str()) {
            self.dir = PathBuf::from(dir);
        }

        if let Some(rotate) = config.get("rotate").and_then(|v| v.as_bool()) {
            self.rotate = rotate;
        }

        if let Some(max_size) = config.get("max_size").and_then(|v| v.as_str()) {
            self.max_size = parse_size(max_size).unwrap_or(10 * 1024 * 1024);
        }

        if let Some(level) = config.get("level").and_then(|v| v.as_str()) {
            self.min_level = level.parse().unwrap_or(LogLevel::Debug);
        }

        std::fs::create_dir_all(&self.dir)?;

        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        let mut files = self.files.lock().await;
        files.clear();
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_OBSERVABILITY_SINK]
    }
}

#[async_trait]
impl ObservabilitySink for FileObsPlugin {
    async fn init_obs(&mut self, config: &serde_json::Value) -> PluginResult<()> {
        if let Some(dir) = config.get("dir").and_then(|v| v.as_str()) {
            self.dir = PathBuf::from(dir);
        }

        if let Some(rotate) = config.get("rotate").and_then(|v| v.as_bool()) {
            self.rotate = rotate;
        }

        if let Some(max_size) = config.get("max_size").and_then(|v| v.as_str()) {
            self.max_size = parse_size(max_size).unwrap_or(10 * 1024 * 1024);
        }

        if let Some(level) = config.get("level").and_then(|v| v.as_str()) {
            self.min_level = level.parse().unwrap_or(LogLevel::Debug);
        }

        std::fs::create_dir_all(&self.dir)?;

        Ok(())
    }

    async fn handle(&self, event: &ObservabilityEvent) {
        if let ObservabilityEvent::Log { level, .. } = event {
            if *level < self.min_level {
                return;
            }
        }

        let service = event.service_fqn();
        let line = serde_json::to_string(event).unwrap_or_else(|_| format!("{:?}", event));

        if let Err(e) = self.write_event(service, &line).await {
            error!("Failed to write log for {}: {}", service, e);
        }
    }

    async fn flush(&self) -> PluginResult<()> {
        let files = self.files.lock().await;
        for file in files.values() {
            file.sync_all()?;
        }
        Ok(())
    }
}

fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();

    if let Some(kb) = s.strip_suffix("KB") {
        kb.trim().parse::<u64>().ok().map(|v| v * 1024)
    } else if let Some(mb) = s.strip_suffix("MB") {
        mb.trim().parse::<u64>().ok().map(|v| v * 1024 * 1024)
    } else if let Some(gb) = s.strip_suffix("GB") {
        gb.trim()
            .parse::<u64>()
            .ok()
            .map(|v| v * 1024 * 1024 * 1024)
    } else if let Some(b) = s.strip_suffix("B") {
        b.trim().parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(FileObsPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024"), Some(1024));
        assert_eq!(parse_size("10KB"), Some(10 * 1024));
        assert_eq!(parse_size("10MB"), Some(10 * 1024 * 1024));
        assert_eq!(parse_size("1GB"), Some(1024 * 1024 * 1024));
    }
}
