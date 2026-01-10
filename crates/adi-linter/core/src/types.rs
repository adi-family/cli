//! Core types for the linter system.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Linter category - domain classification for organizing linters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// Dependencies, layering, module boundaries
    Architecture,
    /// Vulnerabilities, secrets, injection, OWASP
    Security,
    /// Complexity, duplication, SOLID principles
    CodeQuality,
    /// Language idioms, anti-patterns, deprecations
    BestPractices,
    /// Bugs, logic errors, null safety, type issues
    Correctness,
    /// Error propagation, panic handling, exceptions
    ErrorHandling,
    /// Inefficient patterns, memory, allocations
    Performance,
    /// Formatting, whitespace, conventions
    Style,
    /// Naming conventions, casing
    Naming,
    /// Missing docs, outdated comments, API docs
    Documentation,
    /// Test quality, coverage, assertions
    Testing,
    /// User-defined category
    #[serde(untagged)]
    Custom(String),
}

impl Category {
    /// Default priority for this category.
    /// Higher value = more important = runs first.
    pub fn default_priority(&self) -> u32 {
        match self {
            Category::Security => 1000,
            Category::Correctness => 750,
            Category::ErrorHandling => 750,
            Category::Architecture => 750,
            Category::Performance => 500,
            Category::CodeQuality => 500,
            Category::BestPractices => 500,
            Category::Testing => 500,
            Category::Documentation => 250,
            Category::Naming => 250,
            Category::Style => 100,
            Category::Custom(_) => 500,
        }
    }

    /// Icon for pretty output.
    pub fn icon(&self) -> &'static str {
        match self {
            Category::Security => "[SEC]",
            Category::Correctness => "[BUG]",
            Category::ErrorHandling => "[ERR]",
            Category::Architecture => "[ARC]",
            Category::Performance => "[PRF]",
            Category::CodeQuality => "[QUA]",
            Category::BestPractices => "[BST]",
            Category::Testing => "[TST]",
            Category::Documentation => "[DOC]",
            Category::Naming => "[NAM]",
            Category::Style => "[STY]",
            Category::Custom(_) => "[CUS]",
        }
    }

    /// Human-readable name.
    pub fn display_name(&self) -> &str {
        match self {
            Category::Security => "Security",
            Category::Correctness => "Correctness",
            Category::ErrorHandling => "Error Handling",
            Category::Architecture => "Architecture",
            Category::Performance => "Performance",
            Category::CodeQuality => "Code Quality",
            Category::BestPractices => "Best Practices",
            Category::Testing => "Testing",
            Category::Documentation => "Documentation",
            Category::Naming => "Naming",
            Category::Style => "Style",
            Category::Custom(name) => name.as_str(),
        }
    }
}

impl Default for Category {
    fn default() -> Self {
        Category::CodeQuality
    }
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Hint = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
}

impl Severity {
    /// Short label for output.
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Error => "ERROR",
            Severity::Warning => "WARN",
            Severity::Info => "INFO",
            Severity::Hint => "HINT",
        }
    }
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Warning
    }
}

/// A diagnostic issue reported by a linter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Rule ID within the linter (e.g., "no-unwrap").
    pub rule_id: String,
    /// Linter ID that produced this diagnostic.
    pub linter_id: String,
    /// Category classifications (supports multiple categories per issue).
    pub categories: Vec<Category>,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Primary location of the issue.
    pub location: Location,
    /// Optional auto-fix (None = not fixable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Fix>,
    /// Related locations for context.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<RelatedInfo>,
    /// Additional tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<DiagnosticTag>,
}

impl Diagnostic {
    /// Create a new diagnostic with a single category.
    pub fn new(
        rule_id: impl Into<String>,
        linter_id: impl Into<String>,
        category: Category,
        severity: Severity,
        message: impl Into<String>,
        location: Location,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            linter_id: linter_id.into(),
            categories: vec![category],
            severity,
            message: message.into(),
            location,
            fix: None,
            related: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Create a new diagnostic with multiple categories.
    pub fn with_categories(
        rule_id: impl Into<String>,
        linter_id: impl Into<String>,
        categories: Vec<Category>,
        severity: Severity,
        message: impl Into<String>,
        location: Location,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            linter_id: linter_id.into(),
            categories: if categories.is_empty() {
                vec![Category::default()]
            } else {
                categories
            },
            severity,
            message: message.into(),
            location,
            fix: None,
            related: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Get the primary category (first in the list).
    pub fn primary_category(&self) -> &Category {
        self.categories.first().unwrap_or(&Category::CodeQuality)
    }

    /// Check if diagnostic belongs to a specific category.
    pub fn has_category(&self, category: &Category) -> bool {
        self.categories.contains(category)
    }

    /// Add a category to this diagnostic.
    pub fn add_category(&mut self, category: Category) {
        if !self.categories.contains(&category) {
            self.categories.push(category);
        }
    }

    /// Add an auto-fix.
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    /// Add related info.
    pub fn with_related(mut self, related: RelatedInfo) -> Self {
        self.related.push(related);
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: DiagnosticTag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Check if this diagnostic is auto-fixable.
    pub fn is_fixable(&self) -> bool {
        self.fix.is_some()
    }
}

/// Source location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// File path.
    pub file: PathBuf,
    /// Start line (1-based).
    pub start_line: u32,
    /// Start column (1-based).
    pub start_col: u32,
    /// End line (1-based).
    pub end_line: u32,
    /// End column (1-based).
    pub end_col: u32,
}

impl Location {
    /// Create a new location.
    pub fn new(file: PathBuf, start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            file,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// Create a location for a single line.
    pub fn line(file: PathBuf, line: u32) -> Self {
        Self {
            file,
            start_line: line,
            start_col: 1,
            end_line: line,
            end_col: u32::MAX,
        }
    }

    /// Create a location for an entire file.
    pub fn file(file: PathBuf) -> Self {
        Self {
            file,
            start_line: 1,
            start_col: 1,
            end_line: u32::MAX,
            end_col: u32::MAX,
        }
    }
}

/// Additional context location for a diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedInfo {
    /// Context message.
    pub message: String,
    /// Location of the related info.
    pub location: Location,
}

/// Additional metadata tags for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticTag {
    /// Using deprecated API.
    Deprecated,
    /// Dead code, unused import.
    Unnecessary,
    /// Needs security review.
    SecurityHotspot,
    /// Breaking change detected.
    Breaking,
}

/// An auto-fix for a diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    /// Human-readable description of the fix.
    pub description: String,
    /// Text edits to apply.
    pub edits: Vec<TextEdit>,
    /// Mark as preferred fix if multiple are available.
    #[serde(default)]
    pub is_preferred: bool,
}

