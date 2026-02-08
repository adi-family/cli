//! Utility functions and types

/// Re-export common types
pub use crate::core::{Plugin, PluginMetadata, PluginContext, PluginEvent, PluginType};
pub use crate::error::{PluginError, Result};
pub use crate::runner::RuntimeContext;

use std::time::Duration;

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
