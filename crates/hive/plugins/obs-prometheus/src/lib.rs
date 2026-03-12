//! Prometheus Metrics Observability Plugin for Hive
//!
//! Exports metrics in Prometheus format for scraping.
//!
//! ## Configuration
//!
//! ```yaml
//! observability:
//!   plugins:
//!     - prometheus
//!
//! defaults:
//!   hive.obs.prometheus:
//!     port: 9090
//!     path: /metrics
//!     prefix: hive_
//! ```

use async_trait::async_trait;
use lib_plugin_abi_v3::{
    obs::{HealthStatus, MetricValue, ObservabilityEvent, ObservabilitySink},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_OBSERVABILITY_SINK,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

pub struct PrometheusObsPlugin {
    config: PrometheusConfig,
    metrics: Arc<RwLock<MetricsStore>>,
}

#[derive(Default)]
struct MetricsStore {
    counters: HashMap<String, HashMap<String, u64>>,
    gauges: HashMap<String, HashMap<String, f64>>,
    histograms: HashMap<String, HashMap<String, HistogramData>>,
}

#[derive(Clone)]
struct HistogramData {
    count: u64,
    sum: f64,
    buckets: Vec<(f64, u64)>,
}

impl Default for PrometheusObsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PrometheusObsPlugin {
    pub fn new() -> Self {
        Self {
            config: PrometheusConfig::default(),
            metrics: Arc::new(RwLock::new(MetricsStore::default())),
        }
    }

    fn format_labels(labels: &HashMap<String, String>) -> String {
        if labels.is_empty() {
            return String::new();
        }

        let parts: Vec<String> = labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();

        format!("{{{}}}", parts.join(","))
    }

    pub async fn export(&self) -> String {
        let metrics = self.metrics.read().await;
        let prefix = &self.config.prefix;
        let mut output = String::new();

        for (name, label_values) in &metrics.counters {
            output.push_str(&format!("# HELP {}{} Counter metric\n", prefix, name));
            output.push_str(&format!("# TYPE {}{} counter\n", prefix, name));
            for (labels, value) in label_values {
                output.push_str(&format!("{}{}{} {}\n", prefix, name, labels, value));
            }
        }

        for (name, label_values) in &metrics.gauges {
            output.push_str(&format!("# HELP {}{} Gauge metric\n", prefix, name));
            output.push_str(&format!("# TYPE {}{} gauge\n", prefix, name));
            for (labels, value) in label_values {
                output.push_str(&format!("{}{}{} {}\n", prefix, name, labels, value));
            }
        }

        for (name, label_values) in &metrics.histograms {
            output.push_str(&format!("# HELP {}{} Histogram metric\n", prefix, name));
            output.push_str(&format!("# TYPE {}{} histogram\n", prefix, name));
            for (labels, data) in label_values {
                for (le, count) in &data.buckets {
                    let bucket_labels = if labels.is_empty() {
                        format!("{{le=\"{}\"}}", le)
                    } else {
                        format!("{{{},le=\"{}\"}}", &labels[1..labels.len() - 1], le)
                    };
                    output.push_str(&format!(
                        "{}{}_bucket{} {}\n",
                        prefix, name, bucket_labels, count
                    ));
                }
                let inf_labels = if labels.is_empty() {
                    "{le=\"+Inf\"}".to_string()
                } else {
                    format!("{{{},le=\"+Inf\"}}", &labels[1..labels.len() - 1])
                };
                output.push_str(&format!(
                    "{}{}_bucket{} {}\n",
                    prefix, name, inf_labels, data.count
                ));
                output.push_str(&format!("{}{}_sum{} {}\n", prefix, name, labels, data.sum));
                output.push_str(&format!(
                    "{}{}_count{} {}\n",
                    prefix, name, labels, data.count
                ));
            }
        }

        output
    }
}

#[async_trait]
impl Plugin for PrometheusObsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.obs.prometheus".to_string(),
            name: "prometheus".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Export metrics in Prometheus format".to_string()),
            category: Some(PluginCategory::Obs),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        self.config = serde_json::from_value(ctx.config.clone())
            .unwrap_or_else(|_| PrometheusConfig::default());

        debug!(
            "Prometheus metrics will be available at :{}{}",
            self.config.port, self.config.path
        );

        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_OBSERVABILITY_SINK]
    }
}

#[async_trait]
impl ObservabilitySink for PrometheusObsPlugin {
    async fn init_obs(&mut self, config: &serde_json::Value) -> PluginResult<()> {
        self.config =
            serde_json::from_value(config.clone()).unwrap_or_else(|_| PrometheusConfig::default());

        debug!(
            "Prometheus metrics will be available at :{}{}",
            self.config.port, self.config.path
        );

        Ok(())
    }

