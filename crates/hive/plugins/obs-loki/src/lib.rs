//! Grafana Loki Observability Plugin for Hive
//!
//! Sends logs to Grafana Loki for aggregation.
//!
//! ## Configuration
//!
//! ```yaml
//! observability:
//!   plugins:
//!     - loki
//!
//! defaults:
//!   hive.obs.loki:
//!     url: http://localhost:3100
//!     batch_size: 100
//!     flush_interval: 5s
//!     labels:
//!       env: production
//! ```

use anyhow::anyhow;
use async_trait::async_trait;
use lib_plugin_abi_v3::{
    obs::{LogLevel, LogStream, ObservabilityEvent, ObservabilitySink},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_OBSERVABILITY_SINK,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};

/// Dedicated Tokio runtime for HTTP operations.
///
/// cdylib plugins link their own Tokio, invisible to the host process.
/// Without a local runtime, hyper-util DNS resolution panics with
/// "no reactor running". This single-worker runtime provides that reactor.
fn http_runtime() -> &'static tokio::runtime::Runtime {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .thread_name("loki-http")
            .build()
            .expect("Failed to create Loki HTTP runtime")
    })
}

pub struct LokiObsPlugin {
    config: LokiConfig,
    client: reqwest::Client,
    buffer: Arc<Mutex<Vec<LokiEntry>>>,
}

impl Default for LokiObsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl LokiObsPlugin {
    pub fn new() -> Self {
        Self {
            config: LokiConfig::default(),
            client: reqwest::Client::new(),
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if Loki is reachable (best-effort, errors are debug-logged).
    async fn check_readiness(&self) {
        let client = self.client.clone();
        let url = format!("{}/ready", self.config.url);
        let config_url = self.config.url.clone();

        let result = http_runtime()
            .spawn(async move { client.get(&url).send().await })
            .await;

        match result {
            Ok(Ok(resp)) if resp.status().is_success() => {
                debug!("Connected to Loki at {}", config_url);
            }
            Ok(Ok(resp)) => {
                debug!("Loki at {} returned status {}", config_url, resp.status());
            }
            Ok(Err(e)) => {
                debug!("Could not connect to Loki at {}: {}", config_url, e);
            }
            Err(e) => {
                debug!("Loki readiness check failed: {}", e);
            }
        }
    }

    async fn send_to_loki(&self, entries: Vec<LokiEntry>) -> anyhow::Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut streams: HashMap<String, Vec<[String; 2]>> = HashMap::new();

        for entry in entries {
            let labels = entry.labels.clone();
            let label_str = format_labels(&labels);

            streams
                .entry(label_str)
                .or_default()
                .push([entry.timestamp, entry.line]);
        }

        let push_request = LokiPushRequest {
            streams: streams
                .into_iter()
                .map(|(labels, values)| LokiStream { stream: parse_labels(&labels), values })
                .collect(),
        };

        let url = format!("{}/loki/api/v1/push", self.config.url);
        let client = self.client.clone();
        let stream_count = push_request.streams.len();

        http_runtime()
            .spawn(async move {
                let response = client
                    .post(&url)
                    .json(&push_request)
                    .send()
                    .await
                    .map_err(|e| anyhow!("Failed to send to Loki: {}", e))?;

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    error!("Loki push failed ({}): {}", status, body);
                } else {
                    debug!("Sent {} entries to Loki", stream_count);
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
            .map_err(|e| anyhow!("Loki send task panicked: {}", e))??;

        Ok(())
    }
}

#[async_trait]
impl Plugin for LokiObsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.obs.loki".to_string(),
            name: "Loki Observability".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Send logs to Grafana Loki".to_string()),
            category: Some(PluginCategory::Obs),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        self.config = serde_json::from_value(ctx.config.clone())
            .map_err(|e| anyhow!("Invalid Loki config: {}", e))?;

        self.check_readiness().await;
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        self.flush().await
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_OBSERVABILITY_SINK]
    }
}

