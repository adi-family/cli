//! Common types used across plugins

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Runtime context passed to plugins during execution
#[derive(Debug, Clone, Default)]
pub struct RuntimeContext {
    /// Allocated ports by name
    pub ports: HashMap<String, u16>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Working directory
    pub working_dir: PathBuf,
    /// Service name
    pub service_name: String,
}

impl RuntimeContext {
    /// Create a new runtime context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the service name
    pub fn with_service(mut self, name: &str) -> Self {
        self.service_name = name.to_string();
        self
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = dir;
        self
    }

    /// Set ports
    pub fn with_ports(mut self, ports: HashMap<String, u16>) -> Self {
        self.ports = ports;
        self
    }

    /// Set environment variables
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Interpolate a string with runtime values
    /// Supports: {{runtime.port.X}}, {{env.X}}, {{service}}
    pub fn interpolate(&self, input: &str) -> anyhow::Result<String> {
        let mut result = input.to_string();

        // Replace {{runtime.port.X}}
        while let Some(start) = result.find("{{runtime.port.") {
            let end = result[start..]
                .find("}}")
                .map(|i| start + i + 2)
                .ok_or_else(|| anyhow::anyhow!("Unclosed template: {}", input))?;

            let key = &result[start + 15..end - 2];
            let port = self
                .ports
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("Unknown port: {}", key))?;

            result = format!("{}{}{}", &result[..start], port, &result[end..]);
        }

        // Replace {{env.X}}
        while let Some(start) = result.find("{{env.") {
            let end = result[start..]
                .find("}}")
                .map(|i| start + i + 2)
                .ok_or_else(|| anyhow::anyhow!("Unclosed template: {}", input))?;

            let key = &result[start + 6..end - 2];
            let value = self
                .env
                .get(key)
                .cloned()
                .or_else(|| std::env::var(key).ok())
                .unwrap_or_default();

            result = format!("{}{}{}", &result[..start], value, &result[end..]);
        }

        // Replace {{service}}
        result = result.replace("{{service}}", &self.service_name);

        Ok(result)
    }
}

/// Service configuration passed to plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    /// Plugin-specific configuration
    pub config: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_port() {
        let mut ctx = RuntimeContext::new();
        ctx.ports.insert("main".to_string(), 8080);

        let result = ctx
            .interpolate("http://localhost:{{runtime.port.main}}")
            .unwrap();
        assert_eq!(result, "http://localhost:8080");
    }

    #[test]
    fn test_interpolate_service() {
        let ctx = RuntimeContext::new().with_service("my-service");

        let result = ctx.interpolate("hive-{{service}}").unwrap();
        assert_eq!(result, "hive-my-service");
    }
}