impl Fix {
    /// Create a new fix.
    pub fn new(description: impl Into<String>, edits: Vec<TextEdit>) -> Self {
        Self {
            description: description.into(),
            edits,
            is_preferred: false,
        }
    }

    /// Create a simple single-edit fix.
    pub fn simple(description: impl Into<String>, file: PathBuf, range: Range, new_text: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            edits: vec![TextEdit {
                file,
                range,
                new_text: new_text.into(),
            }],
            is_preferred: false,
        }
    }

    /// Mark as preferred.
    pub fn preferred(mut self) -> Self {
        self.is_preferred = true;
        self
    }
}

/// A single text edit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// File to edit (supports cross-file fixes).
    pub file: PathBuf,
    /// Byte range to replace.
    pub range: Range,
    /// New text to insert.
    pub new_text: String,
}

/// Byte range in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    /// Start byte offset.
    pub start: usize,
    /// End byte offset.
    pub end: usize,
}

impl Range {
    /// Create a new range.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Check if range is empty.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Get range length.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// Scope at which a linter operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LintScope {
    /// Linter receives entire file content.
    #[default]
    File,
    /// Linter is called per line.
    Line,
    /// Linter is called per symbol (requires indexer).
    Symbol,
}

/// How external linter receives input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputMode {
    /// File path is passed via template variable.
    #[default]
    FilePath,
    /// Content is piped to stdin.
    Stdin,
    /// Both file path and stdin available.
    Both,
}

/// How external linter outputs results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    /// Expects standard JSON diagnostic format.
    #[default]
    Json,
    /// Exit code only (0 = pass, non-zero = fail).
    ExitCode,
    /// Each line is a diagnostic (format: line:col:message).
    Lines,
}

/// Priority level constants.
pub mod priority {
    /// Security, must-fix issues.
    pub const CRITICAL: u32 = 1000;
    /// Errors, blockers.
    pub const HIGH: u32 = 750;
    /// Standard linting.
    pub const NORMAL: u32 = 500;
    /// Style, suggestions.
    pub const LOW: u32 = 250;
    /// Formatting, whitespace.
    pub const COSMETIC: u32 = 100;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_priority() {
        assert!(Category::Security.default_priority() > Category::CodeQuality.default_priority());
        assert!(Category::CodeQuality.default_priority() > Category::Style.default_priority());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
        assert!(Severity::Info > Severity::Hint);
    }

    #[test]
    fn test_diagnostic_builder() {
        let diag = Diagnostic::new(
            "test-rule",
            "test-linter",
            Category::Security,
            Severity::Error,
            "Test message",
            Location::line(PathBuf::from("test.rs"), 42),
        )
        .with_tag(DiagnosticTag::SecurityHotspot);

        assert_eq!(diag.rule_id, "test-rule");
        assert!(diag.tags.contains(&DiagnosticTag::SecurityHotspot));
        assert!(!diag.is_fixable());
        assert!(diag.has_category(&Category::Security));
    }

    #[test]
    fn test_diagnostic_multi_category() {
        let diag = Diagnostic::with_categories(
            "memory-leak",
            "test-linter",
            vec![Category::Security, Category::Performance, Category::Correctness],
            Severity::Error,
            "Potential memory leak detected",
            Location::line(PathBuf::from("test.rs"), 10),
        );

        assert_eq!(diag.categories.len(), 3);
        assert!(diag.has_category(&Category::Security));
        assert!(diag.has_category(&Category::Performance));
        assert!(diag.has_category(&Category::Correctness));
        assert!(!diag.has_category(&Category::Style));
        assert_eq!(diag.primary_category(), &Category::Security);
    }
}
