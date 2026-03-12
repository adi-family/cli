//! Observability Plugins
//!
//! Built-in observability plugins for log output and file storage.

use crate::observability::{LogLevel, ObservabilityEvent};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error, info};

#[async_trait]
pub trait ObsPlugin: Send + Sync {
    fn name(&self) -> &str;

    async fn init(&mut self, config: &serde_json::Value) -> Result<()>;

    async fn handle(&self, event: &ObservabilityEvent);

    async fn shutdown(&self) -> Result<()>;
}

/// Stdout observability plugin - logs events to console
pub struct StdoutObsPlugin {
    format: OutputFormat,
    min_level: LogLevel,
    colors: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Pretty,
    /// JSON format (one object per line)
    Json,
    Compact,
}

impl Default for StdoutObsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl StdoutObsPlugin {
    pub fn new() -> Self {
        Self {
            format: OutputFormat::Pretty,
            min_level: LogLevel::Info,
            colors: true,
        }
    }

    fn format_event(&self, event: &ObservabilityEvent) -> String {
        match self.format {
            OutputFormat::Json => {
                serde_json::to_string(event).unwrap_or_else(|_| format!("{:?}", event))
            }
            OutputFormat::Pretty => self.format_pretty(event),
            OutputFormat::Compact => self.format_compact(event),
        }
    }

    fn format_pretty(&self, event: &ObservabilityEvent) -> String {
        match event {
            ObservabilityEvent::Log {
                timestamp,
                service_fqn,
                level,
                message,
                ..
            } => {
                let level_str = if self.colors {
                    match level {
                        LogLevel::Trace => "\x1b[90mTRACE\x1b[0m",
                        LogLevel::Debug => "\x1b[36mDEBUG\x1b[0m",
                        LogLevel::Info => "\x1b[32mINFO\x1b[0m ",
                        LogLevel::Notice => "\x1b[34mNOTCE\x1b[0m",
                        LogLevel::Warn => "\x1b[33mWARN\x1b[0m ",
                        LogLevel::Error => "\x1b[31mERROR\x1b[0m",
                        LogLevel::Fatal => "\x1b[35mFATAL\x1b[0m",
                    }
                } else {
                    match level {
                        LogLevel::Trace => "TRACE",
                        LogLevel::Debug => "DEBUG",
                        LogLevel::Info => "INFO ",
                        LogLevel::Notice => "NOTCE",
                        LogLevel::Warn => "WARN ",
                        LogLevel::Error => "ERROR",
                        LogLevel::Fatal => "FATAL",
                    }
                };
                format!(
                    "[{}] {} [{}] {}",
                    timestamp.format("%Y-%m-%dT%H:%M:%S%.3f"),
                    level_str,
                    service_fqn,
                    message
                )
            }
            ObservabilityEvent::ServiceEvent {
                timestamp,
                service_fqn,
                event,
                ..
            } => {
                format!(
                    "[{}] SERVICE [{}] {:?}",
                    timestamp.format("%Y-%m-%dT%H:%M:%S%.3f"),
                    service_fqn,
                    event,
                )
            }
            ObservabilityEvent::HealthCheck {
                timestamp,
                service_fqn,
                status,
                ..
            } => {
                let status_str = if self.colors {
                    match status {
                        crate::observability::HealthStatus::Healthy => "\x1b[32mHEALTHY\x1b[0m",
                        crate::observability::HealthStatus::Unhealthy => "\x1b[31mUNHEALTHY\x1b[0m",
                        crate::observability::HealthStatus::Unknown => "\x1b[33mUNKNOWN\x1b[0m",
                    }
                } else {
                    match status {
                        crate::observability::HealthStatus::Healthy => "HEALTHY",
                        crate::observability::HealthStatus::Unhealthy => "UNHEALTHY",
                        crate::observability::HealthStatus::Unknown => "UNKNOWN",
                    }
                };
                format!(
                    "[{}] HEALTH [{}] {}",
                    timestamp.format("%Y-%m-%dT%H:%M:%S%.3f"),
                    service_fqn,
                    status_str
                )
            }
            ObservabilityEvent::Metric {
                timestamp,
                service_fqn,
                name,
                value,
                ..
            } => {
                format!(
                    "[{}] METRIC [{}] {}: {:?}",
                    timestamp.format("%Y-%m-%dT%H:%M:%S%.3f"),
                    service_fqn,
                    name,
                    value
                )
            }
            _ => format!("{:?}", event),
        }
    }

    fn format_compact(&self, event: &ObservabilityEvent) -> String {
        match event {
            ObservabilityEvent::Log {
                service_fqn,
                level,
                message,
                ..
            } => {
                format!("{:?}|{}|{}", level, service_fqn, message)
            }
            _ => format!("{:?}", event),
        }
    }
}

#[async_trait]
impl ObsPlugin for StdoutObsPlugin {
    fn name(&self) -> &str {
        "stdout"
    }

    async fn init(&mut self, config: &serde_json::Value) -> Result<()> {
        if let Some(format) = config.get("format").and_then(|v| v.as_str()) {
            self.format = match format {
                "json" => OutputFormat::Json,
                "compact" => OutputFormat::Compact,
                _ => OutputFormat::Pretty,
            };
        }

        if let Some(level) = config.get("level").and_then(|v| v.as_str()) {
            self.min_level = level.parse().unwrap_or(LogLevel::Info);
        }

        if let Some(colors) = config.get("colors").and_then(|v| v.as_bool()) {
            self.colors = colors;
        }

        Ok(())
    }

