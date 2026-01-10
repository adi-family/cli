use crate::error::Result;
use crate::tool::ToolCategory;
use crate::types::ToolCall;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    Auto,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub pattern: String,
    pub level: PermissionLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl PermissionRule {
    pub fn new(pattern: impl Into<String>, level: PermissionLevel) -> Self {
        Self {
            pattern: pattern.into(),
            level,
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn matches(&self, tool_name: &str, arguments: &serde_json::Value) -> bool {
        let pattern_parts: Vec<&str> = self.pattern.split(':').collect();
        if pattern_parts.is_empty() {
            return false;
        }

        if pattern_parts[0] != "tool" {
            return false;
        }

        if pattern_parts.len() < 2 {
            return false;
        }

        let tool_pattern = pattern_parts[1];
        if !glob_match(tool_pattern, tool_name) {
            return false;
        }

        if pattern_parts.len() >= 3 {
            let arg_pattern = pattern_parts[2];
            let arg_str = serde_json::to_string(arguments).unwrap_or_default();
            if !glob_match(arg_pattern, &arg_str) {
                return false;
            }
        }

        true
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0], parts[1]);
            return text.starts_with(prefix) && text.ends_with(suffix);
        }
        glob::Pattern::new(pattern)
            .map(|p| p.matches(text))
            .unwrap_or(false)
    } else {
        pattern == text
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Allow,
    AllowAll,
    Deny,
    Abort,
}

#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    async fn request_approval(
        &self,
        tool_call: &ToolCall,
        rule: Option<&PermissionRule>,
    ) -> Result<ApprovalDecision>;
}

pub struct AutoApprover;

#[async_trait]
impl ApprovalHandler for AutoApprover {
    async fn request_approval(
        &self,
        _tool_call: &ToolCall,
        _rule: Option<&PermissionRule>,
    ) -> Result<ApprovalDecision> {
        Ok(ApprovalDecision::Allow)
    }
}

#[derive(Default)]
pub struct PermissionManager {
    rules: Vec<PermissionRule>,
    session_overrides: HashMap<String, PermissionLevel>,
    category_defaults: HashMap<ToolCategory, PermissionLevel>,
}

impl PermissionManager {
    pub fn new() -> Self {
        let mut category_defaults = HashMap::new();
        category_defaults.insert(ToolCategory::ReadOnly, PermissionLevel::Auto);
        category_defaults.insert(ToolCategory::Write, PermissionLevel::Ask);
        category_defaults.insert(ToolCategory::Execute, PermissionLevel::Ask);
        category_defaults.insert(ToolCategory::External, PermissionLevel::Ask);

        Self {
            rules: Vec::new(),
            session_overrides: HashMap::new(),
            category_defaults,
        }
    }

    pub fn add_rule(&mut self, rule: PermissionRule) {
        self.rules.push(rule);
    }

    pub fn add_rules(&mut self, rules: Vec<PermissionRule>) {
        self.rules.extend(rules);
    }

    pub fn set_session_override(&mut self, pattern: String, level: PermissionLevel) {
        self.session_overrides.insert(pattern, level);
    }

    pub fn clear_session_overrides(&mut self) {
        self.session_overrides.clear();
    }

    pub fn check(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        category: Option<ToolCategory>,
    ) -> (PermissionLevel, Option<&PermissionRule>) {
        for (pattern, level) in &self.session_overrides {
            let temp_rule = PermissionRule::new(pattern.clone(), *level);
            if temp_rule.matches(tool_name, arguments) {
                return (*level, None);
            }
        }

        for rule in &self.rules {
            if rule.matches(tool_name, arguments) {
                return (rule.level, Some(rule));
            }
        }

        if let Some(cat) = category {
            if let Some(level) = self.category_defaults.get(&cat) {
                return (*level, None);
            }
        }

        (PermissionLevel::Ask, None)
    }

    pub fn with_defaults() -> Self {
        let mut manager = Self::new();
        manager.add_rules(vec![
            PermissionRule::new("tool:read_file:*", PermissionLevel::Auto),
            PermissionRule::new("tool:glob:*", PermissionLevel::Auto),
            PermissionRule::new("tool:grep:*", PermissionLevel::Auto),
            PermissionRule::new("tool:bash:git status*", PermissionLevel::Auto),
            PermissionRule::new("tool:bash:git diff*", PermissionLevel::Auto),
            PermissionRule::new("tool:bash:git log*", PermissionLevel::Auto),
            PermissionRule::new("tool:bash:ls*", PermissionLevel::Auto),
            PermissionRule::new("tool:bash:cat*", PermissionLevel::Auto),
            PermissionRule::new("tool:bash:rm*", PermissionLevel::Deny)
                .with_reason("Destructive operation"),
            PermissionRule::new("tool:*:*.env", PermissionLevel::Deny).with_reason("Secrets file"),
            PermissionRule::new("tool:*:*credentials*", PermissionLevel::Deny)
                .with_reason("Credentials file"),
        ]);
        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_rule_matches() {
        let rule = PermissionRule::new("tool:read_file:*", PermissionLevel::Auto);
        assert!(rule.matches("read_file", &serde_json::json!({"path": "/test.txt"})));
        assert!(!rule.matches("write_file", &serde_json::json!({})));
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("git*", "git status"));
        assert!(glob_match("*.txt", "file.txt"));
        assert!(!glob_match("git*", "other command"));
    }

    #[test]
    fn test_permission_manager_check() {
        let manager = PermissionManager::with_defaults();

        let (level, _) = manager.check("read_file", &serde_json::json!({}), None);
        assert_eq!(level, PermissionLevel::Auto);

        // Category default for Execute is Ask
        let (level, _) = manager.check(
            "some_bash_command",
            &serde_json::json!({"command": "rm -rf /"}),
            Some(ToolCategory::Execute),
        );
        assert_eq!(level, PermissionLevel::Ask);
    }

    #[test]
    fn test_session_override() {
        let mut manager = PermissionManager::new();
        manager.set_session_override("tool:write_file:*".to_string(), PermissionLevel::Auto);

        let (level, _) = manager.check("write_file", &serde_json::json!({}), None);
        assert_eq!(level, PermissionLevel::Auto);
    }

    #[test]
    fn test_category_default() {
        let manager = PermissionManager::new();

        let (level, _) = manager.check(
            "unknown_tool",
            &serde_json::json!({}),
            Some(ToolCategory::ReadOnly),
        );
        assert_eq!(level, PermissionLevel::Auto);

        let (level, _) = manager.check(
            "unknown_tool",
            &serde_json::json!({}),
            Some(ToolCategory::Write),
        );
        assert_eq!(level, PermissionLevel::Ask);
    }
}
