//! Autofix engine - sequential fix application with re-linting.

use crate::runner::Runner;
use crate::types::{Diagnostic, Fix, Range, TextEdit};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Configuration for the autofix engine.
#[derive(Debug, Clone)]
pub struct AutofixConfig {
    /// Maximum number of fix iterations.
    pub max_iterations: usize,
    /// Dry run mode (show fixes without applying).
    pub dry_run: bool,
    /// Interactive mode (prompt before each fix).
    pub interactive: bool,
}

impl Default for AutofixConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            dry_run: false,
            interactive: false,
        }
    }
}

/// Result of an autofix run.
#[derive(Debug)]
pub struct AutofixResult {
    /// Fixes that were applied.
    pub fixes_applied: Vec<AppliedFix>,
    /// Remaining diagnostics after all fixes.
    pub remaining_diagnostics: Vec<Diagnostic>,
    /// Number of iterations performed.
    pub iterations: usize,
    /// Whether max iterations was reached.
    pub max_iterations_reached: bool,
}

impl AutofixResult {
    /// Get number of fixes applied.
    pub fn fixes_count(&self) -> usize {
        self.fixes_applied.len()
    }

    /// Get number of remaining issues.
    pub fn remaining_count(&self) -> usize {
        self.remaining_diagnostics.len()
    }

    /// Check if all issues were fixed.
    pub fn all_fixed(&self) -> bool {
        self.remaining_diagnostics.is_empty()
    }
}

/// A fix that was applied.
#[derive(Debug)]
pub struct AppliedFix {
    /// The original diagnostic.
    pub diagnostic: Diagnostic,
    /// The fix that was applied.
    pub fix: Fix,
    /// Iteration number when applied.
    pub iteration: usize,
}

/// Autofix engine.
pub struct AutofixEngine<'a> {
    runner: &'a Runner,
    config: AutofixConfig,
    /// Callback for interactive mode.
    prompt_callback: Option<Box<dyn Fn(&Diagnostic, &Fix) -> bool + 'a>>,
}

impl<'a> AutofixEngine<'a> {
    /// Create a new autofix engine.
    pub fn new(runner: &'a Runner, config: AutofixConfig) -> Self {
        Self {
            runner,
            config,
            prompt_callback: None,
        }
    }

    /// Set the prompt callback for interactive mode.
    pub fn with_prompt<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Diagnostic, &Fix) -> bool + 'a,
    {
        self.prompt_callback = Some(Box::new(callback));
        self
    }

    /// Run autofix.
    pub async fn run(&self, files: Option<Vec<PathBuf>>) -> anyhow::Result<AutofixResult> {
        let mut applied_fixes = Vec::new();
        let mut iteration = 0;
        let mut skipped_diagnostics: Vec<String> = Vec::new();

        loop {
            iteration += 1;
            if iteration > self.config.max_iterations {
                // Get final state
                let lint_result = self.runner.run(files.clone()).await?;
                return Ok(AutofixResult {
                    fixes_applied: applied_fixes,
                    remaining_diagnostics: lint_result.diagnostics,
                    iterations: iteration - 1,
                    max_iterations_reached: true,
                });
            }

            // Run all linters
            let lint_result = self.runner.run(files.clone()).await?;

            // Collect fixable diagnostics, excluding skipped ones
            let fixable = self.collect_fixable(&lint_result.diagnostics, &skipped_diagnostics);

            if fixable.is_empty() {
                // No more fixes available
                return Ok(AutofixResult {
                    fixes_applied: applied_fixes,
                    remaining_diagnostics: lint_result.diagnostics,
                    iterations: iteration,
                    max_iterations_reached: false,
                });
            }

            // Get highest priority fix
            let to_fix = &fixable[0];
            let fix = to_fix.fix.as_ref().unwrap();

            // Dry run - just report
            if self.config.dry_run {
                tracing::info!(
                    "Would fix: {} at {}:{}",
                    to_fix.message,
                    to_fix.location.file.display(),
                    to_fix.location.start_line
                );

                applied_fixes.push(AppliedFix {
                    diagnostic: to_fix.clone(),
                    fix: fix.clone(),
                    iteration,
                });

                // Mark as skipped so we don't try again
                skipped_diagnostics.push(diagnostic_key(to_fix));
                continue;
            }

            // Interactive mode - prompt user
            if self.config.interactive {
                if let Some(ref callback) = self.prompt_callback {
                    if !callback(to_fix, fix) {
                        // User declined
                        skipped_diagnostics.push(diagnostic_key(to_fix));
                        continue;
                    }
                }
            }

            // Apply the fix
            self.apply_fix(fix).await?;

            applied_fixes.push(AppliedFix {
                diagnostic: to_fix.clone(),
                fix: fix.clone(),
                iteration,
            });

            tracing::debug!(
                "Applied fix for {} at {}:{}",
                to_fix.rule_id,
                to_fix.location.file.display(),
                to_fix.location.start_line
            );

            // Loop continues - will re-run ALL linters
        }
    }

    /// Collect fixable diagnostics sorted by priority.
    fn collect_fixable(&self, diagnostics: &[Diagnostic], skipped: &[String]) -> Vec<Diagnostic> {
        let mut fixable: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.is_fixable())
            .filter(|d| !skipped.contains(&diagnostic_key(d)))
            .cloned()
            .collect();

        // Sort by:
        // 1. Linter priority (descending) - via highest category default
        // 2. Severity (descending)
        // 3. File path
        // 4. Line number
        fixable.sort_by(|a, b| {
            // Get priority from highest category (multi-category support)
            let a_priority = a
                .categories
                .iter()
                .map(|c| c.default_priority())
                .max()
                .unwrap_or(500);
            let b_priority = b
                .categories
                .iter()
                .map(|c| c.default_priority())
                .max()
                .unwrap_or(500);

            b_priority
                .cmp(&a_priority)
                .then_with(|| b.severity.cmp(&a.severity))
                .then_with(|| a.location.file.cmp(&b.location.file))
                .then_with(|| a.location.start_line.cmp(&b.location.start_line))
                .then_with(|| a.location.start_col.cmp(&b.location.start_col))
        });

        fixable
    }

    /// Apply a fix to the file(s).
    async fn apply_fix(&self, fix: &Fix) -> anyhow::Result<()> {
        // Group edits by file
        let mut by_file: HashMap<&Path, Vec<&TextEdit>> = HashMap::new();
        for edit in &fix.edits {
            by_file.entry(&edit.file).or_default().push(edit);
        }

        // Apply to each file
        for (file, edits) in by_file {
            let content = tokio::fs::read_to_string(file).await?;
            let new_content = apply_edits(&content, &edits)?;
            tokio::fs::write(file, new_content).await?;
        }

        Ok(())
    }
}