    async fn handle(&self, event: &ObservabilityEvent) {
        if let ObservabilityEvent::Log { level, .. } = event {
            if *level < self.min_level {
                return;
            }
        }

        let output = self.format_event(event);
        println!("{}", output);
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// File observability plugin - writes events to log files
pub struct FileObsPlugin {
    dir: PathBuf,
    rotate: bool,
    /// Maximum file size before rotation
    max_size: u64,
    /// Current file handles (per service)
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

    fn get_or_create_file(&self, service: &str) -> Result<()> {
        let mut files = self.files.lock().unwrap();
        
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

    fn write_event(&self, service: &str, line: &str) -> Result<()> {
        self.get_or_create_file(service)?;
        
        let mut files = self.files.lock().unwrap();
        if let Some(file) = files.get_mut(service) {
            writeln!(file, "{}", line)?;
            
            // Check for rotation
            if self.rotate {
                let metadata = file.metadata()?;
                if metadata.len() > self.max_size {
                    drop(files);
                    self.rotate_file(service)?;
                }
            }
        }
        
        Ok(())
    }

    fn rotate_file(&self, service: &str) -> Result<()> {
        let file_path = self.dir.join(format!("{}.log", service));
        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let rotated_path = self.dir.join(format!("{}.{}.log", service, timestamp));
        
        // Close the current file
        {
            let mut files = self.files.lock().unwrap();
            files.remove(service);
        }
        
        // Rename to rotated name
        std::fs::rename(&file_path, &rotated_path)?;
        
        info!("Rotated log file for {} to {}", service, rotated_path.display());
        Ok(())
    }
}

#[async_trait]
impl ObsPlugin for FileObsPlugin {
    fn name(&self) -> &str {
        "file"
    }

    async fn init(&mut self, config: &serde_json::Value) -> Result<()> {
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

        // Create directory
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
        
        if let Err(e) = self.write_event(service, &line) {
            error!("Failed to write log for {}: {}", service, e);
        }
    }

    async fn shutdown(&self) -> Result<()> {
        // Flush and close all files
        let mut files = self.files.lock().unwrap();
        files.clear();
        Ok(())
    }
}

/// Parse size string (e.g., "10MB", "1GB") to bytes
fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();
    
    if let Some(kb) = s.strip_suffix("KB") {
        kb.trim().parse::<u64>().ok().map(|v| v * 1024)
    } else if let Some(mb) = s.strip_suffix("MB") {
        mb.trim().parse::<u64>().ok().map(|v| v * 1024 * 1024)
    } else if let Some(gb) = s.strip_suffix("GB") {
        gb.trim().parse::<u64>().ok().map(|v| v * 1024 * 1024 * 1024)
    } else if let Some(b) = s.strip_suffix("B") {
        b.trim().parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}

/// Observability plugin manager
pub struct ObsPluginManager {
    plugins: Vec<Box<dyn ObsPlugin>>,
    event_rx: Option<broadcast::Receiver<ObservabilityEvent>>,
}

impl ObsPluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            event_rx: None,
        }
    }

    pub fn register(&mut self, plugin: Box<dyn ObsPlugin>) {
        self.plugins.push(plugin);
    }

    pub async fn init_all(&mut self, defaults: &HashMap<String, serde_json::Value>) -> Result<()> {
        for plugin in &mut self.plugins {
            let plugin_id = format!("hive.obs.{}", plugin.name());
            let config = defaults.get(&plugin_id).cloned().unwrap_or_default();
            plugin.init(&config).await?;
        }
        Ok(())
    }

    pub fn set_receiver(&mut self, rx: broadcast::Receiver<ObservabilityEvent>) {
        self.event_rx = Some(rx);
    }

    pub async fn run(&mut self) {
        let Some(mut rx) = self.event_rx.take() else {
            return;
        };

        loop {
            match rx.recv().await {
                Ok(event) => {
                    for plugin in &self.plugins {
                        plugin.handle(&event).await;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    debug!("Observability receiver lagged {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        for plugin in &self.plugins {
            plugin.shutdown().await?;
        }
        Ok(())
    }
}

impl Default for ObsPluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::LogStream;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024"), Some(1024));
        assert_eq!(parse_size("10KB"), Some(10 * 1024));
        assert_eq!(parse_size("10MB"), Some(10 * 1024 * 1024));
        assert_eq!(parse_size("1GB"), Some(1024 * 1024 * 1024));
    }

    #[tokio::test]
    async fn test_stdout_plugin() {
        let mut plugin = StdoutObsPlugin::new();
        plugin.init(&serde_json::json!({"format": "json"})).await.unwrap();
        
        let event = ObservabilityEvent::Log {
            timestamp: Utc::now(),
            service_fqn: "test".to_string(),
            level: LogLevel::Info,
            message: "test message".to_string(),
            fields: HashMap::new(),
            stream: LogStream::Stdout,
        };
        
        plugin.handle(&event).await;
    }
}
