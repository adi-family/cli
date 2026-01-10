//! Pretty terminal output formatter.

use super::Formatter;
use crate::runner::LintResult;
use crate::types::{Category, Diagnostic, Severity};
use std::collections::HashMap;
use std::io::Write;

/// Configuration for pretty output.
#[derive(Debug, Clone)]
pub struct PrettyConfig {
    /// Use colors in output.
    pub colors: bool,
    /// Group diagnostics by category.
    pub group_by_category: bool,
    /// Show file context (source lines).
    pub show_context: bool,
    /// Show fixable indicator.
    pub show_fixable: bool,
}

impl Default for PrettyConfig {
    fn default() -> Self {
        Self {
            colors: true,
            group_by_category: true,
            show_context: false,
            show_fixable: true,
        }
    }
}

/// Pretty terminal formatter.
pub struct PrettyFormatter {
    config: PrettyConfig,
}

impl Default for PrettyFormatter {
    fn default() -> Self {
        Self {
            config: PrettyConfig::default(),
        }
    }
}

impl PrettyFormatter {
    /// Create a new pretty formatter.
    pub fn new(config: PrettyConfig) -> Self {
        Self { config }
    }

    fn severity_color(&self, severity: Severity) -> &'static str {
        if !self.config.colors {
            return "";
        }
        match severity {
            Severity::Error => "\x1b[31m",   // Red
            Severity::Warning => "\x1b[33m", // Yellow
            Severity::Info => "\x1b[36m",    // Cyan
            Severity::Hint => "\x1b[90m",    // Gray
        }
    }

    fn reset(&self) -> &'static str {
        if self.config.colors {
            "\x1b[0m"
        } else {
            ""
        }
    }

    fn bold(&self) -> &'static str {
        if self.config.colors {
            "\x1b[1m"
        } else {
            ""
        }
    }

    fn dim(&self) -> &'static str {
        if self.config.colors {
            "\x1b[2m"
        } else {
            ""
        }
    }

    fn format_diagnostic<W: Write>(&self, diag: &Diagnostic, w: &mut W) -> anyhow::Result<()> {
        let color = self.severity_color(diag.severity);
        let reset = self.reset();
        let dim = self.dim();

        // Format: SEVERITY file:line:col message [rule-id]
        write!(
            w,
            "   {color}{:5}{reset}  {}:{}:{}  {}",
            diag.severity.label(),
            diag.location.file.display(),
            diag.location.start_line,
            diag.location.start_col,
            diag.message,
        )?;

        // Rule ID
        write!(w, " {dim}[{}]{reset}", diag.rule_id)?;

        // Fixable indicator
        if self.config.show_fixable && diag.is_fixable() {
            write!(w, " {dim}[fixable]{reset}")?;
        }

        writeln!(w)?;

        Ok(())
    }

    fn format_category_group<W: Write>(
        &self,
        category: &Category,
        diagnostics: &[&Diagnostic],
        w: &mut W,
    ) -> anyhow::Result<()> {
        let bold = self.bold();
        let reset = self.reset();

        // Category header
        writeln!(
            w,
            "\n{bold}{} {} ({} issues){reset}",
            category.icon(),
            category.display_name(),
            diagnostics.len()
        )?;

        // Sort by severity, then file, then line
        let mut sorted = diagnostics.to_vec();
        sorted.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.location.file.cmp(&b.location.file))
                .then_with(|| a.location.start_line.cmp(&b.location.start_line))
        });

        for diag in sorted {
            self.format_diagnostic(diag, w)?;
        }

        Ok(())
    }

    fn format_summary<W: Write>(&self, result: &LintResult, w: &mut W) -> anyhow::Result<()> {
        let bold = self.bold();
        let reset = self.reset();
        let dim = self.dim();

        writeln!(w)?;
        writeln!(w, "{dim}{}{reset}", "─".repeat(60))?;

        // Count by severity
        let errors = result
            .by_severity
            .get(&Severity::Error)
            .copied()
            .unwrap_or(0);
        let warnings = result
            .by_severity
            .get(&Severity::Warning)
            .copied()
            .unwrap_or(0);
        let infos = result
            .by_severity
            .get(&Severity::Info)
            .copied()
            .unwrap_or(0);
        let hints = result
            .by_severity
            .get(&Severity::Hint)
            .copied()
            .unwrap_or(0);

        let total = result.diagnostics.len();
        let fixable = result.fixable_count();
        let categories = result.by_category.len();

        write!(w, "{bold}Summary:{reset} ")?;

        let mut parts = Vec::new();
        if errors > 0 {
            parts.push(format!(
                "{}{} errors{}",
                self.severity_color(Severity::Error),
                errors,
                reset
            ));
        }
        if warnings > 0 {
            parts.push(format!(
                "{}{} warnings{}",
                self.severity_color(Severity::Warning),
                warnings,
                reset
            ));
        }
        if infos > 0 {
            parts.push(format!(
                "{}{} info{}",
                self.severity_color(Severity::Info),
                infos,
                reset
            ));
        }
        if hints > 0 {
            parts.push(format!(
                "{}{} hints{}",
                self.severity_color(Severity::Hint),
                hints,
                reset
            ));
        }

        if parts.is_empty() {
            writeln!(w, "No issues found!")?;
        } else {
            writeln!(w, "{}", parts.join(", "))?;
            writeln!(
                w,
                "         {total} issues across {categories} categories, {fixable} fixable"
            )?;
        }

        // Duration and files
        writeln!(
            w,
            "{dim}Checked {} files in {:?}{reset}",
            result.files_checked, result.duration
        )?;

        // Errors during linting
        if !result.errors.is_empty() {
            writeln!(w)?;
            writeln!(
                w,
                "{}Linter errors:{reset}",
                self.severity_color(Severity::Warning)
            )?;
            for err in &result.errors {
                writeln!(
                    w,
                    "  - {}: {}",
                    err.linter_id,
                    err.message
                )?;
            }
        }

        Ok(())
    }
}

