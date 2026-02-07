//! Quota management system for limiting tool operations

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::permission::PermissionLevel;

/// Time period for quota resets
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum QuotaPeriod {
    /// Per session (resets when session ends)
    Session,
    /// Per minute (rolling window)
    Minute,
    /// Per hour (rolling window)
    Hour,
    /// Per day (rolling window)
    Day,
}

impl QuotaPeriod {
    /// Get duration for this period
    pub fn duration(&self) -> Option<Duration> {
        match self {
            Self::Session => None, // No time limit
            Self::Minute => Some(Duration::minutes(1)),
            Self::Hour => Some(Duration::hours(1)),
            Self::Day => Some(Duration::days(1)),
        }
    }
}

/// Quota configuration for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaConfig {
    /// Maximum number of operations allowed
    pub max_operations: usize,

    /// Time period for quota reset
    pub period: QuotaPeriod,

    /// Permission level to escalate to when quota exceeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalate_to: Option<PermissionLevel>,

    /// Custom message when quota exceeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl QuotaConfig {
    pub fn new(max_operations: usize, period: QuotaPeriod) -> Self {
        Self {
            max_operations,
            period,
            escalate_to: Some(PermissionLevel::Ask),
            message: None,
        }
    }

    pub fn with_escalation(mut self, level: PermissionLevel) -> Self {
        self.escalate_to = Some(level);
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn per_session(max_operations: usize) -> Self {
        Self::new(max_operations, QuotaPeriod::Session)
    }

    pub fn per_minute(max_operations: usize) -> Self {
        Self::new(max_operations, QuotaPeriod::Minute)
    }

    pub fn per_hour(max_operations: usize) -> Self {
        Self::new(max_operations, QuotaPeriod::Hour)
    }

    pub fn per_day(max_operations: usize) -> Self {
        Self::new(max_operations, QuotaPeriod::Day)
    }
}

/// Tracks quota usage for a single tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaUsage {
    /// Number of operations performed
    pub count: usize,

    /// Timestamps of operations (for time-based quotas)
    pub timestamps: Vec<DateTime<Utc>>,

    /// When this quota tracking started
    pub started_at: DateTime<Utc>,
}

impl Default for QuotaUsage {
    fn default() -> Self {
        Self::new()
    }
}

impl QuotaUsage {
    pub fn new() -> Self {
        Self {
            count: 0,
            timestamps: Vec::new(),
            started_at: Utc::now(),
        }
    }

    /// Record an operation
    pub fn record(&mut self) {
        let now = Utc::now();
        self.count += 1;
        self.timestamps.push(now);
    }

    /// Get count within time window
    pub fn count_in_window(&self, period: QuotaPeriod) -> usize {
        match period.duration() {
            None => self.count, // Session-based, return total
            Some(duration) => {
                let cutoff = Utc::now() - duration;
                self.timestamps.iter().filter(|&&ts| ts > cutoff).count()
            }
        }
    }

    /// Clean up old timestamps outside the window
    pub fn cleanup(&mut self, period: QuotaPeriod) {
        if let Some(duration) = period.duration() {
            let cutoff = Utc::now() - duration;
            self.timestamps.retain(|&ts| ts > cutoff);
            self.count = self.timestamps.len();
        }
    }

    /// Reset quota counters
    pub fn reset(&mut self) {
        self.count = 0;
        self.timestamps.clear();
        self.started_at = Utc::now();
    }
}

/// Result of quota check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaCheckResult {
    /// No quota configured for this tool
    NoQuota,

    /// Operation allowed, remaining operations
    Allowed { remaining: usize, total: usize },

    /// Quota exceeded, should escalate permission
    Exceeded {
        escalate_to: Option<PermissionLevel>,
        message: Option<String>,
    },
}

/// Quota usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaStats {
    pub tool_name: String,
    pub current: usize,
    pub limit: usize,
    pub period: QuotaPeriod,
    pub started_at: DateTime<Utc>,
}

/// Manages quotas for all tools
#[derive(Debug, Default)]
pub struct QuotaManager {
    /// Quota configurations per tool
    configs: HashMap<String, QuotaConfig>,

