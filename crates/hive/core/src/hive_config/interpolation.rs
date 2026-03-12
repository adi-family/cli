//! Variable Interpolation
//!
//! Handles two types of variable interpolation:
//! 1. Parse-time plugins (`${plugin.key}`) - resolved when YAML is parsed
//! 2. Runtime templates (`{{runtime...}}`) - resolved at service start

use anyhow::{anyhow, Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::{debug, trace, warn};

static PARSE_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\.([^}:]+)(?::-([^}]*))?\}").unwrap()
});

static RUNTIME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{runtime\.port\.([a-zA-Z_][a-zA-Z0-9_]*)\}\}").unwrap());

static USES_PORT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{uses\.([a-zA-Z_][a-zA-Z0-9_]*)\.port\.([a-zA-Z_][a-zA-Z0-9_]*)\}\}").unwrap()
});

static ESCAPED_DOLLAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\$\{").unwrap());
static ESCAPED_BRACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{\{").unwrap());

pub trait ParsePlugin: Send + Sync {
    fn name(&self) -> &str;
    fn resolve(&self, key: &str) -> Result<Option<String>>;
}

/// Resolves from dotenv overrides first, then process environment.
pub struct EnvParsePlugin {
    dotenv_vars: HashMap<String, String>,
}

impl Default for EnvParsePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvParsePlugin {
    pub fn new() -> Self {
        Self {
            dotenv_vars: HashMap::new(),
        }
    }

    pub fn with_dotenv(dotenv_vars: HashMap<String, String>) -> Self {
        Self { dotenv_vars }
    }
}

impl ParsePlugin for EnvParsePlugin {
    fn name(&self) -> &str {
        "env"
    }

    fn resolve(&self, key: &str) -> Result<Option<String>> {
        if let Some(value) = self.dotenv_vars.get(key) {
            trace!(key = %key, source = "dotenv", "Resolved env variable");
            return Ok(Some(value.clone()));
        }
        let value = std::env::var(key).ok();
        if value.is_some() {
            trace!(key = %key, source = "process_env", "Resolved env variable");
        } else {
            trace!(key = %key, "Env variable not found");
        }
        Ok(value)
    }
}

pub struct ServiceParsePlugin {
    service_name: String,
}

impl ServiceParsePlugin {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }
}

impl ParsePlugin for ServiceParsePlugin {
    fn name(&self) -> &str {
        "service"
    }

    fn resolve(&self, key: &str) -> Result<Option<String>> {
        match key {
            "name" => Ok(Some(self.service_name.clone())),
            _ => Ok(None),
        }
    }
}

pub struct ParseContext {
    plugins: HashMap<String, Box<dyn ParsePlugin>>,
}

impl Default for ParseContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ParseContext {
    pub fn new() -> Self {
        let mut ctx = Self {
            plugins: HashMap::new(),
        };
        ctx.register_plugin(Box::new(EnvParsePlugin::new()));
        ctx
    }

    pub fn with_dotenv(dotenv_vars: HashMap<String, String>) -> Self {
        let mut ctx = Self {
            plugins: HashMap::new(),
        };
        ctx.register_plugin(Box::new(EnvParsePlugin::with_dotenv(dotenv_vars)));
        ctx
    }

    pub fn register_plugin(&mut self, plugin: Box<dyn ParsePlugin>) {
        self.plugins.insert(plugin.name().to_string(), plugin);
    }

    pub fn set_service_name(&mut self, name: &str) {
        self.plugins.insert(
            "service".to_string(),
            Box::new(ServiceParsePlugin::new(name)),
        );
    }

    pub fn interpolate(&self, input: &str) -> Result<String> {
        let escaped = ESCAPED_DOLLAR.replace_all(input, "\x00DOLLAR\x00");
        let escaped = ESCAPED_BRACE.replace_all(&escaped, "\x00BRACE\x00");

        let result = escaped.to_string();
        let mut last_end = 0;
        let mut output = String::new();

        for cap in PARSE_TIME_REGEX.captures_iter(&result) {
            let full_match = cap.get(0).unwrap();
            let plugin_name = cap.get(1).unwrap().as_str();
            let key = cap.get(2).unwrap().as_str();
            let default = cap.get(3).map(|m| m.as_str());

            output.push_str(&result[last_end..full_match.start()]);

            let value = if let Some(plugin) = self.plugins.get(plugin_name) {
                plugin
                    .resolve(key)
                    .with_context(|| format!("Failed to resolve ${{{}.{}}}", plugin_name, key))?
            } else {
                warn!(plugin = %plugin_name, key = %key, "Unknown parse plugin referenced");
                None
            };

            match (value, default) {
                (Some(ref v), _) => {
                    trace!(plugin = %plugin_name, key = %key, "Resolved parse-time variable");
                    output.push_str(v);
                }
                (None, Some(d)) => {
                    debug!(plugin = %plugin_name, key = %key, default = %d, "Using default value for variable");
                    output.push_str(d);
                }
                (None, None) => {
                    warn!(plugin = %plugin_name, key = %key, "Unresolved variable with no default");
                    return Err(anyhow!(
                        "Unresolved variable: ${{{}.{}}} (no default provided)",
                        plugin_name,
                        key
                    ));
                }
            }

            last_end = full_match.end();
        }

        output.push_str(&result[last_end..]);

        let output = output.replace("\x00DOLLAR\x00", "${");
        let output = output.replace("\x00BRACE\x00", "{{");

        Ok(output)
    }
}

pub struct RuntimeContext {
    ports: HashMap<String, u16>,
    uses_ports: HashMap<String, HashMap<String, u16>>,
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeContext {
    pub fn new() -> Self {
        Self {
            ports: HashMap::new(),
            uses_ports: HashMap::new(),
        }
    }

    pub fn set_port(&mut self, name: &str, port: u16) {
        self.ports.insert(name.to_string(), port);
    }

    pub fn set_ports(&mut self, ports: HashMap<String, u16>) {
        self.ports = ports;
    }

    pub fn add_uses_ports(&mut self, alias: &str, ports: HashMap<String, u16>) {
        self.uses_ports.insert(alias.to_string(), ports);
    }

    pub fn get_port(&self, name: &str) -> Option<u16> {
        self.ports.get(name).copied()
    }

    pub fn interpolate(&self, input: &str) -> Result<String> {
        let escaped = ESCAPED_BRACE.replace_all(input, "\x00BRACE\x00");
        let mut result = escaped.to_string();

        let mut last_end = 0;
        let mut output = String::new();

        for cap in RUNTIME_REGEX.captures_iter(&result.clone()) {
            let full_match = cap.get(0).unwrap();
            let port_name = cap.get(1).unwrap().as_str();

            output.push_str(&result[last_end..full_match.start()]);

            if let Some(port) = self.ports.get(port_name) {
                trace!(port_name = %port_name, port = %port, "Resolved runtime port");
                output.push_str(&port.to_string());
            } else {
                warn!(port_name = %port_name, "Unresolved runtime port");
                return Err(anyhow!(
                    "Unresolved runtime port: {{{{runtime.port.{}}}}}",
                    port_name
                ));
            }

            last_end = full_match.end();
        }
        output.push_str(&result[last_end..]);
        result = output;

        let mut last_end = 0;
        let mut output = String::new();

        for cap in USES_PORT_REGEX.captures_iter(&result.clone()) {
            let full_match = cap.get(0).unwrap();
            let alias = cap.get(1).unwrap().as_str();
            let port_name = cap.get(2).unwrap().as_str();

            output.push_str(&result[last_end..full_match.start()]);

            if let Some(ports) = self.uses_ports.get(alias) {
                if let Some(port) = ports.get(port_name) {
                    trace!(alias = %alias, port_name = %port_name, port = %port, "Resolved uses port");
                    output.push_str(&port.to_string());
                } else {
                    warn!(alias = %alias, port_name = %port_name, "Unresolved uses port");
                    return Err(anyhow!(
                        "Unresolved uses port: {{{{uses.{}.port.{}}}}}",
                        alias,
                        port_name
                    ));
                }
            } else {
                warn!(alias = %alias, "Unknown uses alias");
                return Err(anyhow!(
                    "Unknown uses alias: {} in {{{{uses.{}.port.{}}}}}",
                    alias,
                    alias,
                    port_name
                ));
            }

            last_end = full_match.end();
        }
        output.push_str(&result[last_end..]);

        let output = output.replace("\x00BRACE\x00", "{{");

        Ok(output)
    }
}

pub fn interpolate_json_value(
    value: &mut serde_json::Value,
    parse_ctx: &ParseContext,
) -> Result<()> {
    match value {
        serde_json::Value::String(s) => {
            *s = parse_ctx.interpolate(s)?;
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                interpolate_json_value(item, parse_ctx)?;
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                interpolate_json_value(v, parse_ctx)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_interpolation() {
        std::env::set_var("TEST_VAR", "hello");
        let ctx = ParseContext::new();

        let result = ctx.interpolate("value: ${env.TEST_VAR}").unwrap();
        assert_eq!(result, "value: hello");

        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_default_value() {
        let ctx = ParseContext::new();

        let result = ctx
            .interpolate("value: ${env.NONEXISTENT:-default}")
            .unwrap();
        assert_eq!(result, "value: default");
    }

    #[test]
    fn test_escaped_dollar() {
        let ctx = ParseContext::new();

        let result = ctx.interpolate("shell: $${NOT_INTERPOLATED}").unwrap();
        assert_eq!(result, "shell: ${NOT_INTERPOLATED}");
    }

    #[test]
    fn test_runtime_port_interpolation() {
        let mut ctx = RuntimeContext::new();
        ctx.set_port("http", 8080);
        ctx.set_port("grpc", 9090);

        let result = ctx
            .interpolate("http://localhost:{{runtime.port.http}}/api")
            .unwrap();
        assert_eq!(result, "http://localhost:8080/api");
    }

    #[test]
    fn test_uses_port_interpolation() {
        let mut ctx = RuntimeContext::new();
        let mut pg_ports = HashMap::new();
        pg_ports.insert("db".to_string(), 5432);
        ctx.add_uses_ports("pg", pg_ports);

        let result = ctx.interpolate("port: {{uses.pg.port.db}}").unwrap();
        assert_eq!(result, "port: 5432");
    }

    #[test]
    fn test_service_name_plugin() {
        let mut ctx = ParseContext::new();
        ctx.set_service_name("auth");

        let result = ctx.interpolate("name: ${service.name}").unwrap();
        assert_eq!(result, "name: auth");
    }
}
