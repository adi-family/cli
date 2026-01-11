//! External linter - subprocess execution for any executable.

use super::{LintContext, Linter, LinterConfig};
use crate::types::{Category, Diagnostic, InputMode, LintScope, Location, OutputMode, Severity};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// External linter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalLinterConfig {
    /// Command template with placeholders.
    pub exec: String,
    /// How to pass input to the command.
    #[serde(default)]
    pub input_mode: InputMode,
    /// How to parse output from the command.
    #[serde(default)]
    pub output_mode: OutputMode,
    /// Timeout for command execution.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Default severity for diagnostics.
    #[serde(default)]
    pub severity: Severity,
    /// Fallback message when using exit code mode.
    #[serde(default)]
    pub message: Option<String>,
    /// Optional fix command template.
    #[serde(default)]
    pub fix_exec: Option<String>,
}

fn default_timeout() -> u64 {
    30
}

/// External linter - runs any executable.
pub struct ExternalLinter {
    config: LinterConfig,
    external: ExternalLinterConfig,
}

impl ExternalLinter {
    /// Create a new external linter with a single category.
    pub fn new(
        id: impl Into<String>,
        category: Category,
        patterns: Vec<String>,
        external: ExternalLinterConfig,
    ) -> anyhow::Result<Self> {
        Self::with_categories(id, vec![category], patterns, external)
    }

    /// Create a new external linter with multiple categories.
    pub fn with_categories(
        id: impl Into<String>,
        categories: Vec<Category>,
        patterns: Vec<String>,
        external: ExternalLinterConfig,
    ) -> anyhow::Result<Self> {
        let config = LinterConfig::with_categories(id, categories, patterns)?;
        Ok(Self { config, external })
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.config = self.config.with_priority(priority);
        self
    }

    /// Expand template variables in command.
    fn expand_template(&self, template: &str, ctx: &LintContext) -> String {
        let file_str = ctx.file.to_string_lossy();
        let dir = ctx
            .file
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let basename = ctx
            .file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = ctx
            .file
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        template
            .replace("{file}", &file_str)
            .replace("{dir}", &dir)
            .replace("{basename}", &basename)
            .replace("{ext}", &ext)
            .replace("{line}", &ctx.line.unwrap_or(0).to_string())
    }

    /// Parse command into program and arguments.
    fn parse_command(&self, cmd: &str) -> (String, Vec<String>) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return (String::new(), Vec::new());
        }

        let program = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        (program, args)
    }

    /// Execute command and get output.
    async fn execute(&self, ctx: &LintContext) -> anyhow::Result<CommandOutput> {
        let expanded = self.expand_template(&self.external.exec, ctx);
        let (program, args) = self.parse_command(&expanded);

        if program.is_empty() {
            anyhow::bail!("Empty command");
        }

        let mut cmd = Command::new(&program);
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Handle stdin input mode
        if matches!(self.external.input_mode, InputMode::Stdin | InputMode::Both) {
            cmd.stdin(Stdio::piped());
        }

        let mut child = cmd.spawn()?;

        // Write content to stdin if needed
        if matches!(self.external.input_mode, InputMode::Stdin | InputMode::Both) {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(ctx.content.as_bytes()).await?;
            }
        }

        // Wait with timeout
        let timeout = Duration::from_secs(self.external.timeout_secs);
        let output = tokio::time::timeout(timeout, child.wait_with_output()).await??;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Parse output into diagnostics.
    fn parse_output(&self, ctx: &LintContext, output: CommandOutput) -> Vec<Diagnostic> {
        match self.external.output_mode {
            OutputMode::Json => self.parse_json_output(ctx, &output.stdout),
            OutputMode::ExitCode => self.parse_exit_code(ctx, output.exit_code),
            OutputMode::Lines => self.parse_lines_output(ctx, &output.stdout),
        }
    }

    fn parse_json_output(&self, ctx: &LintContext, stdout: &str) -> Vec<Diagnostic> {
        // Try to parse as array of diagnostics or single diagnostic
        if let Ok(diags) = serde_json::from_str::<Vec<ExternalDiagnostic>>(stdout) {
            return diags
                .into_iter()
                .map(|d| self.convert_external_diagnostic(ctx, d))
                .collect();
        }

        if let Ok(diag) = serde_json::from_str::<ExternalDiagnostic>(stdout) {
            return vec![self.convert_external_diagnostic(ctx, diag)];
        }

        // Try to parse as wrapper object with diagnostics field
        if let Ok(wrapper) = serde_json::from_str::<DiagnosticsWrapper>(stdout) {
            return wrapper
                .diagnostics
                .into_iter()
                .map(|d| self.convert_external_diagnostic(ctx, d))
                .collect();
        }

        Vec::new()
    }

    fn convert_external_diagnostic(
        &self,
        ctx: &LintContext,
        ext: ExternalDiagnostic,
    ) -> Diagnostic {
        Diagnostic::with_categories(
            ext.rule_id.unwrap_or_else(|| self.config.id.clone()),
            &self.config.id,
            self.config.categories.clone(),
            ext.severity.unwrap_or(self.external.severity),
            ext.message,
            Location::new(
                ext.file.unwrap_or_else(|| ctx.file.clone()),
                ext.line.unwrap_or(1),
                ext.column.unwrap_or(1),
                ext.end_line.unwrap_or(ext.line.unwrap_or(1)),
                ext.end_column.unwrap_or(ext.column.unwrap_or(1)),
            ),
        )
    }

    fn parse_exit_code(&self, ctx: &LintContext, code: i32) -> Vec<Diagnostic> {
        if code == 0 {
            Vec::new()
        } else {
            vec![Diagnostic::with_categories(
                &self.config.id,
                &self.config.id,
                self.config.categories.clone(),
                self.external.severity,
                self.external
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("Check failed (exit code {})", code)),
                Location::file(ctx.file.clone()),
            )]
        }
    }

    fn parse_lines_output(&self, ctx: &LintContext, stdout: &str) -> Vec<Diagnostic> {
        stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| self.parse_line_format(ctx, line))
            .collect()
    }

    /// Parse line format: "line:col:message" or "line:message"
    fn parse_line_format(&self, ctx: &LintContext, line: &str) -> Option<Diagnostic> {
        let parts: Vec<&str> = line.splitn(3, ':').collect();

        match parts.len() {
            3 => {
                // line:col:message
                let line_num = parts[0].trim().parse().ok()?;
                let col = parts[1].trim().parse().ok()?;
                let message = parts[2].trim();

                Some(Diagnostic::with_categories(
                    &self.config.id,
                    &self.config.id,
                    self.config.categories.clone(),
                    self.external.severity,
                    message,
                    Location::new(ctx.file.clone(), line_num, col, line_num, col),
                ))
            }
            2 => {
                // line:message
                let line_num = parts[0].trim().parse().ok()?;
                let message = parts[1].trim();

                Some(Diagnostic::with_categories(
                    &self.config.id,
                    &self.config.id,
                    self.config.categories.clone(),
                    self.external.severity,
                    message,
                    Location::line(ctx.file.clone(), line_num),
                ))
            }
            _ => {
                // Just a message
                Some(Diagnostic::with_categories(
                    &self.config.id,
                    &self.config.id,
                    self.config.categories.clone(),
                    self.external.severity,
                    line.trim(),
                    Location::file(ctx.file.clone()),
                ))
            }
        }
    }
}

