//! Tool configuration management

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

use crate::error::Result;
use crate::permission::PermissionLevel;
use crate::quota::QuotaConfig;

/// Configuration for a single tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Permission level override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionLevel>,

    /// Whether the tool is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Quota configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota: Option<QuotaConfig>,

    /// Tool-specific configuration
    #[serde(default)]
    pub config: Value,

    /// Optional notes/reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

fn default_enabled() -> bool {
    true
}

impl ToolConfig {
    pub fn new() -> Self {
        Self {
            permission: None,
            enabled: true,
            quota: None,
            config: Value::Null,
            note: None,
        }
    }

    pub fn with_permission(mut self, permission: PermissionLevel) -> Self {
        self.permission = Some(permission);
        self
    }

    pub fn with_quota(mut self, quota: QuotaConfig) -> Self {
        self.quota = Some(quota);
        self
    }

    pub fn with_config(mut self, config: Value) -> Self {
        self.config = config;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Collection of tool configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolConfigSet {
    #[serde(flatten)]
    pub tools: HashMap<String, ToolConfig>,
}

impl ToolConfigSet {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: impl Into<String>, config: ToolConfig) {
        self.tools.insert(name.into(), config);
    }

    pub fn get(&self, tool_name: &str) -> Option<&ToolConfig> {
        self.tools.get(tool_name)
    }

    pub fn get_mut(&mut self, tool_name: &str) -> Option<&mut ToolConfig> {
        self.tools.get_mut(tool_name)
    }

    /// Load from TOML file
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let parsed: HashMap<String, HashMap<String, ToolConfig>> = toml::from_str(&content)?;

        let mut config_set = Self::new();
        if let Some(tools) = parsed.get("tools") {
            for (name, config) in tools {
                config_set.add(name, config.clone());
            }
        }

        Ok(config_set)
    }

    /// Save to TOML file
    pub fn to_toml_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut output = HashMap::new();
        output.insert("tools", &self.tools);

        let content = toml::to_string_pretty(&output)
            .map_err(|e| crate::error::AgentError::TomlSerError(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quota::QuotaPeriod;

    #[test]
    fn test_tool_config_builder() {
        let config = ToolConfig::new()
            .with_permission(PermissionLevel::Auto)
            .with_quota(QuotaConfig::per_session(5))
            .with_note("Test tool");

        assert_eq!(config.permission, Some(PermissionLevel::Auto));
        assert!(config.quota.is_some());
        assert_eq!(config.note, Some("Test tool".to_string()));
        assert!(config.enabled);
    }

    #[test]
    fn test_tool_config_disabled() {
        let config = ToolConfig::new().disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_tool_config_set() {
        let mut set = ToolConfigSet::new();
        set.add(
            "tool1",
            ToolConfig::new().with_permission(PermissionLevel::Auto),
        );
        set.add("tool2", ToolConfig::new().disabled());

        assert!(set.get("tool1").is_some());
        assert!(set.get("tool2").is_some());
        assert_eq!(
            set.get("tool1").unwrap().permission,
            Some(PermissionLevel::Auto)
        );
        assert!(!set.get("tool2").unwrap().enabled);
    }

    #[test]
    fn test_toml_serialization() {
        let mut set = ToolConfigSet::new();
        set.add(
            "test_tool",
            ToolConfig::new()
                .with_permission(PermissionLevel::Ask)
                .with_quota(QuotaConfig::per_session(10)),
        );

        let toml = toml::to_string_pretty(&set.tools).unwrap();
        assert!(toml.contains("test_tool"));
    }
}
