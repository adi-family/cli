//! Orchestration Plugin ABI
//!
//! Shared ABI definitions for orchestration plugins across different orchestrators (Hive, etc.).
//! This library provides stable trait definitions for common orchestration concerns.
//!
//! ## Plugin Categories
//!
//! | Category | Trait | Description |
//! |----------|-------|-------------|
//! | Runner | [`RunnerPlugin`] | Execute services (script, docker, etc.) |
//! | Env | [`EnvPlugin`] | Provide environment variables |
//! | Health | [`HealthPlugin`] | Check service readiness |
//! | Proxy | [`ProxyPlugin`] | Middleware for HTTP proxy |
//! | Obs | [`ObsPlugin`] | Observability (logging, metrics) |
//! | Rollout | [`RolloutPlugin`] | Deployment strategies |
//!
//! ## Lifecycle Hooks
//!
//! Runner plugins can also execute one-shot tasks for lifecycle hooks.
//! See the [`hooks`] module for hook types and the [`HookExecutor`].
//!
//! ## Plugin IDs
//!
//! Plugins are identified by their plugin ID following the pattern:
//! `<orchestrator>.<category>.<name>` (e.g., `hive.runner.docker`, `hive.obs.stdout`)
//!
//! ## Usage
//!
//! Orchestration systems (like Hive) can depend on this crate to define their plugin contracts.
//! Plugin authors implement these traits to create compatible plugins.

pub mod env;
pub mod health;
pub mod hooks;
pub mod loader;
pub mod obs;
pub mod proxy;
pub mod rollout;
pub mod runner;
pub mod types;

pub use env::EnvPlugin;
pub use health::{HealthPlugin, HealthResult};
pub use hooks::{
    HookContext, HookEvent, HookEventResult, HookExecutor, HookRunnerConfig, HookStep,
    HookStepResult, HooksConfig, OnFailure, StepType,
};
pub use loader::{plugin_loader, PluginInfo, PluginLoader, PluginStatus};
pub use obs::{ObsPlugin, ObservabilityEvent, LogLevel, HealthStatus, ServiceEventType, MetricValue};
pub use proxy::{ProxyPlugin, ProxyResult};
pub use rollout::{RolloutPlugin, RolloutStrategy};
pub use runner::{HookExitStatus, RunnerPlugin, ProcessHandle, ProcessStatus};
pub use types::*;

/// Plugin metadata returned by all plugins
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// Plugin ID (e.g., "hive.runner.docker")
    pub id: String,
    /// Plugin name (e.g., "docker")
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin category
    pub category: PluginCategory,
}

/// Plugin categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginCategory {
    Runner,
    Env,
    Health,
    Proxy,
    Obs,
    Rollout,
}

impl PluginCategory {
    /// Get the plugin ID prefix for this category
    pub fn prefix(&self) -> &'static str {
        match self {
            PluginCategory::Runner => "hive.runner.",
            PluginCategory::Env => "hive.env.",
            PluginCategory::Health => "hive.health.",
            PluginCategory::Proxy => "hive.proxy.",
            PluginCategory::Obs => "hive.obs.",
            PluginCategory::Rollout => "hive.rollout.",
        }
    }

    /// Parse category from plugin ID
    pub fn from_plugin_id(plugin_id: &str) -> Option<(Self, &str)> {
        if let Some(name) = plugin_id.strip_prefix("hive.runner.") {
            Some((PluginCategory::Runner, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.env.") {
            Some((PluginCategory::Env, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.health.") {
            Some((PluginCategory::Health, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.proxy.") {
            Some((PluginCategory::Proxy, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.obs.") {
            Some((PluginCategory::Obs, name))
        } else if let Some(name) = plugin_id.strip_prefix("hive.rollout.") {
            Some((PluginCategory::Rollout, name))
        } else {
            None
        }
    }
}

/// Resolve a short name to a full plugin ID
pub fn resolve_plugin_id(category: PluginCategory, name: &str) -> String {
    if name.starts_with("hive.") {
        name.to_string()
    } else {
        format!("{}{}", category.prefix(), name)
    }
}
