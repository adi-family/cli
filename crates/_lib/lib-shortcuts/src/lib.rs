//! Standardized shortcut URL registry for ADI products.
//!
//! All external links across ADI components use this crate to build URLs.
//! Pattern: `{BASE}/{shortcut-name}`
//!
//! To change the base domain or prefix, update `BASE` — every link updates at once.

/// Base URL for all shortcut links.
pub const BASE: &str = "https://adi.the-ihor.com/sc";

/// Build a full shortcut URL from a shortcut name.
pub fn url(name: &str) -> String {
    format!("{}/{}", BASE, name)
}

// ── Hive error pages ──

pub const HIVE_ERROR_400: &str = "hive-error-400";
pub const HIVE_ERROR_400_LLM: &str = "hive-error-400-llm";
pub const HIVE_ERROR_404: &str = "hive-error-404";
pub const HIVE_ERROR_404_LLM: &str = "hive-error-404-llm";
pub const HIVE_ERROR_502: &str = "hive-error-502";
pub const HIVE_ERROR_502_LLM: &str = "hive-error-502-llm";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_builds_correctly() {
        assert_eq!(url("hive-error-404"), "https://adi.the-ihor.com/sc/hive-error-404");
    }

    #[test]
    fn test_all_shortcuts_are_unique() {
        let all = [
            HIVE_ERROR_400,
            HIVE_ERROR_400_LLM,
            HIVE_ERROR_404,
            HIVE_ERROR_404_LLM,
            HIVE_ERROR_502,
            HIVE_ERROR_502_LLM,
        ];
        let mut seen = std::collections::HashSet::new();
        for sc in &all {
            assert!(seen.insert(sc), "duplicate shortcut: {}", sc);
        }
    }
}
