//! JSON output formatter.

use super::Formatter;
use crate::runner::LintResult;
use crate::types::Diagnostic;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;

/// Configuration for JSON output.
#[derive(Debug, Clone)]
pub struct JsonConfig {
    /// Pretty print JSON.
    pub pretty: bool,
}

impl Default for JsonConfig {
    fn default() -> Self {
        Self { pretty: false }
    }
}

/// JSON formatter.
pub struct JsonFormatter {
    config: JsonConfig,
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self {
            config: JsonConfig::default(),
        }
    }
}

impl JsonFormatter {
    /// Create a new JSON formatter.
    pub fn new(config: JsonConfig) -> Self {
        Self { config }
    }

    /// Create a pretty-printing JSON formatter.
    pub fn pretty() -> Self {
        Self {
            config: JsonConfig { pretty: true },
        }
    }
}

impl Formatter for JsonFormatter {
    fn format<W: Write>(&self, result: &LintResult, w: &mut W) -> anyhow::Result<()> {
        let output = JsonOutput::from_result(result);

        if self.config.pretty {
            serde_json::to_writer_pretty(&mut *w, &output)?;
        } else {
            serde_json::to_writer(&mut *w, &output)?;
        }

        writeln!(w)?;
        Ok(())
    }
}

/// JSON output structure.
#[derive(Debug, Serialize)]
pub struct JsonOutput {
    /// Output format version.
    pub version: &'static str,
    /// All diagnostics.
    pub diagnostics: Vec<JsonDiagnostic>,
    /// Summary information.
    pub summary: JsonSummary,
    /// Linter errors (not lint issues).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<JsonError>,
}

impl JsonOutput {
    /// Create from a LintResult.
    pub fn from_result(result: &LintResult) -> Self {
        Self {
            version: "1.0",
            diagnostics: result.diagnostics.iter().map(JsonDiagnostic::from).collect(),
            summary: JsonSummary::from_result(result),
            errors: result.errors.iter().map(|e| JsonError {
                linter_id: e.linter_id.clone(),
                file: e.file.as_ref().map(|p| p.to_string_lossy().to_string()),
                message: e.message.clone(),
            }).collect(),
        }
    }
}

/// JSON diagnostic structure.
#[derive(Debug, Serialize)]
pub struct JsonDiagnostic {
    /// Rule ID.
    pub rule_id: String,
    /// Linter ID.
    pub linter_id: String,
    /// Categories (supports multiple).
    pub categories: Vec<String>,
    /// Severity.
    pub severity: String,
    /// Message.
    pub message: String,
    /// Location.
    pub location: JsonLocation,
    /// Whether this diagnostic is fixable.
    pub fixable: bool,
    /// Fix description (if fixable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_description: Option<String>,
    /// Diagnostic tags.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl From<&Diagnostic> for JsonDiagnostic {
    fn from(diag: &Diagnostic) -> Self {
        Self {
            rule_id: diag.rule_id.clone(),
            linter_id: diag.linter_id.clone(),
            categories: diag.categories.iter().map(|c| c.display_name().to_string()).collect(),
            severity: format!("{:?}", diag.severity).to_lowercase(),
            message: diag.message.clone(),
            location: JsonLocation {
                file: diag.location.file.to_string_lossy().to_string(),
                start_line: diag.location.start_line,
                start_col: diag.location.start_col,
                end_line: diag.location.end_line,
                end_col: diag.location.end_col,
            },
            fixable: diag.is_fixable(),
            fix_description: diag.fix.as_ref().map(|f| f.description.clone()),
            tags: diag.tags.iter().map(|t| format!("{:?}", t).to_lowercase()).collect(),
        }
    }
}

/// JSON location structure.
#[derive(Debug, Serialize)]
pub struct JsonLocation {
    /// File path.
    pub file: String,
    /// Start line (1-based).
    pub start_line: u32,
    /// Start column (1-based).
    pub start_col: u32,
    /// End line (1-based).
    pub end_line: u32,
    /// End column (1-based).
    pub end_col: u32,
}

/// JSON summary structure.
#[derive(Debug, Serialize)]
pub struct JsonSummary {
    /// Total diagnostics.
    pub total: usize,
    /// Count by severity.
    pub by_severity: HashMap<String, usize>,
    /// Count by category.
    pub by_category: HashMap<String, usize>,
    /// Fixable count.
    pub fixable: usize,
    /// Files checked.
    pub files_checked: usize,
    /// Duration in milliseconds.
    pub duration_ms: u64,
}

impl JsonSummary {
    /// Create from a LintResult.
    pub fn from_result(result: &LintResult) -> Self {
        Self {
            total: result.diagnostics.len(),
            by_severity: result
                .by_severity
                .iter()
                .map(|(k, v)| (format!("{:?}", k).to_lowercase(), *v))
                .collect(),
            by_category: result
                .by_category
                .iter()
                .map(|(k, v)| (k.clone(), v.total))
                .collect(),
            fixable: result.fixable_count(),
            files_checked: result.files_checked,
            duration_ms: result.duration.as_millis() as u64,
        }
    }
}

/// JSON error structure.
#[derive(Debug, Serialize)]
pub struct JsonError {
    /// Linter ID.
    pub linter_id: String,
    /// File (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Error message.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Category, Location, Severity};
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_test_result() -> LintResult {
        let diagnostics = vec![
            Diagnostic::new(
                "no-todo",
                "test-linter",
                Category::CodeQuality,
                Severity::Warning,
                "Found TODO comment",
                Location::new(PathBuf::from("src/main.rs"), 10, 5, 10, 20),
            ),
        ];

        let mut by_severity = HashMap::new();
        by_severity.insert(Severity::Warning, 1);

        let mut by_category = HashMap::new();
        by_category.insert(
            "Code Quality".to_string(),
            crate::runner::CategorySummary {
                total: 1,
                by_severity: {
                    let mut m = HashMap::new();
                    m.insert(Severity::Warning, 1);
                    m
                },
                fixable: 0,
            },
        );

        LintResult {
            diagnostics,
            files_checked: 5,
            duration: Duration::from_millis(150),
            errors: vec![],
            by_category,
            by_severity,
        }
    }

    #[test]
    fn test_json_format() {
        let result = create_test_result();
        let formatter = JsonFormatter::default();

        let mut output = Vec::new();
        formatter.format(&result, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        // Parse as JSON to verify it's valid
        let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

        assert_eq!(parsed["version"], "1.0");
        assert_eq!(parsed["diagnostics"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["summary"]["total"], 1);
    }

    #[test]
    fn test_json_pretty() {
        let result = create_test_result();
        let formatter = JsonFormatter::pretty();

        let mut output = Vec::new();
        formatter.format(&result, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        // Pretty JSON should have newlines
        assert!(output_str.contains("\n"));
        assert!(output_str.contains("  ")); // Indentation
    }
}
