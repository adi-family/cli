//! Lint runner - orchestrates parallel linting execution.

use crate::files::FileIterator;
use crate::linter::{LintContext, Linter};
use crate::registry::LinterRegistry;
use crate::types::{Diagnostic, LintScope, Severity};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Configuration for the lint runner.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Root directory to lint.
    pub root: PathBuf,
    /// Run linters in parallel.
    pub parallel: bool,
    /// Maximum number of concurrent linters.
    pub max_workers: usize,
    /// Stop on first error.
    pub fail_fast: bool,
    /// Timeout per linter (per file).
    pub timeout: Duration,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            parallel: true,
            max_workers: num_cpus::get(),
            fail_fast: false,
            timeout: Duration::from_secs(30),
        }
    }
}

impl RunnerConfig {
    /// Create a new config with the given root.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            ..Default::default()
        }
    }

    /// Set parallel execution.
    pub fn parallel(mut self, enabled: bool) -> Self {
        self.parallel = enabled;
        self
    }

    /// Set max workers.
    pub fn max_workers(mut self, workers: usize) -> Self {
        self.max_workers = workers;
        self
    }

    /// Set fail fast mode.
    pub fn fail_fast(mut self, enabled: bool) -> Self {
        self.fail_fast = enabled;
        self
    }

    /// Set timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Result of a lint run.
#[derive(Debug)]
pub struct LintResult {
    /// All diagnostics found.
    pub diagnostics: Vec<Diagnostic>,
    /// Number of files checked.
    pub files_checked: usize,
    /// Total duration.
    pub duration: Duration,
    /// Errors that occurred during linting (not lint issues).
    pub errors: Vec<LintError>,
    /// Per-category summary.
    pub by_category: HashMap<String, CategorySummary>,
    /// Per-severity summary.
    pub by_severity: HashMap<Severity, usize>,
}

impl LintResult {
    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Check if there are any warnings or errors.
    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity >= Severity::Warning)
    }

    /// Get count of fixable diagnostics.
    pub fn fixable_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_fixable()).count()
    }

    /// Get diagnostics by severity.
    pub fn filter_severity(&self, severity: Severity) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .collect()
    }

    /// Get diagnostics for a specific file.
    pub fn for_file(&self, path: &Path) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.location.file == path)
            .collect()
    }
}

/// Summary for a category.
#[derive(Debug, Default, Clone)]
pub struct CategorySummary {
    /// Total diagnostics.
    pub total: usize,
    /// By severity.
    pub by_severity: HashMap<Severity, usize>,
    /// Fixable count.
    pub fixable: usize,
}

/// Error that occurred during linting.
#[derive(Debug)]
pub struct LintError {
    /// Linter ID.
    pub linter_id: String,
    /// File being linted (if applicable).
    pub file: Option<PathBuf>,
    /// Error message.
    pub message: String,
}

/// Lint runner.
pub struct Runner {
    registry: Arc<LinterRegistry>,
    config: RunnerConfig,
}

impl Runner {
    /// Create a new runner.
    pub fn new(registry: LinterRegistry, config: RunnerConfig) -> Self {
        Self {
            registry: Arc::new(registry),
            config,
        }
    }

    /// Run linting on the configured root or specific files.
    pub async fn run(&self, files: Option<Vec<PathBuf>>) -> anyhow::Result<LintResult> {
        let start = Instant::now();

        // Collect files
        let files = match files {
            Some(f) => f,
            None => self.collect_files(),
        };

        let files_checked = files.len();
        let mut all_diagnostics = Vec::new();
        let mut all_errors = Vec::new();

        // Group linters by priority (descending)
        let priority_groups = self.registry.by_priority_groups();

        // Execute by priority level (sequential between levels, parallel within)
        for (_priority, linters) in priority_groups.into_iter().rev() {
            let (diags, errors) = self.run_priority_group(&linters, &files).await?;
            all_diagnostics.extend(diags);
            all_errors.extend(errors);

            if self.config.fail_fast && has_errors(&all_diagnostics) {
                break;
            }
        }

        // Deduplicate diagnostics
        all_diagnostics = deduplicate_diagnostics(all_diagnostics);

        // Build summaries
        let by_category = build_category_summary(&all_diagnostics);
        let by_severity = build_severity_summary(&all_diagnostics);

        Ok(LintResult {
            diagnostics: all_diagnostics,
            files_checked,
            duration: start.elapsed(),
            errors: all_errors,
            by_category,
            by_severity,
        })
    }