/// Apply text edits to content.
fn apply_edits(content: &str, edits: &[&TextEdit]) -> anyhow::Result<String> {
    let mut edits = edits.to_vec();

    // Sort by position descending (apply from end to start to preserve offsets)
    edits.sort_by(|a, b| b.range.start.cmp(&a.range.start));

    let mut result = content.to_string();

    for edit in edits {
        // Validate range
        if edit.range.start > result.len() || edit.range.end > result.len() {
            anyhow::bail!(
                "Edit range {}..{} out of bounds (file length: {})",
                edit.range.start,
                edit.range.end,
                result.len()
            );
        }

        if edit.range.start > edit.range.end {
            anyhow::bail!(
                "Invalid edit range: start {} > end {}",
                edit.range.start,
                edit.range.end
            );
        }

        result.replace_range(edit.range.start..edit.range.end, &edit.new_text);
    }

    Ok(result)
}

/// Generate a unique key for a diagnostic (for deduplication/skipping).
fn diagnostic_key(diag: &Diagnostic) -> String {
    format!(
        "{}:{}:{}:{}",
        diag.location.file.display(),
        diag.location.start_line,
        diag.location.start_col,
        diag.rule_id
    )
}

/// Helper to create a fix from line-based replacement.
pub fn line_replacement_fix(
    file: &Path,
    line_num: u32,
    content: &str,
    old_pattern: &str,
    new_text: &str,
    description: &str,
) -> Option<Fix> {
    // Find the line
    let lines: Vec<&str> = content.lines().collect();
    let line_idx = (line_num - 1) as usize;

    if line_idx >= lines.len() {
        return None;
    }

    let line = lines[line_idx];

    // Find pattern in line
    let col = line.find(old_pattern)?;

    // Calculate byte offset
    let line_start: usize = lines[..line_idx]
        .iter()
        .map(|l| l.len() + 1) // +1 for newline
        .sum();

    let start = line_start + col;
    let end = start + old_pattern.len();

    Some(Fix::simple(
        description,
        file.to_path_buf(),
        Range::new(start, end),
        new_text,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_edits() {
        let content = "hello world";
        let edit = TextEdit {
            file: PathBuf::from("test.txt"),
            range: Range::new(6, 11),
            new_text: "rust".to_string(),
        };
        let edits = vec![&edit];

        let result = apply_edits(content, &edits).unwrap();
        assert_eq!(result, "hello rust");
    }

    #[test]
    fn test_apply_multiple_edits() {
        let content = "aaa bbb ccc";
        let edit1 = TextEdit {
            file: PathBuf::from("test.txt"),
            range: Range::new(0, 3),
            new_text: "AAA".to_string(),
        };
        let edit2 = TextEdit {
            file: PathBuf::from("test.txt"),
            range: Range::new(8, 11),
            new_text: "CCC".to_string(),
        };
        let edits = vec![&edit1, &edit2];

        let result = apply_edits(content, &edits).unwrap();
        assert_eq!(result, "AAA bbb CCC");
    }

    #[test]
    fn test_apply_edits_out_of_bounds() {
        let content = "hello";
        let edit = TextEdit {
            file: PathBuf::from("test.txt"),
            range: Range::new(10, 15),
            new_text: "x".to_string(),
        };
        let edits = vec![&edit];

        let result = apply_edits(content, &edits);
        assert!(result.is_err());
    }

    #[test]
    fn test_line_replacement_fix() {
        let content = "line 1\nlet x = foo.unwrap();\nline 3";
        let fix = line_replacement_fix(
            Path::new("test.rs"),
            2,
            content,
            ".unwrap()",
            "?",
            "Replace unwrap with ?",
        );

        assert!(fix.is_some());
        let fix = fix.unwrap();
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].new_text, "?");
    }
}
