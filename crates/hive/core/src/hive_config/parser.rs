//! Hive YAML Configuration Parser
//!
//! Parses hive.yaml files and applies parse-time variable interpolation.

use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::interpolation::{interpolate_json_value, ParseContext, ParsePlugin};
use super::types::*;

pub const HIVE_YAML_PATH: &str = ".adi/hive.yaml";

pub struct HiveConfigParser {
    project_root: PathBuf,
    parse_context: ParseContext,
}

impl HiveConfigParser {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            parse_context: ParseContext::new(),
        }
    }

    pub fn register_plugin(&mut self, plugin: Box<dyn ParsePlugin>) {
        self.parse_context.register_plugin(plugin);
    }

    pub fn config_path(&self) -> PathBuf {
        self.project_root.join(HIVE_YAML_PATH)
    }

    pub fn config_exists(&self) -> bool {
        self.config_path().exists()
    }

    pub fn parse(&self) -> Result<HiveConfig> {
        let config_path = self.config_path();
        self.parse_file(&config_path)
    }

    pub fn parse_file(&self, path: &Path) -> Result<HiveConfig> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        self.parse_str(&content)
    }

    pub fn parse_str(&self, content: &str) -> Result<HiveConfig> {
        let raw_value: serde_json::Value =
            serde_yml::from_str(content).context("Failed to parse YAML")?;

        let dotenv_vars = self.load_dotenv_files(&raw_value)?;

        let mut interpolated = raw_value;
        self.interpolate_value_with_dotenv(&mut interpolated, None, &dotenv_vars)?;

        let config: HiveConfig =
            serde_json::from_value(interpolated).context("Failed to deserialize config")?;

        if config.version != "1" {
            return Err(anyhow!(
                "Unsupported hive.yaml version: {}. Expected \"1\"",
                config.version
            ));
        }

        Ok(config)
    }

    /// Loads into a HashMap instead of process env to avoid stale cached
    /// values when the daemon is long-running and .env files change.
    fn load_dotenv_files(&self, raw_value: &serde_json::Value) -> Result<HashMap<String, String>> {
        let mut dotenv_vars = HashMap::new();

        let dotenv_files = raw_value
            .get("environment")
            .and_then(|env| env.get("dotenv"))
            .and_then(|dotenv| dotenv.get("files"))
            .and_then(|files| files.as_array());

        let Some(files) = dotenv_files else {
            return Ok(dotenv_vars);
        };

        for file_value in files {
            let Some(file_path) = file_value.as_str() else {
                continue;
            };

            let full_path = if std::path::Path::new(file_path).is_absolute() {
                std::path::PathBuf::from(file_path)
            } else {
                self.project_root.join(file_path)
            };

            if !full_path.exists() {
                tracing::debug!("Dotenv file not found (skipping): {}", full_path.display());
                continue;
            }

            let content = std::fs::read_to_string(&full_path)
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

                    // Explicit env vars take precedence over dotenv
                    if std::env::var(key).is_err() {
                        dotenv_vars.insert(key.to_string(), value.to_string());
                        tracing::trace!("Loaded from dotenv: {}={}", key, value);
                    }
                }
            }

            tracing::debug!("Loaded dotenv file: {}", full_path.display());
        }

        Ok(dotenv_vars)
    }

    fn interpolate_value_with_dotenv(
        &self,
        value: &mut serde_json::Value,
        service_name: Option<&str>,
        dotenv_vars: &HashMap<String, String>,
    ) -> Result<()> {
        let mut ctx = ParseContext::with_dotenv(dotenv_vars.clone());

        if let Some(name) = service_name {
            ctx.set_service_name(name);
        }

        interpolate_json_value(value, &ctx)
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn resolve_path(&self, relative: &str) -> PathBuf {
        if Path::new(relative).is_absolute() {
            PathBuf::from(relative)
        } else {
            self.project_root.join(relative)
        }
    }
}