impl Formatter for PrettyFormatter {
    fn format<W: Write>(&self, result: &LintResult, w: &mut W) -> anyhow::Result<()> {
        if result.diagnostics.is_empty() {
            let bold = self.bold();
            let reset = self.reset();
            writeln!(w, "{bold}No issues found!{reset}")?;
            writeln!(
                w,
                "Checked {} files in {:?}",
                result.files_checked, result.duration
            )?;
            return Ok(());
        }

        if self.config.group_by_category {
            // Group diagnostics by primary category
            let mut by_category: HashMap<Category, Vec<&Diagnostic>> = HashMap::new();
            for diag in &result.diagnostics {
                by_category
                    .entry(diag.primary_category().clone())
                    .or_default()
                    .push(diag);
            }

            // Sort categories by priority (descending)
            let mut categories: Vec<_> = by_category.keys().cloned().collect();
            categories.sort_by(|a, b| b.default_priority().cmp(&a.default_priority()));

            for category in categories {
                if let Some(diagnostics) = by_category.get(&category) {
                    self.format_category_group(&category, diagnostics, w)?;
                }
            }
        } else {
            // Flat list sorted by file then line
            let mut sorted = result.diagnostics.clone();
            sorted.sort_by(|a, b| {
                a.location
                    .file
                    .cmp(&b.location.file)
                    .then_with(|| a.location.start_line.cmp(&b.location.start_line))
            });

            for diag in &sorted {
                self.format_diagnostic(diag, w)?;
            }
        }

        self.format_summary(result, w)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Location;
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
            Diagnostic::new(
                "sec-001",
                "security-linter",
                Category::Security,
                Severity::Error,
                "Hardcoded password detected",
                Location::new(PathBuf::from("src/config.rs"), 25, 1, 25, 30),
            ),
        ];

        let mut by_severity = HashMap::new();
        by_severity.insert(Severity::Error, 1);
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
        by_category.insert(
            "Security".to_string(),
            crate::runner::CategorySummary {
                total: 1,
                by_severity: {
                    let mut m = HashMap::new();
                    m.insert(Severity::Error, 1);
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
    fn test_pretty_format() {
        let result = create_test_result();
        let formatter = PrettyFormatter::new(PrettyConfig {
            colors: false,
            ..Default::default()
        });

        let mut output = Vec::new();
        formatter.format(&result, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("Security"));
        assert!(output_str.contains("Code Quality"));
        assert!(output_str.contains("Hardcoded password detected"));
        assert!(output_str.contains("Found TODO comment"));
        assert!(output_str.contains("1 errors"));
        assert!(output_str.contains("1 warnings"));
    }

    #[test]
    fn test_pretty_no_issues() {
        let result = LintResult {
            diagnostics: vec![],
            files_checked: 10,
            duration: Duration::from_millis(50),
            errors: vec![],
            by_category: HashMap::new(),
            by_severity: HashMap::new(),
        };

        let formatter = PrettyFormatter::new(PrettyConfig {
            colors: false,
            ..Default::default()
        });

        let mut output = Vec::new();
        formatter.format(&result, &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("No issues found!"));
    }
}
