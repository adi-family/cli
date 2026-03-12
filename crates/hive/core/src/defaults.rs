//! Plugin Defaults System
//!
//! Handles defaults inheritance for plugins as specified in hive.yaml.
//! The `defaults` section allows configuring default values for all plugin instances.

use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, trace};

#[derive(Debug, Clone, Default)]
pub struct DefaultsManager {
    defaults: HashMap<String, Value>,
}

impl DefaultsManager {
    pub fn new() -> Self {
        Self {
            defaults: HashMap::new(),
        }
    }

    /// Create from hive.yaml defaults section
    pub fn from_config(defaults: &HashMap<String, Value>) -> Self {
        Self {
            defaults: defaults.clone(),
        }
    }

    pub fn get(&self, plugin_id: &str) -> Option<&Value> {
        self.defaults.get(plugin_id)
    }

    /// Get defaults with fallback to empty object
    pub fn get_or_empty(&self, plugin_id: &str) -> Value {
        self.defaults
            .get(plugin_id)
            .cloned()
            .unwrap_or_else(|| Value::Object(Default::default()))
    }

    /// Merge defaults with service-specific config
    /// Service config takes precedence over defaults
    pub fn merge_with(&self, plugin_id: &str, service_config: &Value) -> Value {
        let defaults = self.get_or_empty(plugin_id);
        trace!(plugin_id = %plugin_id, has_defaults = self.defaults.contains_key(plugin_id), "Merging defaults with service config");
        merge_json(&defaults, service_config)
    }

    pub fn set(&mut self, plugin_id: &str, config: Value) {
        self.defaults.insert(plugin_id.to_string(), config);
    }

    pub fn plugin_ids(&self) -> Vec<&str> {
        self.defaults.keys().map(|s| s.as_str()).collect()
    }
}

/// Deep merge two JSON values, with b taking precedence
pub fn merge_json(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Object(a_obj), Value::Object(b_obj)) => {
            let mut result = a_obj.clone();
            for (key, b_value) in b_obj {
                if let Some(a_value) = result.get(key) {
                    result.insert(key.clone(), merge_json(a_value, b_value));
                } else {
                    result.insert(key.clone(), b_value.clone());
                }
            }
            Value::Object(result)
        }
        // For non-objects, b always wins
        (_, b) => b.clone(),
    }
}

/// Apply defaults to environment config providers
pub fn apply_env_defaults(
    defaults: &DefaultsManager,
    config: &mut crate::hive_config::EnvironmentConfig,
) {
    for (provider_name, provider_config) in &mut config.providers {
        let plugin_id = format!("hive.env.{}", provider_name);
        trace!(plugin_id = %plugin_id, "Applying env defaults");
        let merged = defaults.merge_with(&plugin_id, provider_config);
        *provider_config = merged;
    }
}

/// Apply defaults to runner config
pub fn apply_runner_defaults(
    defaults: &DefaultsManager,
    config: &mut crate::hive_config::RunnerConfig,
) {
    let plugin_id = format!("hive.runner.{}", config.runner_type);

    if let Some(type_config) = config.config.get_mut(&config.runner_type) {
        let merged = defaults.merge_with(&plugin_id, type_config);
        *type_config = merged;
    }
}

/// Apply defaults to health check config
pub fn apply_health_defaults(
    defaults: &DefaultsManager,
    config: &mut crate::hive_config::HealthCheck,
) {
    let plugin_id = format!("hive.health.{}", config.check_type);

    if let Some(type_config) = config.config.get_mut(&config.check_type) {
        let merged = defaults.merge_with(&plugin_id, type_config);
        *type_config = merged;
    }
}

/// Apply defaults to rollout config
pub fn apply_rollout_defaults(
    defaults: &DefaultsManager,
    config: &mut crate::hive_config::RolloutConfig,
) {
    let plugin_id = format!("hive.rollout.{}", config.rollout_type);

    if let Some(type_config) = config.config.get_mut(&config.rollout_type) {
        let merged = defaults.merge_with(&plugin_id, type_config);
        *type_config = merged;
    }
}

/// Apply all defaults to a service config
pub fn apply_service_defaults(
    defaults: &DefaultsManager,
    config: &mut crate::hive_config::ServiceConfig,
) {
    // Apply runner defaults
    apply_runner_defaults(defaults, &mut config.runner);

    // Apply environment defaults
    if let Some(env) = &mut config.environment {
        apply_env_defaults(defaults, env);
    }

    // Apply health check defaults
    if let Some(health_config) = &mut config.healthcheck {
        match health_config {
            crate::hive_config::HealthCheckConfig::Single(check) => {
                apply_health_defaults(defaults, check);
            }
            crate::hive_config::HealthCheckConfig::Multiple(checks) => {
                for check in checks {
                    apply_health_defaults(defaults, check);
                }
            }
        }
    }

    // Apply rollout defaults
    if let Some(rollout) = &mut config.rollout {
        apply_rollout_defaults(defaults, rollout);
    }
}

/// Apply defaults to entire hive config
pub fn apply_all_defaults(config: &mut crate::hive_config::HiveConfig) {
    let defaults = DefaultsManager::from_config(&config.defaults);
    debug!(
        plugin_count = defaults.plugin_ids().len(),
        service_count = config.services.len(),
        "Applying all defaults to config"
    );

    for (name, service_config) in config.services.iter_mut() {
        trace!(service = %name, "Applying defaults to service");
        apply_service_defaults(&defaults, service_config);
    }

    // Apply global environment defaults
    if let Some(env) = &mut config.environment {
        trace!("Applying global environment defaults");
        apply_env_defaults(&defaults, env);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_merge_json_simple() {
        let a = json!({"key1": "value1"});
        let b = json!({"key2": "value2"});
        let merged = merge_json(&a, &b);

        assert_eq!(merged, json!({"key1": "value1", "key2": "value2"}));
    }

    #[test]
    fn test_merge_json_override() {
        let a = json!({"key": "old"});
        let b = json!({"key": "new"});
        let merged = merge_json(&a, &b);

        assert_eq!(merged, json!({"key": "new"}));
    }

    #[test]
    fn test_merge_json_nested() {
        let a = json!({
            "outer": {
                "inner1": "a",
                "inner2": "b"
            }
        });
        let b = json!({
            "outer": {
                "inner2": "c",
                "inner3": "d"
            }
        });
        let merged = merge_json(&a, &b);

        assert_eq!(
            merged,
            json!({
                "outer": {
                    "inner1": "a",
                    "inner2": "c",
                    "inner3": "d"
                }
            })
        );
    }

    #[test]
    fn test_defaults_manager() {
        let mut defaults = DefaultsManager::new();
        defaults.set(
            "hive.runner.docker",
            json!({
                "socket": "/var/run/docker.sock"
            }),
        );

        let merged = defaults.merge_with(
            "hive.runner.docker",
            &json!({
                "image": "postgres:15"
            }),
        );

        assert_eq!(
            merged,
            json!({
                "socket": "/var/run/docker.sock",
                "image": "postgres:15"
            })
        );
    }
}
