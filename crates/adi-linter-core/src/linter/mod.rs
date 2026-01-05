//! Linter trait and implementations.

pub mod command;
pub mod external;

use crate::types::{Category, Diagnostic, LintScope};
use async_trait::async_trait;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

/// Context passed to linters during execution.
#[derive(Debug, Clone)]
pub struct LintContext {
    /// File path being linted.
    pub file: std::path::PathBuf,
    /// File content.
    pub content: String,
    /// Current line number (if scope == Line).
    pub line: Option<u32>,
    /// Line content (if scope == Line).
    pub line_content: Option<String>,
}

impl LintContext {
    /// Create a new file-level context.
    pub fn file(file: impl Into<std::path::PathBuf>, content: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            content: content.into(),
            line: None,
            line_content: None,
        }
    }

    /// Create a line-level context.
    pub fn line(
        file: impl Into<std::path::PathBuf>,
        content: impl Into<String>,
        line_num: u32,
        line_content: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            content: content.into(),
            line: Some(line_num),
            line_content: Some(line_content.into()),
        }
    }
}

/// Unified interface for all linter types.
#[async_trait]
pub trait Linter: Send + Sync {
    /// Unique identifier for this linter.
    fn id(&self) -> &str;

    /// Category classifications (supports multiple categories).
    fn categories(&self) -> &[Category];

    /// Get the primary category (first in list).
    fn primary_category(&self) -> Category {
        self.categories().first().cloned().unwrap_or_default()
    }

    /// Check if linter belongs to a specific category.
    fn has_category(&self, category: &Category) -> bool {
        self.categories().contains(category)
    }

    /// Priority (higher = runs first, fixes applied first).
    /// Defaults to the highest priority among all categories.
    fn priority(&self) -> u32 {
        self.categories()
            .iter()
            .map(|c| c.default_priority())
            .max()
            .unwrap_or(500)
    }

    /// Glob patterns this linter applies to.
    fn patterns(&self) -> &[String];

    /// Check if linter applies to this file path.
    fn matches(&self, path: &Path) -> bool;

    /// Run linting on the given context.
    async fn lint(&self, ctx: &LintContext) -> anyhow::Result<Vec<Diagnostic>>;

    /// Scope at which this linter operates.
    fn scope(&self) -> LintScope {
        LintScope::File
    }
}

/// Helper to build a GlobSet from patterns.
pub fn build_glob_set(patterns: &[String]) -> anyhow::Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}

/// Base configuration shared by all linter types.
#[derive(Debug, Clone)]
pub struct LinterConfig {
    /// Unique identifier.
    pub id: String,
    /// Category classifications (supports multiple).
    pub categories: Vec<Category>,
    /// Priority override (None = use highest category default).
    pub priority: Option<u32>,
    /// Glob patterns for file matching.
    pub patterns: Vec<String>,
    /// Compiled glob set.
    glob_set: GlobSet,
}

impl LinterConfig {
    /// Create a new linter config with a single category.
    pub fn new(
        id: impl Into<String>,
        category: Category,
        patterns: Vec<String>,
    ) -> anyhow::Result<Self> {
        Self::with_categories(id, vec![category], patterns)
    }

    /// Create a new linter config with multiple categories.
    pub fn with_categories(
        id: impl Into<String>,
        categories: Vec<Category>,
        patterns: Vec<String>,
    ) -> anyhow::Result<Self> {
        let glob_set = build_glob_set(&patterns)?;
        Ok(Self {
            id: id.into(),
            categories: if categories.is_empty() {
                vec![Category::default()]
            } else {
                categories
            },
            priority: None,
            patterns,
            glob_set,
        })
    }

    /// Set priority override.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Get effective priority (priority override or highest category default).
    pub fn effective_priority(&self) -> u32 {
        self.priority.unwrap_or_else(|| {
            self.categories
                .iter()
                .map(|c| c.default_priority())
                .max()
                .unwrap_or(500)
        })
    }

    /// Get primary category.
    pub fn primary_category(&self) -> &Category {
        self.categories.first().unwrap_or(&Category::CodeQuality)
    }

    /// Check if config has a specific category.
    pub fn has_category(&self, category: &Category) -> bool {
        self.categories.contains(category)
    }

    /// Check if path matches patterns.
    pub fn matches(&self, path: &Path) -> bool {
        self.glob_set.is_match(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_matching() {
        let config = LinterConfig::new(
            "test",
            Category::CodeQuality,
            vec!["**/*.rs".to_string(), "**/*.ts".to_string()],
        )
        .unwrap();

        assert!(config.matches(Path::new("src/main.rs")));
        assert!(config.matches(Path::new("lib/utils.ts")));
        assert!(!config.matches(Path::new("readme.md")));
    }

    #[test]
    fn test_priority_override() {
        let config = LinterConfig::new("test", Category::Style, vec!["**/*".to_string()])
            .unwrap()
            .with_priority(999);

        assert_eq!(config.effective_priority(), 999);
    }
}