#[async_trait]
impl ObservabilitySink for LokiObsPlugin {
    async fn init_obs(&mut self, config: &serde_json::Value) -> PluginResult<()> {
        self.config = serde_json::from_value(config.clone())
            .map_err(|e| anyhow!("Invalid Loki config: {}", e))?;

        self.check_readiness().await;
        Ok(())
    }

    async fn handle(&self, event: &ObservabilityEvent) {
        let ObservabilityEvent::Log {
            timestamp,
            service_fqn,
            level,
            message,
            fields,
            stream,
        } = event
        else {
            return;
        };

        if let Some(min_level) = &self.config.level {
            let min: LogLevel = min_level.parse().unwrap_or(LogLevel::Info);
            if *level < min {
                return;
            }
        }

        let mut labels = self.config.labels.clone().unwrap_or_default();
        labels.insert("service".to_string(), service_fqn.clone());
        labels.insert("level".to_string(), level.to_string().to_lowercase());
        labels.insert(
            "stream".to_string(),
            match stream {
                LogStream::Stdout => "stdout",
                LogStream::Stderr => "stderr",
            }
            .to_string(),
        );

        let line = if fields.is_empty() {
            message.clone()
        } else {
            let fields_str: String = fields
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} {}", message, fields_str)
        };

        let entry = LokiEntry {
            timestamp: format!("{}", timestamp.timestamp_nanos_opt().unwrap_or(0)),
            line,
            labels,
        };

        let mut buffer = self.buffer.lock().await;
        buffer.push(entry);

        let batch_size = self.config.batch_size.unwrap_or(100);
        if buffer.len() >= batch_size {
            let entries = std::mem::take(&mut *buffer);
            drop(buffer);

            if let Err(e) = self.send_to_loki(entries).await {
                error!("Failed to send logs to Loki: {}", e);
            }
        }
    }

    async fn flush(&self) -> PluginResult<()> {
        let mut buffer = self.buffer.lock().await;
        let entries = std::mem::take(&mut *buffer);
        drop(buffer);

        self.send_to_loki(entries).await.map_err(Into::into)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LokiConfig {
    #[serde(default = "default_url")]
    pub url: String,
    pub batch_size: Option<usize>,
    pub flush_interval: Option<String>,
    /// Minimum log level
    pub level: Option<String>,
    /// Static labels added to every log entry
    pub labels: Option<HashMap<String, String>>,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Tenant ID for multi-tenant Loki
    pub tenant_id: Option<String>,
}

fn default_url() -> String {
    "http://localhost:3100".to_string()
}

struct LokiEntry {
    timestamp: String,
    line: String,
    labels: HashMap<String, String>,
}

#[derive(Serialize)]
struct LokiPushRequest {
    streams: Vec<LokiStream>,
}

#[derive(Serialize)]
struct LokiStream {
    stream: HashMap<String, String>,
    values: Vec<[String; 2]>,
}

fn format_labels(labels: &HashMap<String, String>) -> String {
    let parts: Vec<String> = labels
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", k, v.replace('\"', "\\\"")))
        .collect();
    format!("{{{}}}", parts.join(", "))
}

fn parse_labels(label_str: &str) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    let inner = label_str.trim_start_matches('{').trim_end_matches('}');
    for part in inner.split(", ") {
        if let Some((k, v)) = part.split_once('=') {
            labels.insert(
                k.to_string(),
                v.trim_matches('"').replace("\\\"", "\""),
            );
        }
    }
    labels
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(LokiObsPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = LokiObsPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.obs.loki");
    }

    #[test]
    fn test_format_labels() {
        let mut labels = HashMap::new();
        labels.insert("service".to_string(), "api".to_string());
        labels.insert("level".to_string(), "info".to_string());

        let formatted = format_labels(&labels);
        assert!(formatted.contains("service=\"api\""));
        assert!(formatted.contains("level=\"info\""));
    }
}