    /// Current usage tracking per tool
    usage: HashMap<String, QuotaUsage>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            usage: HashMap::new(),
        }
    }

    /// Register a quota config for a tool
    pub fn set_quota(&mut self, tool_name: impl Into<String>, config: QuotaConfig) {
        self.configs.insert(tool_name.into(), config);
    }

    /// Remove quota for a tool
    pub fn remove_quota(&mut self, tool_name: &str) {
        self.configs.remove(tool_name);
        self.usage.remove(tool_name);
    }

    /// Check if operation is allowed under quota
    pub fn check(&mut self, tool_name: &str) -> QuotaCheckResult {
        let Some(config) = self.configs.get(tool_name) else {
            return QuotaCheckResult::NoQuota;
        };

        let usage = self
            .usage
            .entry(tool_name.to_string())
            .or_insert_with(QuotaUsage::new);

        // Clean up old timestamps for time-based quotas
        usage.cleanup(config.period);

        let current_count = usage.count_in_window(config.period);

        if current_count < config.max_operations {
            QuotaCheckResult::Allowed {
                remaining: config.max_operations - current_count,
                total: config.max_operations,
            }
        } else {
            QuotaCheckResult::Exceeded {
                escalate_to: config.escalate_to,
                message: config.message.clone(),
            }
        }
    }

    /// Record an operation (call after successful execution)
    pub fn record(&mut self, tool_name: &str) {
        if self.configs.contains_key(tool_name) {
            let usage = self
                .usage
                .entry(tool_name.to_string())
                .or_insert_with(QuotaUsage::new);
            usage.record();
        }
    }

    /// Get current usage stats
    pub fn get_stats(&self, tool_name: &str) -> Option<QuotaStats> {
        let config = self.configs.get(tool_name)?;
        let usage = self.usage.get(tool_name)?;

        let current_count = usage.count_in_window(config.period);

        Some(QuotaStats {
            tool_name: tool_name.to_string(),
            current: current_count,
            limit: config.max_operations,
            period: config.period,
            started_at: usage.started_at,
        })
    }

    /// Get all quota statistics
    pub fn all_stats(&self) -> Vec<QuotaStats> {
        self.configs
            .keys()
            .filter_map(|name| self.get_stats(name))
            .collect()
    }

    /// Reset quota for a specific tool
    pub fn reset_tool(&mut self, tool_name: &str) {
        if let Some(usage) = self.usage.get_mut(tool_name) {
            usage.reset();
        }
    }

    /// Reset all quotas (e.g., session end)
    pub fn reset_all(&mut self) {
        for usage in self.usage.values_mut() {
            usage.reset();
        }
    }

    /// Export usage for session persistence
    pub fn export_usage(&self) -> HashMap<String, QuotaUsage> {
        self.usage.clone()
    }

    /// Import usage from session
    pub fn import_usage(&mut self, usage: HashMap<String, QuotaUsage>) {
        self.usage = usage;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_period_duration() {
        assert!(QuotaPeriod::Session.duration().is_none());
        assert_eq!(QuotaPeriod::Minute.duration(), Some(Duration::minutes(1)));
        assert_eq!(QuotaPeriod::Hour.duration(), Some(Duration::hours(1)));
        assert_eq!(QuotaPeriod::Day.duration(), Some(Duration::days(1)));
    }

    #[test]
    fn test_quota_config_builders() {
        let quota = QuotaConfig::per_session(10);
        assert_eq!(quota.max_operations, 10);
        assert_eq!(quota.period, QuotaPeriod::Session);

        let quota = QuotaConfig::per_minute(5);
        assert_eq!(quota.max_operations, 5);
        assert_eq!(quota.period, QuotaPeriod::Minute);
    }

    #[test]
    fn test_quota_usage_recording() {
        let mut usage = QuotaUsage::new();

        usage.record();
        assert_eq!(usage.count, 1);
        assert_eq!(usage.timestamps.len(), 1);

        usage.record();
        assert_eq!(usage.count, 2);
        assert_eq!(usage.timestamps.len(), 2);
    }

    #[test]
    fn test_quota_manager_session_based() {
        let mut manager = QuotaManager::new();
        manager.set_quota("test_tool", QuotaConfig::per_session(3));

        matches!(manager.check("test_tool"), QuotaCheckResult::Allowed { .. });
        manager.record("test_tool");

        matches!(manager.check("test_tool"), QuotaCheckResult::Allowed { .. });
        manager.record("test_tool");

        matches!(manager.check("test_tool"), QuotaCheckResult::Allowed { .. });
        manager.record("test_tool");

        matches!(
            manager.check("test_tool"),
            QuotaCheckResult::Exceeded { .. }
        );
    }

    #[test]
    fn test_quota_manager_with_escalation() {
        let mut manager = QuotaManager::new();
        manager.set_quota(
            "test_tool",
            QuotaConfig::per_session(2).with_escalation(PermissionLevel::Ask),
        );

        manager.record("test_tool");
        manager.record("test_tool");

        match manager.check("test_tool") {
            QuotaCheckResult::Exceeded { escalate_to, .. } => {
                assert_eq!(escalate_to, Some(PermissionLevel::Ask));
            }
            _ => panic!("Expected Exceeded"),
        }
    }

    #[test]
    fn test_quota_manager_no_quota() {
        let mut manager = QuotaManager::new();
        assert!(matches!(
            manager.check("unknown_tool"),
            QuotaCheckResult::NoQuota
        ));
    }

    #[test]
    fn test_quota_stats() {
        let mut manager = QuotaManager::new();
        manager.set_quota("test_tool", QuotaConfig::per_session(10));
        manager.record("test_tool");
        manager.record("test_tool");

        let stats = manager.get_stats("test_tool").unwrap();
        assert_eq!(stats.current, 2);
        assert_eq!(stats.limit, 10);
    }

    #[test]
    fn test_quota_reset() {
        let mut manager = QuotaManager::new();
        manager.set_quota("test_tool", QuotaConfig::per_session(5));
        manager.record("test_tool");
        manager.record("test_tool");

        manager.reset_tool("test_tool");
        let stats = manager.get_stats("test_tool").unwrap();
        assert_eq!(stats.current, 0);
    }

    #[test]
    fn test_quota_export_import() {
        let mut manager = QuotaManager::new();
        manager.set_quota("test_tool", QuotaConfig::per_session(5));
        manager.record("test_tool");

        let usage = manager.export_usage();
        assert_eq!(usage.get("test_tool").unwrap().count, 1);

        let mut new_manager = QuotaManager::new();
        new_manager.set_quota("test_tool", QuotaConfig::per_session(5));
        new_manager.import_usage(usage);

        let stats = new_manager.get_stats("test_tool").unwrap();
        assert_eq!(stats.current, 1);
    }
}