    async fn handle(&self, event: &ObservabilityEvent) {
        let mut metrics = self.metrics.write().await;

        match event {
            ObservabilityEvent::Metric {
                service_fqn,
                name,
                value,
                labels,
                ..
            } => {
                let mut all_labels = labels.clone();
                all_labels.insert("service".to_string(), service_fqn.clone());
                let label_str = Self::format_labels(&all_labels);

                match value {
                    MetricValue::Counter(v) => {
                        metrics
                            .counters
                            .entry(name.clone())
                            .or_default()
                            .insert(label_str, *v);
                    }
                    MetricValue::Gauge(v) => {
                        metrics
                            .gauges
                            .entry(name.clone())
                            .or_default()
                            .insert(label_str, *v);
                    }
                    MetricValue::Histogram(values) => {
                        let buckets = vec![
                            (0.005, 0),
                            (0.01, 0),
                            (0.025, 0),
                            (0.05, 0),
                            (0.1, 0),
                            (0.25, 0),
                            (0.5, 0),
                            (1.0, 0),
                            (2.5, 0),
                            (5.0, 0),
                            (10.0, 0),
                        ];

                        let mut data = HistogramData {
                            count: values.len() as u64,
                            sum: values.iter().sum(),
                            buckets,
                        };

                        for v in values {
                            for (le, count) in &mut data.buckets {
                                if *v <= *le {
                                    *count += 1;
                                }
                            }
                        }

                        metrics
                            .histograms
                            .entry(name.clone())
                            .or_default()
                            .insert(label_str, data);
                    }
                }
            }
            ObservabilityEvent::HealthCheck {
                service_fqn,
                status,
                response_time_ms,
                ..
            } => {
                let mut labels = HashMap::new();
                labels.insert("service".to_string(), service_fqn.clone());
                let label_str = Self::format_labels(&labels);

                let health_value = match status {
                    HealthStatus::Healthy => 1.0,
                    HealthStatus::Unhealthy => 0.0,
                    HealthStatus::Unknown => -1.0,
                };
                metrics
                    .gauges
                    .entry("health_status".to_string())
                    .or_default()
                    .insert(label_str.clone(), health_value);

                if let Some(ms) = response_time_ms {
                    metrics
                        .gauges
                        .entry("health_check_duration_ms".to_string())
                        .or_default()
                        .insert(label_str, *ms as f64);
                }
            }
            ObservabilityEvent::ServiceEvent {
                service_fqn,
                event_type,
                ..
            } => {
                let mut labels = HashMap::new();
                labels.insert("service".to_string(), service_fqn.clone());
                labels.insert("event".to_string(), format!("{:?}", event_type).to_lowercase());
                let label_str = Self::format_labels(&labels);

                let counter = metrics
                    .counters
                    .entry("service_events_total".to_string())
                    .or_default()
                    .entry(label_str)
                    .or_insert(0);
                *counter += 1;
            }
            ObservabilityEvent::Log {
                service_fqn, level, ..
            } => {
                let mut labels = HashMap::new();
                labels.insert("service".to_string(), service_fqn.clone());
                labels.insert("level".to_string(), level.to_string().to_lowercase());
                let label_str = Self::format_labels(&labels);

                let counter = metrics
                    .counters
                    .entry("log_messages_total".to_string())
                    .or_default()
                    .entry(label_str)
                    .or_insert(0);
                *counter += 1;
            }
        }
    }

    async fn flush(&self) -> PluginResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_path")]
    pub path: String,
    /// Metric name prefix (default: "hive_")
    #[serde(default = "default_prefix")]
    pub prefix: String,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            path: default_path(),
            prefix: default_prefix(),
        }
    }
}

fn default_port() -> u16 {
    9090
}

fn default_path() -> String {
    "/metrics".to_string()
}

fn default_prefix() -> String {
    "hive_".to_string()
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(PrometheusObsPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = PrometheusObsPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.obs.prometheus");
    }

    #[test]
    fn test_format_labels() {
        let mut labels = HashMap::new();
        labels.insert("service".to_string(), "api".to_string());
        labels.insert("level".to_string(), "info".to_string());

        let formatted = PrometheusObsPlugin::format_labels(&labels);
        assert!(formatted.starts_with('{'));
        assert!(formatted.ends_with('}'));
        assert!(formatted.contains("service=\"api\""));
    }

    #[tokio::test]
    async fn test_export() {
        let plugin = PrometheusObsPlugin::new();

        // Add a gauge metric
        {
            let mut metrics = plugin.metrics.write().await;
            metrics
                .gauges
                .entry("test_gauge".to_string())
                .or_default()
                .insert("{service=\"api\"}".to_string(), 42.0);
        }

        let output = plugin.export().await;
        assert!(output.contains("hive_test_gauge"));
        assert!(output.contains("42"));
    }
}
