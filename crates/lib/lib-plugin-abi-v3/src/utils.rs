//! Utility functions and types

/// Re-export common types
pub use crate::core::{Plugin, PluginMetadata, PluginContext, PluginEvent, PluginType};
pub use crate::error::{PluginError, Result};
pub use crate::runner::RuntimeContext;

use std::time::Duration;

/// Resolved shell program and flag for command execution.
#[derive(Debug, Clone)]
pub struct ShellInvocation {
    pub program: String,
    pub flag: &'static str,
}

/// Resolve the user's shell for command execution.
///
/// `config_shell` overrides `$SHELL` env var; falls back to `"sh"`.
/// `login` uses `-lc` (sources full profile) so PATH setup like nvm/fnm is available.
pub fn resolve_shell(config_shell: Option<&str>, login: bool) -> ShellInvocation {
    if cfg!(target_os = "windows") {
        return ShellInvocation {
            program: "cmd".into(),
            flag: "/C",
        };
    }

    let program = config_shell
        .map(String::from)
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "sh".into());

    ShellInvocation {
        program,
        flag: if login { "-lc" } else { "-c" },
    }
}

/// Parse a duration string (e.g., "10s", "5m", "500ms", "1h").
/// Falls back to treating bare numbers as seconds.
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();

    if let Some(ms) = s.strip_suffix("ms") {
        ms.parse::<u64>().ok().map(Duration::from_millis)
    } else if let Some(s_val) = s.strip_suffix('s') {
        s_val.parse::<u64>().ok().map(Duration::from_secs)
    } else if let Some(m) = s.strip_suffix('m') {
        m.parse::<u64>().ok().map(|m| Duration::from_secs(m * 60))
    } else if let Some(h) = s.strip_suffix('h') {
        h.parse::<u64>().ok().map(|h| Duration::from_secs(h * 3600))
    } else {
        // Try parsing as seconds
        s.parse::<u64>().ok().map(Duration::from_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("10s"), Some(Duration::from_secs(10)));
        assert_eq!(parse_duration("500ms"), Some(Duration::from_millis(500)));
        assert_eq!(parse_duration("5m"), Some(Duration::from_secs(300)));
        assert_eq!(parse_duration("1h"), Some(Duration::from_secs(3600)));
        assert_eq!(parse_duration("30"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("abc"), None);
    }
}