#[async_trait]
impl Linter for ExternalLinter {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn categories(&self) -> &[Category] {
        &self.config.categories
    }

    fn priority(&self) -> u32 {
        self.config.effective_priority()
    }

    fn patterns(&self) -> &[String] {
        &self.config.patterns
    }

    fn matches(&self, path: &Path) -> bool {
        self.config.matches(path)
    }

    fn scope(&self) -> LintScope {
        LintScope::File
    }

    async fn lint(&self, ctx: &LintContext) -> anyhow::Result<Vec<Diagnostic>> {
        let output = self.execute(ctx).await?;
        Ok(self.parse_output(ctx, output))
    }
}

/// Output from command execution.
struct CommandOutput {
    stdout: String,
    #[allow(dead_code)]
    stderr: String,
    exit_code: i32,
}

/// External diagnostic format (from JSON output).
#[derive(Debug, Deserialize)]
struct ExternalDiagnostic {
    #[serde(default)]
    rule_id: Option<String>,
    message: String,
    #[serde(default)]
    severity: Option<Severity>,
    #[serde(default)]
    file: Option<std::path::PathBuf>,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    column: Option<u32>,
    #[serde(default)]
    end_line: Option<u32>,
    #[serde(default)]
    end_column: Option<u32>,
}

/// Wrapper for diagnostics array.
#[derive(Debug, Deserialize)]
struct DiagnosticsWrapper {
    diagnostics: Vec<ExternalDiagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_expansion() {
        let linter = ExternalLinter::new(
            "test",
            Category::Security,
            vec!["**/*".to_string()],
            ExternalLinterConfig {
                exec: "check {file} --ext {ext}".to_string(),
                input_mode: InputMode::FilePath,
                output_mode: OutputMode::ExitCode,
                timeout_secs: 30,
                severity: Severity::Error,
                message: None,
                fix_exec: None,
            },
        )
        .unwrap();

        let ctx = LintContext::file("/path/to/file.rs", "content");
        let expanded = linter.expand_template(&linter.external.exec, &ctx);

        assert!(expanded.contains("/path/to/file.rs"));
        assert!(expanded.contains("--ext rs"));
    }

    #[test]
    fn test_parse_line_format() {
        let linter = ExternalLinter::new(
            "test",
            Category::CodeQuality,
            vec!["**/*".to_string()],
            ExternalLinterConfig {
                exec: "check".to_string(),
                input_mode: InputMode::FilePath,
                output_mode: OutputMode::Lines,
                timeout_secs: 30,
                severity: Severity::Warning,
                message: None,
                fix_exec: None,
            },
        )
        .unwrap();

        let ctx = LintContext::file("test.rs", "");

        // Test line:col:message format
        let diag = linter.parse_line_format(&ctx, "42:10:Some error").unwrap();
        assert_eq!(diag.location.start_line, 42);
        assert_eq!(diag.location.start_col, 10);
        assert_eq!(diag.message, "Some error");

        // Test line:message format
        let diag = linter.parse_line_format(&ctx, "42:Some error").unwrap();
        assert_eq!(diag.location.start_line, 42);
        assert_eq!(diag.message, "Some error");
    }

    #[test]
    fn test_json_parsing() {
        let linter = ExternalLinter::new(
            "test",
            Category::Security,
            vec!["**/*".to_string()],
            ExternalLinterConfig {
                exec: "check".to_string(),
                input_mode: InputMode::FilePath,
                output_mode: OutputMode::Json,
                timeout_secs: 30,
                severity: Severity::Error,
                message: None,
                fix_exec: None,
            },
        )
        .unwrap();

        let ctx = LintContext::file("test.rs", "");
        let json = r#"[{"message": "Test error", "line": 10, "column": 5}]"#;

        let diags = linter.parse_json_output(&ctx, json);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "Test error");
        assert_eq!(diags[0].location.start_line, 10);
    }
}