    /// Collect files to lint.
    fn collect_files(&self) -> Vec<PathBuf> {
        FileIterator::new(&self.config.root)
            .use_gitignore(true)
            .use_adiignore(true)
            .collect()
    }

    /// Run linters within a priority group.
    async fn run_priority_group(
        &self,
        linters: &[Arc<dyn Linter>],
        files: &[PathBuf],
    ) -> anyhow::Result<(Vec<Diagnostic>, Vec<LintError>)> {
        if self.config.parallel {
            self.run_parallel(linters, files).await
        } else {
            self.run_sequential(linters, files).await
        }
    }

    /// Run linters in parallel.
    async fn run_parallel(
        &self,
        linters: &[Arc<dyn Linter>],
        files: &[PathBuf],
    ) -> anyhow::Result<(Vec<Diagnostic>, Vec<LintError>)> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_workers));
        let mut handles = Vec::new();

        for linter in linters {
            let linter = Arc::clone(linter);
            let files = files.to_vec();
            let sem = Arc::clone(&semaphore);
            let timeout = self.config.timeout;

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                run_linter_on_files(&linter, &files, timeout).await
            });

            handles.push(handle);
        }

        let mut all_diagnostics = Vec::new();
        let mut all_errors = Vec::new();

        for handle in handles {
            let (diags, errors) = handle.await??;
            all_diagnostics.extend(diags);
            all_errors.extend(errors);
        }

        Ok((all_diagnostics, all_errors))
    }

    /// Run linters sequentially.
    async fn run_sequential(
        &self,
        linters: &[Arc<dyn Linter>],
        files: &[PathBuf],
    ) -> anyhow::Result<(Vec<Diagnostic>, Vec<LintError>)> {
        let mut all_diagnostics = Vec::new();
        let mut all_errors = Vec::new();

        for linter in linters {
            let (diags, errors) = run_linter_on_files(linter, files, self.config.timeout).await?;
            all_diagnostics.extend(diags);
            all_errors.extend(errors);
        }

        Ok((all_diagnostics, all_errors))
    }

    /// Get the registry.
    pub fn registry(&self) -> &LinterRegistry {
        &self.registry
    }
}

/// Run a single linter on a set of files.
async fn run_linter_on_files(
    linter: &Arc<dyn Linter>,
    files: &[PathBuf],
    timeout: Duration,
) -> anyhow::Result<(Vec<Diagnostic>, Vec<LintError>)> {
    let mut diagnostics = Vec::new();
    let mut errors = Vec::new();

    // Filter to matching files
    let matching: Vec<_> = files.iter().filter(|f| linter.matches(f)).collect();

    for file in matching {
        let content = match tokio::fs::read_to_string(file).await {
            Ok(c) => c,
            Err(e) => {
                errors.push(LintError {
                    linter_id: linter.id().to_string(),
                    file: Some(file.clone()),
                    message: format!("Failed to read file: {}", e),
                });
                continue;
            }
        };

        match linter.scope() {
            LintScope::File => {
                let ctx = LintContext::file(file.clone(), content);
                match tokio::time::timeout(timeout, linter.lint(&ctx)).await {
                    Ok(Ok(diags)) => diagnostics.extend(diags),
                    Ok(Err(e)) => {
                        errors.push(LintError {
                            linter_id: linter.id().to_string(),
                            file: Some(file.clone()),
                            message: format!("Linter error: {}", e),
                        });
                    }
                    Err(_) => {
                        errors.push(LintError {
                            linter_id: linter.id().to_string(),
                            file: Some(file.clone()),
                            message: "Linter timed out".to_string(),
                        });
                    }
                }
            }
            LintScope::Line => {
                for (line_idx, line_content) in content.lines().enumerate() {
                    let line_num = line_idx as u32 + 1;
                    let ctx =
                        LintContext::line(file.clone(), content.clone(), line_num, line_content);

                    match tokio::time::timeout(timeout, linter.lint(&ctx)).await {
                        Ok(Ok(diags)) => diagnostics.extend(diags),
                        Ok(Err(e)) => {
                            errors.push(LintError {
                                linter_id: linter.id().to_string(),
                                file: Some(file.clone()),
                                message: format!("Linter error at line {}: {}", line_num, e),
                            });
                        }
                        Err(_) => {
                            errors.push(LintError {
                                linter_id: linter.id().to_string(),
                                file: Some(file.clone()),
                                message: format!("Linter timed out at line {}", line_num),
                            });
                        }
                    }
                }
            }
            LintScope::Symbol => {
                // Symbol scope requires indexer integration - skip for now
            }
        }
    }

    Ok((diagnostics, errors))
}

fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

fn deduplicate_diagnostics(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    // Sort by file, line, col, rule_id for stable deduplication
    diagnostics.sort_by(|a, b| {
        a.location
            .file
            .cmp(&b.location.file)
            .then_with(|| a.location.start_line.cmp(&b.location.start_line))
            .then_with(|| a.location.start_col.cmp(&b.location.start_col))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });

    // Remove duplicates (same file, line, col, rule)
    diagnostics.dedup_by(|a, b| {
        a.location.file == b.location.file
            && a.location.start_line == b.location.start_line
            && a.location.start_col == b.location.start_col
            && a.rule_id == b.rule_id
    });

    diagnostics
}

fn build_category_summary(diagnostics: &[Diagnostic]) -> HashMap<String, CategorySummary> {
    let mut summary: HashMap<String, CategorySummary> = HashMap::new();

    for diag in diagnostics {
        // Count in each category for multi-category diagnostics
        for category in &diag.categories {
            let category_name = category.display_name().to_string();
            let entry = summary.entry(category_name).or_default();

            entry.total += 1;
            *entry.by_severity.entry(diag.severity).or_default() += 1;
            if diag.is_fixable() {
                entry.fixable += 1;
            }
        }
    }

    summary
}

fn build_severity_summary(diagnostics: &[Diagnostic]) -> HashMap<Severity, usize> {
    let mut summary = HashMap::new();

    for diag in diagnostics {
        *summary.entry(diag.severity).or_default() += 1;
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::command::{CommandLinter, CommandType};
    use crate::types::Category;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_linter() -> CommandLinter {
        CommandLinter::new(
            "test-linter",
            Category::CodeQuality,
            vec!["**/*.rs".to_string()],
            CommandType::RegexForbid {
                pattern: "TODO".to_string(),
                message: "Found TODO".to_string(),
                fix: None,
            },
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_runner_basic() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.rs"),
            "fn main() {\n    // TODO: fix\n}",
        )
        .unwrap();

        let mut registry = LinterRegistry::new();
        registry.register(create_test_linter());

        let config = RunnerConfig::new(dir.path());
        let runner = Runner::new(registry, config);

        let result = runner.run(None).await.unwrap();

        assert_eq!(result.files_checked, 1);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].message, "Found TODO");
    }

    #[tokio::test]
    async fn test_runner_parallel() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.rs"), "// TODO").unwrap();
        fs::write(dir.path().join("b.rs"), "// TODO").unwrap();

        let mut registry = LinterRegistry::new();
        registry.register(create_test_linter());

        let config = RunnerConfig::new(dir.path()).parallel(true).max_workers(2);
        let runner = Runner::new(registry, config);

        let result = runner.run(None).await.unwrap();

        assert_eq!(result.files_checked, 2);
        assert_eq!(result.diagnostics.len(), 2);
    }

    #[test]
    fn test_deduplication() {
        use crate::types::Location;

        let diags = vec![
            Diagnostic::new(
                "rule1",
                "linter1",
                Category::Security,
                Severity::Error,
                "msg1",
                Location::new(PathBuf::from("a.rs"), 1, 1, 1, 10),
            ),
            Diagnostic::new(
                "rule1",
                "linter1",
                Category::Security,
                Severity::Error,
                "msg1",
                Location::new(PathBuf::from("a.rs"), 1, 1, 1, 10),
            ),
            Diagnostic::new(
                "rule2",
                "linter1",
                Category::Security,
                Severity::Error,
                "msg2",
                Location::new(PathBuf::from("a.rs"), 1, 1, 1, 10),
            ),
        ];

        let deduped = deduplicate_diagnostics(diags);
        assert_eq!(deduped.len(), 2); // rule1 + rule2
    }
}
