use crate::hive_config::{
    extract_cmd_health_config, extract_http_health_config, extract_tcp_health_config, HealthCheck,
    HealthCheckConfig, RuntimeContext,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Shared via Arc — updated by check tasks, read by anyone.
pub struct HealthStatus {
    results: Vec<AtomicBool>,
    total: usize,
}

impl HealthStatus {
    pub fn new(total: usize) -> Self {
        let results = (0..total).map(|_| AtomicBool::new(false)).collect();
        Self { results, total }
    }

    pub fn healthy_count(&self) -> usize {
        self.results
            .iter()
            .filter(|slot| slot.load(Ordering::Relaxed))
            .count()
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy_count() == self.total
    }
}

pub struct HealthChecker {
    client: reqwest::Client,
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthChecker {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Spawns independent check tasks per health check; returns a live-updating status handle.
    pub fn start_health_checks(
        &self,
        service_name: &str,
        config: &HealthCheckConfig,
        ports: &HashMap<String, u16>,
    ) -> Arc<HealthStatus> {
        let checks = config.checks();
        let interval = self.parse_interval(&checks);
        let start_period = self.parse_start_period(&checks);
        let status = Arc::new(HealthStatus::new(checks.len()));

        for (i, check) in checks.into_iter().enumerate() {
            let checker = self.client.clone();
            let ports = ports.clone();
            let check = check.clone();
            let status = Arc::clone(&status);
            let name = service_name.to_string();

            tokio::spawn(async move {
                if start_period > Duration::ZERO {
                    debug!(
                        "Waiting {}s start period for {} check {}",
                        start_period.as_secs(),
                        name,
                        check.check_type
                    );
                    tokio::time::sleep(start_period).await;
                }

                let checker = HealthChecker { client: checker };
                run_check_loop(&checker, &name, &check, &ports, &status.results[i], interval)
                    .await;
            });
        }

        status
    }

    pub async fn run_single_check(
        &self,
        check: &HealthCheck,
        ports: &HashMap<String, u16>,
    ) -> Result<bool> {
        let mut runtime_ctx = RuntimeContext::new();
        runtime_ctx.set_ports(ports.clone());

        match check.check_type.as_str() {
            "http" => self.check_http(check, &runtime_ctx).await,
            "tcp" => self.check_tcp(check, &runtime_ctx).await,
            "cmd" => self.check_cmd(check).await,
            other => Err(anyhow!("Unknown health check type: {}", other)),
        }
    }

    async fn check_http(&self, check: &HealthCheck, runtime_ctx: &RuntimeContext) -> Result<bool> {
        let config = extract_http_health_config(check)?;

        let port_str = runtime_ctx.interpolate(&config.port)?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| anyhow!("Invalid port: {}", port_str))?;

        let url = format!("http://127.0.0.1:{}{}", port, config.path);

        let timeout = config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(Duration::from_secs(5));

        let request = match config.method.to_uppercase().as_str() {
            "GET" => self.client.get(&url),
            "HEAD" => self.client.head(&url),
            "POST" => self.client.post(&url),
            _ => self.client.get(&url),
        };

        match request.timeout(timeout).send().await {
            Ok(response) => {
                let status = response.status();
                let expected = config.status.unwrap_or(200);

                // 200 means "any 2xx", specific code means exact match
                let is_healthy = if expected == 200 {
                    status.is_success()
                } else {
                    status.as_u16() == expected
                };

                Ok(is_healthy)
            }
            Err(e) => {
                debug!("HTTP health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn check_tcp(&self, check: &HealthCheck, runtime_ctx: &RuntimeContext) -> Result<bool> {
        let config = extract_tcp_health_config(check)?;

        let port_str = runtime_ctx.interpolate(&config.port)?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| anyhow!("Invalid port: {}", port_str))?;

        let timeout = config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(Duration::from_secs(5));

        let addr = format!("127.0.0.1:{}", port);

        match tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(mut stream)) => {
                let _ = stream.shutdown().await;
                Ok(true)
            }
            Ok(Err(e)) => {
                debug!("TCP health check failed to connect: {}", e);
                Ok(false)
            }
            Err(_) => {
                debug!("TCP health check timed out");
                Ok(false)
            }
        }
    }

    async fn check_cmd(&self, check: &HealthCheck) -> Result<bool> {
        let config = extract_cmd_health_config(check)?;

        let timeout = config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(Duration::from_secs(30));

        let shell = lib_plugin_abi_v3::utils::resolve_shell(None, true);

        let mut cmd = Command::new(&shell.program);
        cmd.arg(shell.flag).arg(&config.command);

        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        }

        match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(output)) => Ok(output.status.success()),
            Ok(Err(e)) => {
                debug!("Command health check failed: {}", e);
                Ok(false)
            }
            Err(_) => {
                debug!("Command health check timed out");
                Ok(false)
            }
        }
    }

    pub async fn check_cmd_in_container(
        &self,
        check: &HealthCheck,
        container_name: &str,
    ) -> Result<bool> {
        let config = extract_cmd_health_config(check)?;

        let timeout = config
            .timeout
            .as_ref()
            .and_then(|t| parse_duration(t))
            .unwrap_or(Duration::from_secs(30));

        let mut cmd = Command::new("docker");
        cmd.args(["exec", container_name, "sh", "-c", &config.command]);

        match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(output)) => Ok(output.status.success()),
            Ok(Err(e)) => {
                debug!("Docker exec health check failed: {}", e);
                Ok(false)
            }
            Err(_) => {
                debug!("Docker exec health check timed out");
                Ok(false)
            }
        }
    }

    fn parse_interval(&self, checks: &[&HealthCheck]) -> Duration {
        for check in checks {
            if let Some(interval) = check
                .config
                .get(&check.check_type)
                .and_then(|c| c.get("interval"))
                .and_then(|v| v.as_str())
                .and_then(|s| parse_duration(s))
            {
                return interval;
            }
        }
        Duration::from_secs(10)
    }

    fn parse_start_period(&self, checks: &[&HealthCheck]) -> Duration {
        for check in checks {
            if let Some(period) = check
                .config
                .get(&check.check_type)
                .and_then(|c| c.get("start_period"))
                .and_then(|v| v.as_str())
                .and_then(|s| parse_duration(s))
            {
                return period;
            }
        }
        Duration::ZERO
    }

    pub async fn check_once(
        &self,
        config: &HealthCheckConfig,
        ports: &HashMap<String, u16>,
    ) -> Result<bool> {
        self.check_once_with_container(config, ports, None).await
    }

    pub async fn check_once_with_container(
        &self,
        config: &HealthCheckConfig,
        ports: &HashMap<String, u16>,
        container_name: Option<&str>,
    ) -> Result<bool> {
        let checks = config.checks();

        for check in checks {
            let result = if check.check_type == "cmd" && container_name.is_some() {
                // cmd checks in containers run inside the container
                self.check_cmd_in_container(check, container_name.unwrap())
                    .await
            } else {
                self.run_single_check(check, ports).await
            };

            match result {
                Ok(true) => continue,
                Ok(false) => return Ok(false),
                Err(e) => {
                    debug!("Health check error: {}", e);
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
}

/// Runs one check forever, writes result to its atomic slot.
async fn run_check_loop(
    checker: &HealthChecker,
    service_name: &str,
    check: &HealthCheck,
    ports: &HashMap<String, u16>,
    slot: &AtomicBool,
    interval: Duration,
) {
    loop {
        let ok = checker
            .run_single_check(check, ports)
            .await
            .unwrap_or(false);

        let was = slot.swap(ok, Ordering::Relaxed);
        if ok && !was {
            info!(
                "Health check {} now passing for {}",
                check.check_type, service_name
            );
        } else if !ok && was {
            warn!(
                "Health check {} now failing for {}",
                check.check_type, service_name
            );
        }

        tokio::time::sleep(interval).await;
    }
}

pub use lib_plugin_abi_v3::utils::parse_duration;
