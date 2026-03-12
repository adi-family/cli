mod env_plugins;
mod environment;
mod health;
mod process;
mod rollout;

pub use env_plugins::*;
pub use environment::*;
pub use health::*;
pub use process::*;
pub use rollout::*;

use crate::hive_config::{
    get_rollout_ports, topological_sort, topological_sort_levels, HiveConfig, RestartPolicy,
    RuntimeContext, ServiceConfig, ServiceInfo, ServiceState, ROLLOUT_TYPE_BLUE_GREEN,
};
use crate::observability::{
    EventCollector, LogLevel, LogStream, ObservabilityEvent, ServiceEventType,
};
use crate::plugins::plugin_manager;
use crate::runtime_db::RuntimeDb;
use crate::service_proxy::ServiceProxyState;
use anyhow::{anyhow, Context, Result};
use futures::future::join_all;
use lib_plugin_abi_v3::hooks::{HookContext, HookEvent, HookExecutor, HookOutputStream};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Returns the Docker container name for a service.
///
/// Uses `container_name` from the docker runner config if set,
/// otherwise falls back to `hive-<service_name>`.
fn docker_container_name(service_name: &str, service_config: &ServiceConfig) -> String {
    service_config
        .runner
        .config
        .get("docker")
        .and_then(|d| d.get("container_name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("hive-{}", service_name))
}

/// Recursively resolve `{{runtime.port.X}}` references in a JSON config value.
///
/// The hive config uses `{{runtime.port.X}}` syntax for port interpolation, while
/// the plugin ABI uses `${PORT:X}` syntax. Pre-interpolating the config ensures
/// plugins receive literal port numbers regardless of their interpolation format.
fn pre_interpolate_config(value: &mut serde_json::Value, ctx: &RuntimeContext) {
    match value {
        serde_json::Value::String(s) => {
            if let Ok(interpolated) = ctx.interpolate(s) {
                *s = interpolated;
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                pre_interpolate_config(item, ctx);
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values_mut() {
                pre_interpolate_config(v, ctx);
            }
        }
        _ => {}
    }
}

fn resolve_service_shell(config: &ServiceConfig) -> String {
    let shell_from_config = crate::hive_config::extract_script_config(&config.runner)
        .ok()
        .and_then(|sc| sc.shell);
    resolve_shell(shell_from_config.as_deref())
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServicePhase {
    WaitingFor(String),
    PreHooks,
    Building,
    Starting,
    PostHooks,
    Running,
    Failed(String),
}

#[derive(Clone)]
pub struct ServiceManager {
    project_root: PathBuf,
    config: HiveConfig,
    services: Arc<RwLock<HashMap<String, ServiceRuntime>>>,
    process_manager: Arc<ProcessManager>,
    health_checker: Arc<HealthChecker>,
    env_resolver: Arc<EnvironmentResolver>,
    rollout_manager: Arc<RolloutManager>,
    proxy_state: Arc<ServiceProxyState>,
    event_collector: Option<Arc<EventCollector>>,
    source_name: String,
}

pub struct ServiceRuntime {
    pub name: String,
    pub state: ServiceState,
    pub process: Option<ProcessHandle>,
    pub ports: HashMap<String, u16>,
    pub health: Option<Arc<HealthStatus>>,
    pub restart_count: u32,
    pub last_error: Option<String>,
}

impl ServiceRuntime {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: ServiceState::Stopped,
            process: None,
            ports: HashMap::new(),
            health: None,
            restart_count: 0,
            last_error: None,
        }
    }

    fn to_info(&self) -> ServiceInfo {
        ServiceInfo {
            name: self.name.clone(),
            state: self.state.clone(),
            pid: self.process.as_ref().and_then(|p| p.pid()),
            container_id: None,
            ports: self.ports.clone(),
            healthy: self.health.as_ref().map(|h| h.is_healthy()),
            last_error: self.last_error.clone(),
            restart_count: self.restart_count,
        }
    }
}

impl ServiceManager {
    pub fn new(project_root: impl AsRef<Path>, config: HiveConfig) -> Result<Self> {
        let proxy_state = Arc::new(ServiceProxyState::from_config(&config));
        let rollout_manager = Arc::new(RolloutManager::new(proxy_state.clone()));

        let mut env_resolver = EnvironmentResolver::new();
        env_resolver.register(Box::new(DotenvPlugin::new(project_root.as_ref())));

        let runtime_db = Arc::new(RuntimeDb::open(project_root.as_ref())?);
        let process_manager = Arc::new(ProcessManager::new(
            project_root.as_ref().to_path_buf(),
            runtime_db,
        ));

        Ok(Self {
            project_root: project_root.as_ref().to_path_buf(),
            config,
            services: Arc::new(RwLock::new(HashMap::new())),
            process_manager,
            health_checker: Arc::new(HealthChecker::new()),
            env_resolver: Arc::new(env_resolver),
            rollout_manager,
            proxy_state,
            event_collector: None,
            source_name: "default".to_string(),
        })
    }

    pub fn with_proxy_state(
        project_root: impl AsRef<Path>,
        config: HiveConfig,
        proxy_state: Arc<ServiceProxyState>,
    ) -> Result<Self> {
        let rollout_manager = Arc::new(RolloutManager::new(proxy_state.clone()));

        let mut env_resolver = EnvironmentResolver::new();
        env_resolver.register(Box::new(DotenvPlugin::new(project_root.as_ref())));

        let runtime_db = Arc::new(RuntimeDb::open(project_root.as_ref())?);
        let process_manager = Arc::new(ProcessManager::new(
            project_root.as_ref().to_path_buf(),
            runtime_db,
        ));

        Ok(Self {
            project_root: project_root.as_ref().to_path_buf(),
            config,
            services: Arc::new(RwLock::new(HashMap::new())),
            process_manager,
            health_checker: Arc::new(HealthChecker::new()),
            env_resolver: Arc::new(env_resolver),
            rollout_manager,
            proxy_state,
            event_collector: None,
            source_name: "default".to_string(),
        })
    }

    pub fn with_observability(
        project_root: impl AsRef<Path>,
        config: HiveConfig,
        proxy_state: Arc<ServiceProxyState>,
        event_collector: Arc<EventCollector>,
        source_name: String,
    ) -> Result<Self> {
        let rollout_manager = Arc::new(RolloutManager::new(proxy_state.clone()));

        let mut env_resolver = EnvironmentResolver::new();
        env_resolver.register(Box::new(DotenvPlugin::new(project_root.as_ref())));

        let runtime_db = Arc::new(RuntimeDb::open(project_root.as_ref())?);

        let process_manager = Arc::new(ProcessManager::with_event_collector(
            project_root.as_ref().to_path_buf(),
            runtime_db,
            event_collector.clone(),
            source_name.clone(),
        ));

        Ok(Self {
            project_root: project_root.as_ref().to_path_buf(),
            config,
            services: Arc::new(RwLock::new(HashMap::new())),
            process_manager,
            health_checker: Arc::new(HealthChecker::new()),
            env_resolver: Arc::new(env_resolver),
            rollout_manager,
            proxy_state,
            event_collector: Some(event_collector),
            source_name,
        })
    }

    async fn runner_for(
        &self,
        runner_type: &str,
    ) -> Result<Arc<dyn lib_plugin_abi_v3::runner::Runner>> {
        plugin_manager()
            .get_runner(runner_type)
            .await
            .ok_or_else(|| {
                anyhow!(
                    "No runner plugin available for '{}'. \
                     Install with: adi plugin install hive.runner.{}",
                    runner_type,
                    runner_type
                )
            })
    }

    pub fn proxy_state(&self) -> Arc<ServiceProxyState> {
        self.proxy_state.clone()
    }

    pub fn rollout_manager(&self) -> Arc<RolloutManager> {
        self.rollout_manager.clone()
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn config(&self) -> &HiveConfig {
        &self.config
    }

    /// Used by auto-reload when config file changes on disk.
    pub fn update_config(&mut self, config: HiveConfig) {
        self.config = config;
    }

    /// Emit a service lifecycle event to the event collector.
    fn emit_service_event(&self, name: &str, event: ServiceEventType) {
        if let Some(collector) = &self.event_collector {
            let fqn = format!("{}:{}", self.source_name, name);
            collector.emit(ObservabilityEvent::service_event(fqn, event));
        }
    }

    pub async fn start_all(&self) -> Result<()> {
        let order =
            topological_sort(&self.config).context("Failed to determine service start order")?;

        info!("Starting services in order: {:?}", order);

        let runner_types: std::collections::HashSet<&str> = order
            .iter()
            .filter_map(|n| self.config.services.get(n))
            .map(|s| s.runner.runner_type.as_str())
            .collect();

        for runner_type in runner_types {
            if plugin_manager().get_runner(runner_type).await.is_none() {
                return Err(anyhow!(
                    "No runner plugin available for '{}'. \
                     Install with: adi plugin install hive.runner.{}",
                    runner_type, runner_type
                ));
            }
        }

        for service_name in order {
            self.start_service(&service_name).await?;
        }

        Ok(())
    }

    pub async fn start_services(&self, names: &[String]) -> Result<()> {
        let mut needed: std::collections::HashSet<String> = names.iter().cloned().collect();
        let mut to_process: Vec<String> = names.to_vec();

        while let Some(name) = to_process.pop() {
            if let Some(service_config) = self.config.services.get(&name) {
                for dep in &service_config.depends_on {
                    if needed.insert(dep.clone()) {
                        to_process.push(dep.clone());
                    }
                }
            }
        }

        let order = topological_sort(&self.config)?;
        let to_start: Vec<String> = order
            .into_iter()
            .filter(|name| needed.contains(name))
            .collect();

        for service_name in to_start {
            self.start_service(&service_name).await?;
        }

        Ok(())
    }

    pub async fn start_service(&self, name: &str) -> Result<()> {
        self.start_service_with_progress(name, |_| {}).await
    }

    pub async fn start_service_with_progress<F>(&self, name: &str, mut on_progress: F) -> Result<()>
    where
        F: FnMut(ServicePhase),
    {
        let service_config = self
            .config
            .services
            .get(name)
            .ok_or_else(|| anyhow!("Unknown service: {}", name))?;

        let is_blue_green = self.is_blue_green_rollout(service_config);

        if self.reconcile_running_state(name, service_config, is_blue_green).await? {
            return Ok(());
        }

        self.check_port_conflicts(name, service_config).await?;

        self.wait_for_dependencies(name, service_config, &mut on_progress).await?;

        if is_blue_green {
            if let Some(rollout) = &service_config.rollout {
                self.rollout_manager.init_blue_green(name, rollout).await?;
            }
        }

        self.init_service_runtime(name, service_config, is_blue_green).await?;

        let env = self.build_environment_with_error_handling(name, service_config, &mut on_progress).await?;

        self.run_pre_hooks(name, service_config, &env, &mut on_progress).await?;

        self.run_build_step(name, service_config, &env, &mut on_progress).await?;

        on_progress(ServicePhase::Starting);
        let process = self.start_process_with_error_handling(name, service_config, env.clone(), &mut on_progress).await?;

        self.update_runtime_with_process(name, process).await;

        self.setup_health_and_deployment(name, service_config, is_blue_green).await?;

        self.run_post_hooks(name, service_config, &env, &mut on_progress).await?;

        on_progress(ServicePhase::Running);
        info!("Service {} started", name);
        Ok(())
    }

    fn is_blue_green_rollout(&self, service_config: &ServiceConfig) -> bool {
        service_config
            .rollout
            .as_ref()
            .map(|r| r.rollout_type == ROLLOUT_TYPE_BLUE_GREEN)
            .unwrap_or(false)
    }

    /// Returns true if already running (skip start).
    async fn reconcile_running_state(
        &self,
        name: &str,
        service_config: &ServiceConfig,
        is_blue_green: bool,
    ) -> Result<bool> {
        let already_running = {
            let services = self.services.read().await;
            let in_memory_running = services
                .get(name)
                .map(|r| r.state == ServiceState::Running)
                .unwrap_or(false);
            drop(services);

            if in_memory_running {
                let system_running = self.is_service_running_on_system(name, service_config).await;
                if !system_running {
                    warn!("Service {} was marked as running in memory but is not running on system, restarting", name);
                    let mut services = self.services.write().await;
                    if let Some(runtime) = services.get_mut(name) {
                        runtime.state = ServiceState::Stopped;
                        runtime.process = None;
                    }
                    false
                } else {
                    true
                }
            } else {
                self.is_service_running_on_system(name, service_config).await
            }
        };

        // For blue-green, continue even if already running (deploy new instance)
        if already_running && !is_blue_green {
            info!("Service {} is already running", name);
            self.update_running_service_state(name, service_config).await;
            return Ok(true);
        }

        Ok(false)
    }

    async fn update_running_service_state(&self, name: &str, service_config: &ServiceConfig) {
        let mut runtime = ServiceRuntime::new(name);
        runtime.state = ServiceState::Running;
        if let Some(rollout) = &service_config.rollout {
            if let Ok(ports) = get_rollout_ports(rollout) {
                runtime.ports = ports;
            }
        }
        if service_config.runner.runner_type.as_str() == "script" {
            if let Some(pid) = self.process_manager.is_service_running(name) {
                runtime.process = Some(ProcessHandle::from_pid(pid));
            }
        }
        let mut services = self.services.write().await;
        services.insert(name.to_string(), runtime);
    }

    async fn check_port_conflicts(&self, name: &str, service_config: &ServiceConfig) -> Result<()> {
        if service_config.runner.runner_type.as_str() != "script" {
            return Ok(());
        }
        if self.process_manager.is_service_running(name).is_some() {
            return Ok(()); // We already own the port
        }

        if let Some(rollout) = &service_config.rollout {
            if let Ok(ports) = get_rollout_ports(rollout) {
                for (port_name, port) in &ports {
                    if ProcessManager::is_port_in_use(*port) {
                        let mut runtime = ServiceRuntime::new(name);
                        runtime.state = ServiceState::PortConflict;
                        runtime.ports = ports.clone();
                        {
                            let mut services = self.services.write().await;
                            services.insert(name.to_string(), runtime);
                        }
                        return Err(anyhow!(
                            "Cannot start {}: port {} ({}) is already in use by another process. \
                             Stop the conflicting process first, or use a different port.",
                            name, port, port_name
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn init_service_runtime(
        &self,
        name: &str,
        service_config: &ServiceConfig,
        is_blue_green: bool,
    ) -> Result<()> {
        let mut runtime = ServiceRuntime::new(name);
        runtime.state = ServiceState::Starting;

        if let Some(rollout) = &service_config.rollout {
            runtime.ports = if is_blue_green {
                self.rollout_manager.get_deployment_ports(name, rollout).await?
            } else {
                get_rollout_ports(rollout)?
            };
        }

        let mut services = self.services.write().await;
        services.insert(name.to_string(), runtime);
        self.emit_service_event(name, ServiceEventType::Starting);
        Ok(())
    }

    async fn build_environment_with_error_handling<F>(
        &self,
        name: &str,
        service_config: &ServiceConfig,
        on_progress: &mut F,
    ) -> Result<HashMap<String, String>>
    where
        F: FnMut(ServicePhase),
    {
        match self.build_environment(name, service_config).await {
            Ok(env) => Ok(env),
            Err(e) => {
                self.mark_service_crashed(name).await;
                on_progress(ServicePhase::Failed(e.to_string()));
                Err(e)
            }
        }
    }

    async fn run_pre_hooks<F>(
        &self,
        name: &str,
        service_config: &ServiceConfig,
        env: &HashMap<String, String>,
        on_progress: &mut F,
    ) -> Result<()>
    where
        F: FnMut(ServicePhase),
    {
        let has_pre_hooks = service_config
            .hooks
            .as_ref()
            .map(|h| !h.steps_for(HookEvent::PreUp).is_empty())
            .unwrap_or(false);

        if has_pre_hooks {
            on_progress(ServicePhase::PreHooks);
        }

        if let Err(e) = self.run_hooks(HookEvent::PreUp, name, service_config, env).await {
            self.mark_service_crashed(name).await;
            on_progress(ServicePhase::Failed(e.to_string()));
            return Err(e);
        }
        Ok(())
    }

    async fn run_build_step<F>(
        &self,
        name: &str,
        service_config: &ServiceConfig,
        env: &HashMap<String, String>,
        on_progress: &mut F,
    ) -> Result<()>
    where
        F: FnMut(ServicePhase),
    {
        if let Some(build) = &service_config.build {
            let shell = resolve_service_shell(service_config);
            on_progress(ServicePhase::Building);
            if let Err(e) = self.run_build(name, build, env, &shell).await {
                self.mark_service_crashed(name).await;
                on_progress(ServicePhase::Failed(e.to_string()));
                return Err(e);
            }
        }
        Ok(())
    }

    async fn start_process_with_error_handling<F>(
        &self,
        name: &str,
        service_config: &ServiceConfig,
        env: HashMap<String, String>,
        on_progress: &mut F,
    ) -> Result<ProcessHandle>
    where
        F: FnMut(ServicePhase),
    {
        match self.start_process(name, service_config, env).await {
            Ok(p) => Ok(p),
            Err(e) => {
                self.mark_service_crashed(name).await;
                on_progress(ServicePhase::Failed(e.to_string()));
                Err(e)
            }
        }
    }

    async fn update_runtime_with_process(&self, name: &str, process: ProcessHandle) {
        // Persist PID so detect_running_services() works across daemon restarts
        if let Some(pid) = process.pid() {
            let _ = self.process_manager.runtime_db().save_pid(name, pid);
        }
        let mut services = self.services.write().await;
        if let Some(runtime) = services.get_mut(name) {
            runtime.process = Some(process);
            runtime.state = ServiceState::Running;
        }
        self.emit_service_event(name, ServiceEventType::Started);
    }

    async fn setup_health_and_deployment(
        &self,
        name: &str,
        service_config: &ServiceConfig,
        is_blue_green: bool,
    ) -> Result<()> {
        if is_blue_green && service_config.healthcheck.is_some() {
            self.handle_blue_green_deployment(name, service_config).await?;
        } else if service_config.healthcheck.is_some() {
            self.start_health_check(name).await?;
        }
        Ok(())
    }

    async fn run_post_hooks<F>(
        &self,
        name: &str,
        service_config: &ServiceConfig,
        env: &HashMap<String, String>,
        on_progress: &mut F,
    ) -> Result<()>
    where
        F: FnMut(ServicePhase),
    {
        let has_post_hooks = service_config
            .hooks
            .as_ref()
            .map(|h| !h.steps_for(HookEvent::PostUp).is_empty())
            .unwrap_or(false);

        if has_post_hooks {
            on_progress(ServicePhase::PostHooks);
        }

        if let Err(e) = self.run_hooks(HookEvent::PostUp, name, service_config, env).await {
            error!("Post-up hooks failed for service {}: {}", name, e);
            self.stop_service(name).await?;
            on_progress(ServicePhase::Failed(e.to_string()));
            return Err(e);
        }
        Ok(())
    }

    /// Parallel within each dependency level, reverse order.
    pub async fn stop_all(&self) -> Result<()> {
        let mut levels = topological_sort_levels(&self.config)?;
        levels.reverse();

        for level in levels {
            let handles: Vec<_> = level.iter().map(|name| self.stop_service(name)).collect();
            let results = join_all(handles).await;
            for result in results {
                result?;
            }
        }

        Ok(())
    }

    pub async fn stop_service(&self, name: &str) -> Result<()> {
        if let Some(service_config) = self.config.services.get(name) {
            let env = self
                .build_environment(name, service_config)
                .await
                .unwrap_or_default();
            if let Err(e) = self
                .run_hooks(HookEvent::PreDown, name, service_config, &env)
                .await
            {
                // Pre-down defaults to warn, so this only fires on explicit abort
                warn!("Pre-down hooks failed for service {}: {}", name, e);
            }
        }

        let mut services = self.services.write().await;

        if let Some(runtime) = services.get_mut(name) {
            if runtime.state == ServiceState::Stopped {
                return Ok(());
            }

            runtime.state = ServiceState::Stopping;
            self.emit_service_event(name, ServiceEventType::Stopping);

            if let Some(process) = runtime.process.take() {
                let runner_type = self.config.services.get(name)
                    .map(|c| c.runner.runner_type.clone())
                    .unwrap_or_else(|| "script".to_string());
                if let Ok(runner) = self.runner_for(&runner_type).await {
                    let mut abi_handle = lib_plugin_abi_v3::runner::ProcessHandle::from(process);
                    abi_handle.metadata.insert("service_name".to_string(), name.to_string());
                    runner.stop(&abi_handle).await.map_err(|e| anyhow!("{}", e))?;
                }
            }
            let _ = self.process_manager.runtime_db().clear_pid(name);

            runtime.state = ServiceState::Stopped;
            self.emit_service_event(name, ServiceEventType::Stopped);
            info!("Service {} stopped", name);
        } else {
            if let Some(service_config) = self.config.services.get(name) {
                let runner_type = service_config.runner.runner_type.as_str();
                if let Ok(runner) = self.runner_for(runner_type).await {
                    // Kitchen-sink handle: include both PID (from RuntimeDb) and
                    // container_name so each runner uses whichever field it needs.
                    let pid = self.process_manager.runtime_db().read_pid(name);
                    let abi_handle = lib_plugin_abi_v3::runner::ProcessHandle {
                        id: name.to_string(),
                        runner_type: runner_type.to_string(),
                        pid,
                        container_name: Some(docker_container_name(name, service_config)),
                        metadata: HashMap::from([("service_name".to_string(), name.to_string())]),
                    };
                    if let Err(e) = runner.stop(&abi_handle).await {
                        warn!("Failed to stop {} runner for {}: {}", runner_type, name, e);
                    }
                }
                let _ = self.process_manager.runtime_db().clear_pid(name);
            }
        }

        if let Some(service_config) = self.config.services.get(name) {
            let env = self
                .build_environment(name, service_config)
                .await
                .unwrap_or_default();
            if let Err(e) = self
                .run_hooks(HookEvent::PostDown, name, service_config, &env)
                .await
            {
                warn!("Post-down hooks failed for service {}: {}", name, e);
            }
        }

        Ok(())
    }

    pub async fn restart_service(&self, name: &str) -> Result<()> {
        self.stop_service(name).await?;
        self.start_service(name).await?;
        Ok(())
    }

    pub async fn get_status(&self, name: &str) -> Option<ServiceInfo> {
        let services = self.services.read().await;
        services.get(name).map(|r| r.to_info())
    }

    pub async fn get_all_status(&self) -> HashMap<String, ServiceInfo> {
        let services = self.services.read().await;
        services
            .iter()
            .map(|(k, v)| (k.clone(), v.to_info()))
            .collect()
    }

    /// Useful for status commands that create a fresh ServiceManager without in-memory state.
    pub async fn detect_running_services(&self) -> HashMap<String, ServiceInfo> {
        let mut result = HashMap::new();

        for (name, service_config) in &self.config.services {
            let runner_type = service_config.runner.runner_type.as_str();

            match runner_type {
                "script" => {
                    let mut ports = HashMap::new();
                    if let Some(rollout) = &service_config.rollout {
                        if let Ok(rollout_ports) = get_rollout_ports(rollout) {
                            ports = rollout_ports;
                        }
                    }

                    let pid_from_db = self.process_manager.is_service_running(name);

                    if let Some(pid) = pid_from_db {
                        // Managed by hive
                        let healthy = self
                            .check_health_for_status(service_config, &ports, None)
                            .await;

                        let state = if healthy == Some(false) {
                            ServiceState::Unhealthy
                        } else {
                            ServiceState::Running
                        };

                        result.insert(
                            name.clone(),
                            ServiceInfo {
                                name: name.clone(),
                                state,
                                pid: Some(pid),
                                container_id: None,
                                ports,
                                healthy,
                                restart_count: 0,
                                last_error: None,
                            },
                        );
                    } else {
                        // Check if port is in use by unmanaged process
                        let mut conflicting_port = None;
                        for (_port_name, port) in &ports {
                            if ProcessManager::is_port_in_use(*port) {
                                conflicting_port = Some(*port);
                                break;
                            }
                        }

                        if let Some(port) = conflicting_port {
                            // Port used by something we didn't start
                            result.insert(
                                name.clone(),
                                ServiceInfo {
                                    name: name.clone(),
                                    state: ServiceState::PortConflict,
                                    pid: None,
                                    container_id: None,
                                    ports,
                                    healthy: None,
                                    restart_count: 0,
                                    last_error: Some(format!(
                                        "Port {} is in use by another process (not managed by hive)",
                                        port
                                    )),
                                },
                            );
                        } else {
                            result.insert(
                                name.clone(),
                                ServiceInfo {
                                    name: name.clone(),
                                    state: ServiceState::Stopped,
                                    pid: None,
                                    container_id: None,
                                    ports: HashMap::new(),
                                    healthy: None,
                                    restart_count: 0,
                                    last_error: None,
                                },
                            );
                        }
                    }
                }
                _ => {
                    let container_name = docker_container_name(name, service_config);
                    let pid = self.process_manager.runtime_db().read_pid(name);
                    let handle = lib_plugin_abi_v3::runner::ProcessHandle {
                        id: name.clone(),
                        runner_type: runner_type.to_string(),
                        pid,
                        container_name: Some(container_name.clone()),
                        metadata: HashMap::from([("service_name".to_string(), name.clone())]),
                    };

                    let mut ports = HashMap::new();
                    if let Some(rollout) = &service_config.rollout {
                        if let Ok(rollout_ports) = get_rollout_ports(rollout) {
                            ports = rollout_ports;
                        }
                    }

                    if let Some(runner) = plugin_manager().get_runner(runner_type).await {
                        let running = runner.is_running(&handle).await;
                        let state = if running { ServiceState::Running } else { ServiceState::Stopped };

                        let healthy = if running {
                            self.check_health_for_status(service_config, &ports, Some(&container_name)).await
                        } else {
                            None
                        };

                        let state = if healthy == Some(false) { ServiceState::Unhealthy } else { state };

                        result.insert(
                            name.clone(),
                            ServiceInfo {
                                name: name.clone(),
                                state,
                                pid: None,
                                container_id: Some(container_name),
                                ports,
                                healthy,
                                restart_count: 0,
                                last_error: None,
                            },
                        );
                    } else {
                        result.insert(
                            name.clone(),
                            ServiceInfo {
                                name: name.clone(),
                                state: ServiceState::Stopped,
                                pid: None,
                                container_id: None,
                                ports: HashMap::new(),
                                healthy: None,
                                restart_count: 0,
                                last_error: None,
                            },
                        );
                    }
                }
            }
        }

        result
    }

    async fn check_health_for_status(
        &self,
        service_config: &ServiceConfig,
        ports: &HashMap<String, u16>,
        container_name: Option<&str>,
    ) -> Option<bool> {
        let healthcheck = service_config.healthcheck.as_ref()?;

        match self
            .health_checker
            .check_once_with_container(healthcheck, ports, container_name)
            .await
        {
            Ok(healthy) => Some(healthy),
            Err(e) => {
                debug!("Health check failed: {}", e);
                Some(false)
            }
        }
    }

    /// Checks system state (Docker/PID), not just in-memory state.
    async fn is_service_running_on_system(
        &self,
        name: &str,
        service_config: &ServiceConfig,
    ) -> bool {
        let runner_type = service_config.runner.runner_type.as_str();

        match runner_type {
            "script" => {
                // Port-in-use by unmanaged processes is NOT considered "running"
                self.process_manager.is_service_running(name).is_some()
            }
            _ => {
                let pid = self.process_manager.runtime_db().read_pid(name);
                let handle = lib_plugin_abi_v3::runner::ProcessHandle {
                    id: name.to_string(),
                    runner_type: runner_type.to_string(),
                    pid,
                    container_name: Some(docker_container_name(name, service_config)),
                    metadata: HashMap::from([("service_name".to_string(), name.to_string())]),
                };
                match plugin_manager().get_runner(runner_type).await {
                    Some(runner) => runner.is_running(&handle).await,
                    None => false,
                }
            }
        }
    }

    pub async fn get_logs(&self, name: &str, lines: Option<usize>) -> Result<Vec<String>> {
        let services = self.services.read().await;

        if let Some(runtime) = services.get(name) {
            if let Some(process) = &runtime.process {
                return self.process_manager.get_logs(process, lines).await;
            }
        }

        Ok(Vec::new())
    }

    async fn mark_service_crashed(&self, name: &str) {
        let mut services = self.services.write().await;
        if let Some(runtime) = services.get_mut(name) {
            runtime.state = ServiceState::Crashed;
        }
        self.emit_service_event(name, ServiceEventType::Crashed);
    }

    async fn wait_for_dependencies<F>(
        &self,
        name: &str,
        config: &ServiceConfig,
        mut on_progress: F,
    ) -> Result<()>
    where
        F: FnMut(ServicePhase),
    {
        for dep in &config.depends_on {
            info!("Waiting for dependency {} before starting {}", dep, name);
            on_progress(ServicePhase::WaitingFor(dep.clone()));

            let mut attempts = 0;
            loop {
                let services = self.services.read().await;
                if let Some(runtime) = services.get(dep) {
                    match runtime.state {
                        ServiceState::Running => {
                            // Just running state, not healthy — health checks are for proxy readiness
                            break;
                        }
                        ServiceState::Crashed | ServiceState::Exited => {
                            return Err(anyhow!(
                                "Dependency {} failed (state: {}), cannot start {}",
                                dep,
                                runtime.state,
                                name
                            ));
                        }
                        _ => {}
                    }
                }
                drop(services);

                attempts += 1;
                if attempts > 60 {
                    return Err(anyhow!("Timeout waiting for dependency: {}", dep));
                }

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }

        Ok(())
    }

    async fn build_environment(
        &self,
        _name: &str,
        config: &ServiceConfig,
    ) -> Result<HashMap<String, String>> {
        let mut env = HashMap::new();

        if let Some(global_env) = &self.config.environment {
            let global_resolved = self.env_resolver.resolve(global_env).await?;
            env.extend(global_resolved);
        }

        if let Some(service_env) = &config.environment {
            let service_resolved = self.env_resolver.resolve(service_env).await?;
            env.extend(service_resolved);
        }

        let mut runtime_ctx = RuntimeContext::new();
        if let Some(rollout) = &config.rollout {
            let ports = get_rollout_ports(rollout)?;
            runtime_ctx.set_ports(ports);
        }

        for value in env.values_mut() {
            *value = runtime_ctx.interpolate(value)?;
        }

        Ok(env)
    }

    async fn run_build(
        &self,
        name: &str,
        build: &crate::hive_config::BuildConfig,
        env: &HashMap<String, String>,
        shell: &str,
    ) -> Result<()> {
        use crate::hive_config::BuildTrigger;

        match build.build_when {
            BuildTrigger::Never => return Ok(()),
            BuildTrigger::Always => {}
            BuildTrigger::Missing => {
                if let Some(output) = &build.output {
                    let output_path = if std::path::Path::new(output).is_absolute() {
                        std::path::PathBuf::from(output)
                    } else {
                        // Relative to working_dir or project_root
                        build
                            .working_dir
                            .as_ref()
                            .map(|d| self.project_root.join(d))
                            .unwrap_or_else(|| self.project_root.clone())
                            .join(output)
                    };
                    if output_path.exists() {
                        debug!("Build output exists for service {}: {:?}", name, output_path);
                        return Ok(());
                    }
                }
            }
        }

        info!("Building service {}", name);

        let working_dir = build
            .working_dir
            .as_ref()
            .map(|d| self.project_root.join(d))
            .unwrap_or_else(|| self.project_root.clone());

        let output = self
            .process_manager
            .run_command(&build.command, &working_dir, env, shell)
            .await?;

        if !output.status.success() {
            return Err(anyhow!(
                "Build failed for service {}: {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        info!("Build completed for service {}", name);
        Ok(())
    }

    async fn start_process(
        &self,
        name: &str,
        config: &ServiceConfig,
        env: HashMap<String, String>,
    ) -> Result<ProcessHandle> {
        let runner = &config.runner;
        let runner_type = runner.runner_type.as_str();

        let mut runtime_ctx = RuntimeContext::new();
        let mut rollout_ports = HashMap::new();
        if let Some(rollout) = &config.rollout {
            // For blue-green, new instance uses inactive ports
            let ports = if rollout.rollout_type == ROLLOUT_TYPE_BLUE_GREEN {
                self.rollout_manager
                    .get_deployment_ports(name, rollout)
                    .await?
            } else {
                get_rollout_ports(rollout)?
            };
            rollout_ports = ports.clone();
            runtime_ctx.set_ports(ports);
        }

        let plugin_runner = self.runner_for(runner_type).await?;

        // Pre-resolve {{runtime.port.X}} references in config before passing to plugin
        let mut config_val = serde_json::to_value(&runner.config).unwrap_or_default();
        pre_interpolate_config(&mut config_val, &runtime_ctx);

        let mut abi_ctx = lib_plugin_abi_v3::runner::RuntimeContext::new(
            name,
            self.project_root.clone(),
        );
        for (port_name, port) in &rollout_ports {
            abi_ctx = abi_ctx.with_port(port_name.clone(), *port);
        }

        let handle = plugin_runner
            .start(name, &config_val, env, &abi_ctx)
            .await
            .map_err(|e| anyhow!("{}", e))?;

        Ok(ProcessHandle::from(handle))
    }

    async fn start_health_check(&self, name: &str) -> Result<()> {
        let config = self
            .config
            .services
            .get(name)
            .ok_or_else(|| anyhow!("Unknown service: {}", name))?;

        if let Some(healthcheck) = &config.healthcheck {
            let ports = {
                let services = self.services.read().await;
                services
                    .get(name)
                    .map(|r| r.ports.clone())
                    .unwrap_or_default()
            };

            let status = self
                .health_checker
                .start_health_checks(name, healthcheck, &ports);

            let mut services = self.services.write().await;
            if let Some(runtime) = services.get_mut(name) {
                runtime.health = Some(status);
            }
        }

        Ok(())
    }

    async fn handle_blue_green_deployment(&self, name: &str, config: &ServiceConfig) -> Result<()> {
        let Some(rollout) = &config.rollout else {
            return Err(anyhow!("Blue-green deployment requires rollout config"));
        };

        let state = self
            .rollout_manager
            .get_blue_green_state(name)
            .await
            .ok_or_else(|| anyhow!("No blue-green state for service: {}", name))?;

        let deployment = BlueGreenDeployment::new(name, state.clone());

        info!(
            "Blue-green deployment started for {} (timeout: {:?}, healthy_duration: {:?})",
            name,
            deployment.remaining_timeout(),
            deployment.healthy_duration()
        );

        let ports = deployment.new_instance_ports();

        let healthcheck = config.healthcheck.as_ref().unwrap();
        let health_check_result = self
            .wait_for_healthy(name, healthcheck, &ports, &deployment)
            .await;

        match health_check_result {
            Ok(()) => {
                info!(
                    "New instance of {} is healthy, waiting {:?} for stability",
                    name,
                    deployment.healthy_duration()
                );
                deployment.wait_healthy_duration().await;

                if self.check_health_once(name, healthcheck, &ports).await? {
                    self.rollout_manager.switch_blue_green(name).await?;

                    {
                        let mut services = self.services.write().await;
                        if let Some(runtime) = services.get_mut(name) {
                            runtime.ports = self.rollout_manager.get_ports(name, rollout).await?;
                        }
                    }

                    // Start ongoing health checking after successful switch
                    self.start_health_check(name).await?;

                    info!("Blue-green deployment completed for {}", name);
                    Ok(())
                } else {
                    self.handle_blue_green_failure(name).await
                }
            }
            Err(e) => {
                error!("Blue-green deployment failed for {}: {}", name, e);
                self.handle_blue_green_failure(name).await
            }
        }
    }

    async fn wait_for_healthy(
        &self,
        name: &str,
        healthcheck: &crate::hive_config::HealthCheckConfig,
        ports: &HashMap<String, u16>,
        deployment: &BlueGreenDeployment,
    ) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = deployment.remaining_timeout();

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow!("Health check timeout for {}", name));
            }

            if self.check_health_once(name, healthcheck, ports).await? {
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    async fn check_health_once(
        &self,
        _name: &str,
        healthcheck: &crate::hive_config::HealthCheckConfig,
        ports: &HashMap<String, u16>,
    ) -> Result<bool> {
        self.health_checker.check_once(healthcheck, ports).await
    }

    async fn handle_blue_green_failure(&self, name: &str) -> Result<()> {
        let action = self.rollout_manager.handle_failure(name).await?;

        match action {
            OnFailureAction::KeepOld => {
                // Old instance still running on active ports
                warn!(
                    "Blue-green deployment failed for {}, keeping old instance",
                    name
                );
                Ok(())
            }
            OnFailureAction::Abort => {
                // Stop both instances
                error!("Blue-green deployment aborted for {}", name);
                self.stop_service(name).await
            }
        }
    }

    pub async fn handle_service_exit(&self, name: &str, exit_code: i32) -> Result<()> {
        let config = self
            .config
            .services
            .get(name)
            .ok_or_else(|| anyhow!("Unknown service: {}", name))?;

        let should_restart = match config.restart {
            RestartPolicy::Never => false,
            RestartPolicy::OnFailure => exit_code != 0,
            RestartPolicy::Always => true,
            RestartPolicy::UnlessStopped => {
                let services = self.services.read().await;
                services
                    .get(name)
                    .map(|r| r.state != ServiceState::Stopping)
                    .unwrap_or(false)
            }
        };

        if should_restart {
            {
                let mut services = self.services.write().await;
                if let Some(runtime) = services.get_mut(name) {
                    runtime.restart_count += 1;
                    runtime.state = ServiceState::Crashed;
                    runtime.last_error = Some(format!("Exit code: {}", exit_code));
                }
            }
            self.emit_service_event(name, ServiceEventType::Crashed);

            // Exponential backoff
            let services = self.services.read().await;
            let restart_count = services.get(name).map(|r| r.restart_count).unwrap_or(0);
            drop(services);

            let delay = std::cmp::min(60, 1 << restart_count.min(6));
            warn!(
                "Service {} crashed with exit code {}. Restarting in {}s...",
                name, exit_code, delay
            );

            self.emit_service_event(name, ServiceEventType::Restarting);
            tokio::time::sleep(std::time::Duration::from_secs(delay as u64)).await;

            self.start_service(name).await?;
        } else {
            let mut services = self.services.write().await;
            if let Some(runtime) = services.get_mut(name) {
                runtime.state = if exit_code == 0 {
                    self.emit_service_event(name, ServiceEventType::Stopped);
                    ServiceState::Exited
                } else {
                    self.emit_service_event(name, ServiceEventType::Crashed);
                    ServiceState::Crashed
                };
            }
        }

        Ok(())
    }

    async fn run_hooks(
        &self,
        event: HookEvent,
        service_name: &str,
        service_config: &ServiceConfig,
        env: &HashMap<String, String>,
    ) -> Result<()> {
        // Global hooks first, then per-service hooks
        let global_steps = self
            .config
            .hooks
            .as_ref()
            .map(|h| h.steps_for(event).to_vec())
            .unwrap_or_default();

        let service_steps = service_config
            .hooks
            .as_ref()
            .map(|h| h.steps_for(event).to_vec())
            .unwrap_or_default();

        let all_steps: Vec<_> = global_steps.into_iter().chain(service_steps).collect();

        if all_steps.is_empty() {
            return Ok(());
        }

        info!(
            "Running {} hooks for service {} ({} steps)",
            event,
            service_name,
            all_steps.len()
        );

        let service_fqn = format!("{}:{}", self.source_name, service_name);
        let hook_ctx = HookContext {
            event,
            service_name: Some(service_name.to_string()),
            service_fqn: Some(service_fqn.clone()),
            source_name: self.source_name.clone(),
            rollout_type: service_config
                .rollout
                .as_ref()
                .map(|r| r.rollout_type.clone()),
            rollout_color: None,
        };

        // ABI runtime context (different from core RuntimeContext)
        let mut abi_ports = HashMap::new();
        if let Some(rollout) = &service_config.rollout {
            if let Ok(ports) = get_rollout_ports(rollout) {
                abi_ports = ports;
            }
        }
        let mut runtime_ctx =
            lib_plugin_abi_v3::runner::RuntimeContext::new(service_name, self.project_root.clone())
                .with_shell(resolve_service_shell(service_config));
        for (key, value) in env.iter() {
            runtime_ctx = runtime_ctx.with_env(key.clone(), value.clone());
        }
        for (name, port) in abi_ports {
            runtime_ctx = runtime_ctx.with_port(name, port);
        }

        let runners = plugin_manager().get_all_runners().await;
        let mut executor = HookExecutor::new(runners);

        // Wire hook output into event collector so it appears in `adi hive logs`
        if let Some(collector) = &self.event_collector {
            let collector = collector.clone();
            let fqn = service_fqn;
            let hook_event = event;
            executor = executor.with_output_handler(Arc::new(move |line, stream| {
                let (level, log_stream) = match stream {
                    HookOutputStream::Stdout => (LogLevel::Info, LogStream::Stdout),
                    HookOutputStream::Stderr => (LogLevel::Warn, LogStream::Stderr),
                };
                collector.emit(ObservabilityEvent::log(
                    &fqn,
                    level,
                    format!("[{} hook] {}", hook_event, line),
                    log_stream,
                ));
            }));
        }

        let result = executor
            .execute(event, &all_steps, env, &hook_ctx, &runtime_ctx)
            .await;

        if result.success {
            Ok(())
        } else {
            Err(anyhow!(
                "{} hooks failed for service {}: {}",
                event,
                service_name,
                result.error.unwrap_or_default()
            ))
        }
    }
}