pub fn find_project_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        if current.join(HIVE_YAML_PATH).exists() {
            return Some(current);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

pub fn extract_script_config(runner: &RunnerConfig) -> Result<ScriptRunnerConfig> {
    let script_value = runner
        .config
        .get("script")
        .ok_or_else(|| anyhow!("Missing 'script' configuration for script runner"))?;

    serde_json::from_value(script_value.clone())
        .context("Failed to parse script runner configuration")
}

pub fn extract_docker_config(runner: &RunnerConfig) -> Result<DockerRunnerConfig> {
    let docker_value = runner
        .config
        .get("docker")
        .ok_or_else(|| anyhow!("Missing 'docker' configuration for docker runner"))?;

    serde_json::from_value(docker_value.clone())
        .context("Failed to parse docker runner configuration")
}

pub fn extract_recreate_config(rollout: &RolloutConfig) -> Result<RecreateRolloutConfig> {
    let recreate_value = rollout
        .config
        .get("recreate")
        .ok_or_else(|| anyhow!("Missing 'recreate' configuration for recreate rollout"))?;

    serde_json::from_value(recreate_value.clone())
        .context("Failed to parse recreate rollout configuration")
}

pub fn extract_blue_green_config(rollout: &RolloutConfig) -> Result<BlueGreenRolloutConfig> {
    let bg_value = rollout
        .config
        .get(ROLLOUT_TYPE_BLUE_GREEN)
        .ok_or_else(|| anyhow!("Missing 'blue-green' configuration for blue-green rollout"))?;

    serde_json::from_value(bg_value.clone())
        .context("Failed to parse blue-green rollout configuration")
}

pub fn extract_http_health_config(health: &HealthCheck) -> Result<HttpHealthCheckConfig> {
    let http_value = health
        .config
        .get("http")
        .ok_or_else(|| anyhow!("Missing 'http' configuration for http health check"))?;

    serde_json::from_value(http_value.clone())
        .context("Failed to parse http health check configuration")
}

pub fn extract_tcp_health_config(health: &HealthCheck) -> Result<TcpHealthCheckConfig> {
    let tcp_value = health
        .config
        .get("tcp")
        .ok_or_else(|| anyhow!("Missing 'tcp' configuration for tcp health check"))?;

    serde_json::from_value(tcp_value.clone())
        .context("Failed to parse tcp health check configuration")
}

pub fn extract_cmd_health_config(health: &HealthCheck) -> Result<CmdHealthCheckConfig> {
    let cmd_value = health
        .config
        .get("cmd")
        .ok_or_else(|| anyhow!("Missing 'cmd' configuration for cmd health check"))?;

    serde_json::from_value(cmd_value.clone())
        .context("Failed to parse cmd health check configuration")
}

pub fn get_rollout_ports(rollout: &RolloutConfig) -> Result<HashMap<String, u16>> {
    let mut ports = HashMap::new();

    match rollout.rollout_type.as_str() {
        ROLLOUT_TYPE_RECREATE => {
            let config = extract_recreate_config(rollout)?;
            for (name, value) in config.ports {
                ports.insert(name, value.get_port());
            }
        }
        ROLLOUT_TYPE_BLUE_GREEN => {
            let config = extract_blue_green_config(rollout)?;
            for (name, bg_port) in config.ports {
                ports.insert(name, bg_port.blue);
            }
        }
        other => {
            if let Some(ports_value) = rollout
                .config
                .get(&rollout.rollout_type)
                .and_then(|v| v.get("ports"))
            {
                if let Some(ports_obj) = ports_value.as_object() {
                    for (name, value) in ports_obj {
                        if let Some(port) = value.as_u64() {
                            ports.insert(name.clone(), port as u16);
                        }
                    }
                }
            } else {
                tracing::warn!("Unknown rollout type: {}", other);
            }
        }
    }

    Ok(ports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
version: "1"

services:
  web:
    runner:
      type: script
      script:
        run: npm run dev
    rollout:
      type: recreate
      recreate:
        ports:
          http: 3000
    proxy:
      path: /
"#;

        let parser = HiveConfigParser::new(".");
        let config = parser.parse_str(yaml).unwrap();

        assert_eq!(config.version, "1");
        assert!(config.services.contains_key("web"));

        let web = &config.services["web"];
        assert_eq!(web.runner.runner_type, "script");
    }

    #[test]
    fn test_parse_with_env_interpolation() {
        std::env::set_var("TEST_PORT", "8080");

        let yaml = r#"
version: "1"

services:
  api:
    runner:
      type: script
      script:
        run: cargo run
    rollout:
      type: recreate
      recreate:
        ports:
          http: 8080
    environment:
      static:
        PORT: "${env.TEST_PORT}"
"#;

        let parser = HiveConfigParser::new(".");
        let config = parser.parse_str(yaml).unwrap();

        let api = &config.services["api"];
        let env = api.environment.as_ref().unwrap();
        let static_env = env.static_env.as_ref().unwrap();

        assert_eq!(static_env.get("PORT"), Some(&"8080".to_string()));

        std::env::remove_var("TEST_PORT");
    }

    #[test]
    fn test_parse_multiple_proxies() {
        let yaml = r#"
version: "1"

services:
  gateway:
    runner:
      type: script
      script:
        run: cargo run
    rollout:
      type: recreate
      recreate:
        ports:
          http: 8080
          grpc: 9090
    proxy:
      - host: api.example.com
        path: /v1
        port: "{{runtime.port.http}}"
      - host: grpc.example.com
        path: /
        port: "{{runtime.port.grpc}}"
"#;

        let parser = HiveConfigParser::new(".");
        let config = parser.parse_str(yaml).unwrap();

        let gateway = &config.services["gateway"];
        if let Some(ServiceProxyConfig::Multiple(proxies)) = &gateway.proxy {
            assert_eq!(proxies.len(), 2);
            assert_eq!(proxies[0].host, Some("api.example.com".to_string()));
            assert_eq!(proxies[1].host, Some("grpc.example.com".to_string()));
        } else {
            panic!("Expected multiple proxies");
        }
    }

    #[test]
    fn test_parse_multiple_healthchecks() {
        let yaml = r#"
version: "1"

services:
  api:
    runner:
      type: script
      script:
        run: cargo run
    rollout:
      type: recreate
      recreate:
        ports:
          http: 8080
    healthcheck:
      - type: http
        http:
          port: "{{runtime.port.http}}"
          path: /health
      - type: tcp
        tcp:
          port: "{{runtime.port.http}}"
"#;

        let parser = HiveConfigParser::new(".");
        let config = parser.parse_str(yaml).unwrap();

        let api = &config.services["api"];
        if let Some(HealthCheckConfig::Multiple(checks)) = &api.healthcheck {
            assert_eq!(checks.len(), 2);
            assert_eq!(checks[0].check_type, "http");
            assert_eq!(checks[1].check_type, "tcp");
        } else {
            panic!("Expected multiple health checks");
        }
    }
}
