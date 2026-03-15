//! Multi-Source Configuration Manager
//!
//! Manages multiple configuration sources (YAML files or SQLite databases)
//! with unified service management across all sources.

use crate::global_registry::GlobalRegistry;
use crate::hive_config::{validate_config, HiveConfig, HiveConfigParser, ServiceConfig, ServiceInfo, ServiceState, SourceType};
use crate::exposure::ExposureManager;
use crate::observability::EventCollector;
use crate::service_manager::ServiceManager;
use crate::service_proxy::ServiceProxyState;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

const DEFAULT_SOURCE_DIR: &str = ".adi/hive";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceInfo {
    /// Source name (e.g., "default", "my-project")
    pub name: String,
    pub path: PathBuf,
    pub source_type: SourceType,
    pub enabled: bool,
    pub service_count: usize,
    pub status: SourceStatus,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceStatus {
    Loaded,
    Running,
    Stopped,
    Error(String),
}

pub struct SourceManager {
    sources: Arc<RwLock<HashMap<String, ManagedSource>>>,
    /// Exposure manager for cross-source dependencies
    exposure_manager: Arc<ExposureManager>,
    /// Service proxy state (shared across all sources)
    proxy_state: Arc<ServiceProxyState>,
    registry: GlobalRegistry,
    event_collector: Arc<EventCollector>,
}

struct ManagedSource {
    info: SourceInfo,
    /// Parsed configuration (for YAML sources)
    config: Option<HiveConfig>,
    service_manager: Option<ServiceManager>,
}

impl SourceManager {
    fn open_registry() -> GlobalRegistry {
        GlobalRegistry::open().expect("Failed to open global registry")
    }

    pub fn new(event_collector: Arc<EventCollector>) -> Self {
        Self {
            sources: Arc::new(RwLock::new(HashMap::new())),
            exposure_manager: Arc::new(ExposureManager::new()),
            proxy_state: Arc::new(ServiceProxyState::new()),
            registry: Self::open_registry(),
            event_collector,
        }
    }

    pub fn default_source_dir() -> PathBuf {
        default_source_dir()
    }

    pub async fn init(&self) -> Result<()> {
        let default_dir = default_source_dir();
        if default_dir.exists() {
            if detect_source_type(&default_dir).is_some() {
                if let Err(e) = self.add_source(&default_dir, Some("default")).await {
                    warn!("Could not load default source from {}: {}", default_dir.display(), e);
                }
            }
        }

        self.load_saved_sources().await?;

        Ok(())
    }

    /// Add an in-memory source with no backing file.
    ///
    /// Used for ephemeral sources like dynamically-created cocoon services.
    pub async fn add_virtual_source(&self, name: &str) -> Result<()> {
        let sources = self.sources.read().await;
        if sources.contains_key(name) {
            return Ok(());
        }
        drop(sources);

        let config = HiveConfig::default();
        let info = SourceInfo {
            name: name.to_string(),
            path: PathBuf::from(format!("<virtual:{name}>")),
            source_type: SourceType::Yaml,
            enabled: true,
            service_count: 0,
            status: SourceStatus::Loaded,
        };

        let managed = ManagedSource {
            info,
            config: Some(config),
            service_manager: None,
        };

        let mut sources = self.sources.write().await;
        sources.insert(name.to_string(), managed);

        info!("Added virtual source '{}'", name);
        Ok(())
    }

    /// Add a new source (idempotent — reloads if path already registered)
    pub async fn add_source(&self, path: &Path, name: Option<&str>) -> Result<String> {
        let path = path.canonicalize()
            .with_context(|| format!("Path does not exist: {}", path.display()))?;

        if let Some(existing_name) = self.find_source_by_path(&path).await {
            self.reload_source(&existing_name).await?;
            return Ok(existing_name);
        }

        let source_type = detect_source_type(&path)
            .ok_or_else(|| anyhow!(
                "Could not detect source type at {}. Expected hive.yaml or hive.db",
                path.display()
            ))?;

        let name = name.map(|s| s.to_string()).unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

        {
            let sources = self.sources.read().await;
            if sources.contains_key(&name) {
                return Err(anyhow!("Source '{}' already exists", name));
            }
        }

        let (config, service_count) = match source_type {
            SourceType::Yaml => {
                let parser = HiveConfigParser::new(&path);
                let config = parser.parse()
                    .with_context(|| format!("Failed to parse config from {}", path.display()))?;

                let validation = validate_config(&config);
                if !validation.is_valid() {
                    let errors: Vec<String> = validation.errors.iter()
                        .map(|e| format!("  - {}", e))
                        .collect();
                    return Err(anyhow!("Configuration errors:\n{}", errors.join("\n")));
                }

                let count = config.services.len();
                (Some(config), count)
            }
            SourceType::Sqlite => {
                return Err(anyhow!("SQLite source type is not yet implemented"));
            }
        };

        if let Some(ref cfg) = config {
            self.check_conflicts(cfg, &name).await?;
        }

        // Clone source_type before it moves into SourceInfo
        let source_type_for_registry = source_type.clone();

        let info = SourceInfo {
            name: name.clone(),
            path: path.clone(),
            source_type,
            enabled: true,
            service_count,
            status: SourceStatus::Loaded,
        };

        let managed = ManagedSource {
            info,
            config,
            service_manager: None,
        };

        {
            let mut sources = self.sources.write().await;
            sources.insert(name.clone(), managed);
        }

        self.persist_source(&name, &path, source_type_for_registry, true);

        info!("Added source '{}' from {}", name, path.display());
        Ok(name)
    }

    pub async fn remove_source(&self, name: &str) -> Result<()> {
        self.stop_source(name).await?;

        let mut sources = self.sources.write().await;
        sources.remove(name)
            .ok_or_else(|| anyhow!("Unknown source: {}", name))?;

        drop(sources);

        self.registry.remove_source(name).ok();

        info!("Removed source '{}'", name);
        Ok(())
    }

    pub async fn enable_source(&self, name: &str) -> Result<()> {
        let mut sources = self.sources.write().await;
        let source = sources.get_mut(name)
            .ok_or_else(|| anyhow!("Unknown source: {}", name))?;
        source.info.enabled = true;
        drop(sources);

        self.registry.set_enabled(name, true).ok();
        info!("Enabled source '{}'", name);
        Ok(())
    }

    pub async fn disable_source(&self, name: &str) -> Result<()> {
        self.stop_source(name).await?;

        let mut sources = self.sources.write().await;
        let source = sources.get_mut(name)
            .ok_or_else(|| anyhow!("Unknown source: {}", name))?;
        source.info.enabled = false;
        drop(sources);

        self.registry.set_enabled(name, false).ok();
        info!("Disabled source '{}'", name);
        Ok(())
    }

    pub async fn reload_source(&self, name: &str) -> Result<()> {
        let mut sources = self.sources.write().await;
        let source = sources.get_mut(name)
            .ok_or_else(|| anyhow!("Unknown source: {}", name))?;

        match source.info.source_type {
            SourceType::Yaml => {
                let parser = HiveConfigParser::new(&source.info.path);
                let config = parser.parse()
                    .with_context(|| format!("Failed to parse config from {}", source.info.path.display()))?;

                let validation = validate_config(&config);
                if !validation.is_valid() {
                    let errors: Vec<String> = validation.errors.iter()
                        .map(|e| format!("  - {}", e))
                        .collect();
                    return Err(anyhow!("Configuration errors:\n{}", errors.join("\n")));
                }

                source.info.service_count = config.services.len();
                if let Some(manager) = &mut source.service_manager {
                    manager.update_config(config.clone());
                }
                // Update proxy routes so new/changed services are immediately routable
                self.proxy_state.load_source_config(name, &config);
                source.config = Some(config);
            }
            SourceType::Sqlite => {
                return Err(anyhow!("SQLite source type is not yet implemented"));
            }
        }

        info!("Reloaded source '{}'", name);
        Ok(())
    }

    pub async fn start_source(&self, name: &str) -> Result<()> {
        // Phase 1: Setup (short write lock)
        let manager = {
            let mut sources = self.sources.write().await;
            let source = sources.get_mut(name)
                .ok_or_else(|| anyhow!("Unknown source: {}", name))?;

            if !source.info.enabled {
                return Err(anyhow!("Source '{}' is disabled", name));
            }

            self.ensure_service_manager(source, name)?
        }; // WRITE LOCK RELEASED — concurrent reads (list_services polling) now work

        // Phase 2: Start services (no lock held)
        manager.start_all().await?;

        // Phase 3: Update source status (short write lock)
        {
            let mut sources = self.sources.write().await;
            if let Some(source) = sources.get_mut(name) {
                source.info.status = SourceStatus::Running;
            }
        }

        info!("Started source '{}'", name);
        Ok(())
    }

    pub async fn stop_source(&self, name: &str) -> Result<()> {
        let mut sources = self.sources.write().await;
        let source = sources.get_mut(name)
            .ok_or_else(|| anyhow!("Unknown source: {}", name))?;

        if let Some(manager) = &source.service_manager {
            manager.stop_all().await?;
        }

        source.info.status = SourceStatus::Stopped;
        info!("Stopped source '{}'", name);

        Ok(())
    }

    /// Start a specific service (using FQN: source:service)
    pub async fn start_service(&self, fqn: &str) -> Result<()> {
        let (source_name, service_name) = parse_fqn(fqn)?;

        // Phase 1: Setup (short write lock)
        let manager = {
            let mut sources = self.sources.write().await;
            let source = sources.get_mut(&source_name)
                .ok_or_else(|| anyhow!("Unknown source: {}", source_name))?;

            self.ensure_service_manager(source, &source_name)?
        }; // WRITE LOCK RELEASED

        // Phase 2: Start service (no lock held)
        manager.start_service(&service_name).await?;

        // Phase 3: Update source status (short write lock)
        {
            let mut sources = self.sources.write().await;
            if let Some(source) = sources.get_mut(&source_name) {
                source.info.status = SourceStatus::Running;
            }
        }

        info!("Started service {}:{}", source_name, service_name);
        Ok(())
    }

    pub async fn stop_service(&self, fqn: &str) -> Result<()> {
        let (source_name, service_name) = parse_fqn(fqn)?;

        let sources = self.sources.read().await;
        let source = sources.get(&source_name)
            .ok_or_else(|| anyhow!("Unknown source: {}", source_name))?;

        if let Some(manager) = &source.service_manager {
            manager.stop_service(&service_name).await?;
        }

        info!("Stopped service {}:{}", source_name, service_name);
        Ok(())
    }

    /// Restart a specific service (using FQN: source:service).
    /// Reloads source config from disk first so proxy routes pick up any hive.yaml changes.
    pub async fn restart_service(&self, fqn: &str) -> Result<()> {
        let (source_name, _) = parse_fqn(fqn)?;

        // Re-read hive.yaml and refresh proxy routes before touching the service.
        self.reload_source(&source_name).await?;

        if let Err(e) = self.stop_service(fqn).await {
            debug!("Stop during restart failed (ignored): {}", e);
        }

        self.start_service(fqn).await
    }

    pub async fn list_sources(&self) -> Vec<SourceInfo> {
        let sources = self.sources.read().await;
        sources.values().map(|s| s.info.clone()).collect()
    }

    /// List all services across sources, optionally filtered by source name.
    pub async fn list_services(&self, source_filter: Option<&str>) -> Vec<(String, ServiceInfo)> {
        let sources = self.sources.read().await;
        let mut result = Vec::new();

        for (source_name, managed) in sources.iter() {
            if let Some(filter) = source_filter {
                if source_name != filter {
                    continue;
                }
            }

            if let Some(manager) = &managed.service_manager {
                // ServiceManager exists — get live per-service state
                for (_, info) in manager.get_all_status().await {
                    result.push((source_name.clone(), info));
                }
            } else if let Some(config) = &managed.config {
                // No ServiceManager yet — report all services as Stopped
                for name in config.services.keys() {
                    result.push((source_name.clone(), ServiceInfo {
                        name: name.clone(),
                        state: ServiceState::Stopped,
                        pid: None,
                        container_id: None,
                        ports: HashMap::new(),
                        healthy: None,
                        last_error: None,
                        restart_count: 0,
                    }));
                }
            }
        }

        result
    }

    /// Find a source by its canonicalized path
    fn ensure_service_manager(&self, source: &mut ManagedSource, name: &str) -> Result<ServiceManager> {
        if source.service_manager.is_none() {
            let config = source.config.as_ref()
                .ok_or_else(|| anyhow!("Source '{}' has no configuration", name))?;
            self.proxy_state.load_source_config(name, config);
            // Clone the manager — all fields are Arc, so the clone shares state
            // with the original. Service status updates during start_all() are
            // visible to concurrent readers via get_all_status().
            source.service_manager = Some(ServiceManager::with_observability(
                &source.info.path,
                config.clone(),
                self.proxy_state.clone(),
                self.event_collector.clone(),
                name.to_string(),
            )?);
        }
        Ok(source.service_manager.as_ref().unwrap().clone())
    }

    async fn find_source_by_path(&self, canonical_path: &Path) -> Option<String> {
        let sources = self.sources.read().await;
        sources.values()
            .find(|s| s.info.path == canonical_path)
            .map(|s| s.info.name.clone())
    }

    pub async fn get_source(&self, name: &str) -> Option<SourceInfo> {
        let sources = self.sources.read().await;
        sources.get(name).map(|s| s.info.clone())
    }

    /// Get a single service by FQN (source:service)
    pub async fn get_service(&self, fqn: &str) -> Result<Option<(String, ServiceInfo)>> {
        let (source_name, service_name) = parse_fqn(fqn)?;
        let sources = self.sources.read().await;

        if let Some(managed) = sources.get(&source_name) {
            if let Some(manager) = &managed.service_manager {
                // ServiceManager exists — get live state
                if let Some(info) = manager.get_status(&service_name).await {
                    return Ok(Some((source_name, info)));
                }
            } else if let Some(config) = &managed.config {
                // No ServiceManager yet — report as Stopped if service exists in config
                if config.services.contains_key(&service_name) {
                    return Ok(Some((source_name, ServiceInfo {
                        name: service_name,
                        state: ServiceState::Stopped,
                        pid: None,
                        container_id: None,
                        ports: HashMap::new(),
                        healthy: None,
                        last_error: None,
                        restart_count: 0,
                    })));
                }
            }
        }

        Ok(None)
    }

    /// Create a new service dynamically in an existing source.
    ///
    /// Adds the service config to the source's in-memory config and updates
    /// the service manager so the service can be started immediately.
    pub async fn create_service(
        &self,
        source_id: &str,
        name: &str,
        config: ServiceConfig,
    ) -> Result<()> {
        let mut sources = self.sources.write().await;
        let source = sources.get_mut(source_id)
            .ok_or_else(|| anyhow!("Unknown source: {}", source_id))?;

        let hive_config = source.config.as_mut()
            .ok_or_else(|| anyhow!("Source '{}' has no configuration", source_id))?;

        if hive_config.services.contains_key(name) {
            return Err(anyhow!("Service '{}' already exists in source '{}'", name, source_id));
        }

        hive_config.services.insert(name.to_string(), config);
        source.info.service_count = hive_config.services.len();

        if let Some(manager) = &mut source.service_manager {
            manager.update_config(hive_config.clone());
        }

        info!("Created service {}:{}", source_id, name);
        Ok(())
    }

    /// Delete a service from an existing source.
    ///
    /// Stops the service if running, then removes it from the source config.
    pub async fn delete_service(&self, fqn: &str) -> Result<()> {
        let (source_name, service_name) = parse_fqn(fqn)?;

        // Stop the service first (ignore errors if already stopped)
        if let Err(e) = self.stop_service(fqn).await {
            debug!("Stop during delete (ignored): {}", e);
        }

        let mut sources = self.sources.write().await;
        let source = sources.get_mut(&source_name)
            .ok_or_else(|| anyhow!("Unknown source: {}", source_name))?;

        let hive_config = source.config.as_mut()
            .ok_or_else(|| anyhow!("Source '{}' has no configuration", source_name))?;

        if hive_config.services.remove(&service_name).is_none() {
            return Err(anyhow!("Service '{}' not found in source '{}'", service_name, source_name));
        }

        source.info.service_count = hive_config.services.len();

        if let Some(manager) = &mut source.service_manager {
            manager.update_config(hive_config.clone());
        }

        info!("Deleted service {}:{}", source_name, service_name);
        Ok(())
    }

    pub fn exposure_manager(&self) -> &Arc<ExposureManager> {
        &self.exposure_manager
    }

    pub fn proxy_state(&self) -> &Arc<ServiceProxyState> {
        &self.proxy_state
    }

    async fn check_conflicts(&self, config: &HiveConfig, new_source: &str) -> Result<()> {
        let sources = self.sources.read().await;

        for (source_name, managed) in sources.iter() {
            if let Some(existing_config) = &managed.config {
                Self::check_port_conflicts(config, new_source, existing_config, source_name)?;
                Self::check_route_conflicts(config, new_source, existing_config, source_name)?;
                Self::check_expose_conflicts(config, new_source, existing_config, source_name)?;
            }
        }

        Ok(())
    }

    fn check_port_conflicts(
        new_config: &HiveConfig,
        new_source: &str,
        existing_config: &HiveConfig,
        existing_source: &str,
    ) -> Result<()> {
        for (new_name, new_svc) in &new_config.services {
            let new_ports = new_svc.rollout.as_ref()
                .and_then(|r| crate::hive_config::get_rollout_ports(r).ok())
                .unwrap_or_default();

            if new_ports.is_empty() {
                continue;
            }

            for (existing_name, existing_svc) in &existing_config.services {
                let existing_ports = existing_svc.rollout.as_ref()
                    .and_then(|r| crate::hive_config::get_rollout_ports(r).ok())
                    .unwrap_or_default();

                for (port_name, port) in &new_ports {
                    if let Some((ex_port_name, _)) = existing_ports.iter().find(|(_, p)| *p == port) {
                        return Err(anyhow!(
                            "Port {} conflicts: {}:{}.{} vs {}:{}.{}",
                            port,
                            new_source, new_name, port_name,
                            existing_source, existing_name, ex_port_name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn check_route_conflicts(
        new_config: &HiveConfig,
        new_source: &str,
        existing_config: &HiveConfig,
        existing_source: &str,
    ) -> Result<()> {
        for (new_name, new_svc) in &new_config.services {
            let Some(new_proxy) = &new_svc.proxy else { continue };

            for new_ep in new_proxy.endpoints() {
                for (existing_name, existing_svc) in &existing_config.services {
                    let Some(existing_proxy) = &existing_svc.proxy else { continue };

                    if let Some(conflict) = existing_proxy.endpoints().iter()
                        .find(|ex_ep| ex_ep.host == new_ep.host && ex_ep.path == new_ep.path)
                    {
                        return Err(anyhow!(
                            "Route {:?}/{} conflicts: {}:{} vs {}:{}",
                            conflict.host, conflict.path,
                            new_source, new_name,
                            existing_source, existing_name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn check_expose_conflicts(
        new_config: &HiveConfig,
        new_source: &str,
        existing_config: &HiveConfig,
        existing_source: &str,
    ) -> Result<()> {
        for (new_name, new_svc) in &new_config.services {
            let Some(new_expose) = &new_svc.expose else { continue };

            for (existing_name, existing_svc) in &existing_config.services {
                if let Some(existing_expose) = &existing_svc.expose {
                    if new_expose.name == existing_expose.name {
                        return Err(anyhow!(
                            "Expose name '{}' conflicts: {}:{} vs {}:{}",
                            new_expose.name,
                            new_source, new_name,
                            existing_source, existing_name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    async fn load_saved_sources(&self) -> Result<()> {
        let saved = self.registry.list_enabled_sources()
            .unwrap_or_default();

        for source in saved {
            if let Err(e) = self.add_source(&source.path, Some(&source.name)).await {
                warn!("Failed to load saved source '{}': {}", source.name, e);
            }
        }

        Ok(())
    }

    fn persist_source(&self, name: &str, path: &Path, source_type: SourceType, enabled: bool) {
        if name == "default" {
            return; // Don't persist the default source
        }
        if let Err(e) = self.registry.add_source(name, path, source_type, enabled) {
            warn!("Failed to persist source '{}' to registry: {}", name, e);
        }
    }
}

fn default_source_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(DEFAULT_SOURCE_DIR))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOURCE_DIR))
}

/// Read the sources registry without requiring the daemon.
pub fn read_sources_registry() -> HashMap<String, PathBuf> {
    crate::global_registry::read_sources_registry()
}

fn detect_source_type(path: &Path) -> Option<SourceType> {
    if path.extension().map(|e| e == "db").unwrap_or(false) {
        return Some(SourceType::Sqlite);
    }
    
    if path.is_dir() {
        if path.join("hive.db").exists() {
            return Some(SourceType::Sqlite);
        }
        if path.join(".adi/hive.yaml").exists() || path.join("hive.yaml").exists() {
            return Some(SourceType::Yaml);
        }
    }
    
    None
}

/// Parse a Fully Qualified Name (source:service) into (source, service)
fn parse_fqn(fqn: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = fqn.splitn(2, ':').collect();
    
    if parts.len() == 2 {
        Ok((parts[0].to_string(), parts[1].to_string()))
    } else if parts.len() == 1 {
        Err(anyhow!(
            "Service name '{}' must include source (e.g., 'source:service'). \
             Use 'adi hive status' to see available sources.",
            fqn
        ))
    } else {
        Err(anyhow!("Invalid FQN format: {}", fqn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fqn() {
        let (source, service) = parse_fqn("default:postgres").unwrap();
        assert_eq!(source, "default");
        assert_eq!(service, "postgres");

        let (source, service) = parse_fqn("my-project:auth").unwrap();
        assert_eq!(source, "my-project");
        assert_eq!(service, "auth");

        assert!(parse_fqn("postgres").is_err());
    }

    #[test]
    fn test_detect_source_type() {
        let path = PathBuf::from("/some/path/hive.db");
        assert_eq!(detect_source_type(&path), Some(SourceType::Sqlite));
    }
}
