//! ADI Linter Core - Language-agnostic linting system.
//!
//! This crate provides the core linting infrastructure for the ADI ecosystem:
//!
//! - **Multiple linter types**: External (subprocess), Plugin (service registry), Command (inline)
//! - **Category-based organization**: Security, Architecture, Code Quality, etc.
//! - **Priority-based execution**: Higher priority linters run first
//! - **Parallel execution**: Linters within same priority level run concurrently
//! - **Autofix support**: Sequential fix application with full re-linting
//!
//! # Example
//!
//! ```rust,no_run
//! use adi_linter_core::{
//!     config::LinterConfig,
//!     runner::Runner,
//!     output::{format_to_stdout, OutputFormat},
//! };
//! use std::path::Path;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Load configuration
//! let config = LinterConfig::load_from_project(Path::new("."))?;
//!
//! // Build registry and runner
//! let registry = config.build_registry()?;
//! let runner_config = config.runner_config(Path::new("."));
//! let runner = Runner::new(registry, runner_config);
//!
//! // Run linting
//! let result = runner.run(None).await?;
//!
//! // Output results
//! format_to_stdout(&result, OutputFormat::Pretty)?;
//! # Ok(())
//! # }
//! ```

pub mod autofix;
pub mod config;
pub mod files;
pub mod linter;
pub mod output;
pub mod registry;
pub mod runner;
pub mod types;

// Re-exports for convenience
pub use autofix::{AutofixConfig, AutofixEngine, AutofixResult};
pub use config::LinterConfig;
pub use files::{FileIterator, FileIteratorBuilder};
pub use linter::{LintContext, Linter};
pub use output::{format_to_stdout, format_to_string, OutputFormat};
pub use registry::{CategoryConfig, LinterRegistry, LinterRegistryBuilder};
pub use runner::{LintResult, Runner, RunnerConfig};
pub use types::{Category, Diagnostic, Fix, Location, Range, Severity, TextEdit};

/// Run linting with default configuration.
///
/// This is a convenience function for simple use cases.
pub async fn lint(root: &std::path::Path) -> anyhow::Result<LintResult> {
    let config = LinterConfig::load_from_project(root)?;
    let registry = config.build_registry()?;
    let runner_config = config.runner_config(root);
    let runner = Runner::new(registry, runner_config);
    runner.run(None).await
}

/// Run linting with autofix.
///
/// This is a convenience function for simple use cases.
pub async fn lint_and_fix(root: &std::path::Path) -> anyhow::Result<autofix::AutofixResult> {
    let config = LinterConfig::load_from_project(root)?;
    let registry = config.build_registry()?;
    let runner_config = config.runner_config(root);
    let runner = Runner::new(registry, runner_config);
    let autofix_config = config.autofix_config();
    let engine = AutofixEngine::new(&runner, autofix_config);
    engine.run(None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_integration() {
        let dir = TempDir::new().unwrap();

        // Create a test file
        fs::write(dir.path().join("test.rs"), "// TODO: fix this").unwrap();

        // Create linters directory and config
        let linters_dir = dir.path().join(".adi").join("linters");
        fs::create_dir_all(&linters_dir).unwrap();

        // Write global config
        let global_config = r#"
[linter]
parallel = true

[categories]
code-quality = { enabled = true }
"#;
        fs::write(linters_dir.join("config.toml"), global_config).unwrap();

        // Write rule file
        let rule_content = r#"
[rule]
id = "no-todo"
type = "command"
category = "code-quality"
severity = "warning"

[rule.command]
type = "regex-forbid"
pattern = "TODO"
message = "Found TODO"

[rule.glob]
patterns = ["**/*.rs"]
"#;
        fs::write(linters_dir.join("no-todo.toml"), rule_content).unwrap();

        // Run linting
        let result = lint(dir.path()).await.unwrap();

        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].message, "Found TODO");
    }
}
