//! Command linter - inline rules (regex, line length, etc.)
//! Fast, no subprocess spawning.

use super::{LintContext, Linter, LinterConfig};
use crate::types::{Category, Diagnostic, Fix, LintScope, Location, Range, Severity};
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Type of command/check to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CommandType {
    /// Error if regex matches.
    RegexForbid {
        pattern: String,
        message: String,
        #[serde(default)]
        fix: Option<RegexFix>,
    },
    /// Error if regex does NOT match.
    RegexRequire { pattern: String, message: String },
    /// Error if line exceeds max length.
    MaxLineLength { max: usize },
    /// Error if file exceeds max size.
    MaxFileSize { max: usize },
    /// Error if text is found.
    Contains { text: String, message: String },
    /// Error if text is NOT found.
    NotContains { text: String, message: String },
}

/// Regex-based fix configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexFix {
    /// Pattern to match for replacement.
    pub pattern: String,
    /// Replacement text (supports $1, $2 for capture groups).
    pub replacement: String,
}

/// Command linter - inline checks without subprocess.
pub struct CommandLinter {
    config: LinterConfig,
    command: CommandType,
    scope: LintScope,
    severity: Severity,
    compiled_regex: Option<Regex>,
    fix_regex: Option<Regex>,
}

impl CommandLinter {
    /// Create a new command linter with a single category.
    pub fn new(
        id: impl Into<String>,
        category: Category,
        patterns: Vec<String>,
        command: CommandType,
    ) -> anyhow::Result<Self> {
        Self::with_categories(id, vec![category], patterns, command)
    }

    /// Create a new command linter with multiple categories.
    pub fn with_categories(
        id: impl Into<String>,
        categories: Vec<Category>,
        patterns: Vec<String>,
        command: CommandType,
    ) -> anyhow::Result<Self> {
        let config = LinterConfig::with_categories(id, categories, patterns)?;
        let (compiled_regex, fix_regex) = Self::compile_regexes(&command)?;

        Ok(Self {
            config,
            command,
            scope: LintScope::File,
            severity: Severity::Warning,
            compiled_regex,
            fix_regex,
        })
    }

    /// Set lint scope.
    pub fn with_scope(mut self, scope: LintScope) -> Self {
        self.scope = scope;
        self
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.config = self.config.with_priority(priority);
        self
    }

    fn compile_regexes(command: &CommandType) -> anyhow::Result<(Option<Regex>, Option<Regex>)> {
        match command {
            CommandType::RegexForbid { pattern, fix, .. } => {
                let main = Regex::new(pattern)?;
                let fix_rx = fix.as_ref().map(|f| Regex::new(&f.pattern)).transpose()?;
                Ok((Some(main), fix_rx))
            }
            CommandType::RegexRequire { pattern, .. } => Ok((Some(Regex::new(pattern)?), None)),
            _ => Ok((None, None)),
        }
    }

    fn lint_file(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        match &self.command {
            CommandType::RegexForbid { message, fix, .. } => {
                self.check_regex_forbid_file(ctx, message, fix)
            }
            CommandType::RegexRequire { message, .. } => {
                self.check_regex_require_file(ctx, message)
            }
            CommandType::MaxLineLength { max } => self.check_max_line_length(ctx, *max),
            CommandType::MaxFileSize { max } => self.check_max_file_size(ctx, *max),
            CommandType::Contains { text, message } => self.check_contains(ctx, text, message),
            CommandType::NotContains { text, message } => {
                self.check_not_contains(ctx, text, message)
            }
        }
    }

    fn lint_line(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let line_num = ctx.line.unwrap_or(1);
        let line_content = ctx.line_content.as_ref().unwrap_or(&ctx.content);

        match &self.command {
            CommandType::RegexForbid { message, fix, .. } => {
                self.check_regex_forbid_line(ctx, line_num, line_content, message, fix)
            }
            CommandType::RegexRequire { message, .. } => {
                self.check_regex_require_line(ctx, line_num, line_content, message)
            }
            CommandType::MaxLineLength { max } => {
                self.check_line_length(ctx, line_num, line_content, *max)
            }
            CommandType::Contains { text, message } => {
                self.check_line_contains(ctx, line_num, line_content, text, message)
            }
            _ => Vec::new(),
        }
    }

    fn check_regex_forbid_file(
        &self,
        ctx: &LintContext,
        message: &str,
        fix_config: &Option<RegexFix>,
    ) -> Vec<Diagnostic> {
        let regex = match &self.compiled_regex {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        for (line_idx, line) in ctx.content.lines().enumerate() {
            let line_num = line_idx as u32 + 1;
            for mat in regex.find_iter(line) {
                let mut diag = Diagnostic::with_categories(
                    &self.config.id,
                    &self.config.id,
                    self.config.categories.clone(),
                    self.severity,
                    message,
                    Location::new(
                        ctx.file.clone(),
                        line_num,
                        mat.start() as u32 + 1,
                        line_num,
                        mat.end() as u32 + 1,
                    ),
                );

                // Add fix if configured
                if let (Some(fix_cfg), Some(fix_rx)) = (fix_config, &self.fix_regex) {
                    if let Some(fix_match) = fix_rx.find(line) {
                        let new_text = fix_rx.replace(fix_match.as_str(), &fix_cfg.replacement);
                        let line_start = ctx.content[..line_idx]
                            .chars()
                            .map(|c| c.len_utf8())
                            .sum::<usize>()
                            + line_idx; // account for newlines

                        diag = diag.with_fix(Fix::simple(
                            format!("Replace with '{}'", new_text),
                            ctx.file.clone(),
                            Range::new(
                                line_start + fix_match.start(),
                                line_start + fix_match.end(),
                            ),
                            new_text.to_string(),
                        ));
                    }
                }

                diagnostics.push(diag);
            }
        }

        diagnostics
    }

    fn check_regex_forbid_line(
        &self,
        ctx: &LintContext,
        line_num: u32,
        line_content: &str,
        message: &str,
        fix_config: &Option<RegexFix>,
    ) -> Vec<Diagnostic> {
        let regex = match &self.compiled_regex {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        for mat in regex.find_iter(line_content) {
            let mut diag = Diagnostic::with_categories(
                &self.config.id,
                &self.config.id,
                self.config.categories.clone(),
                self.severity,
                message,
                Location::new(
                    ctx.file.clone(),
                    line_num,
                    mat.start() as u32 + 1,
                    line_num,
                    mat.end() as u32 + 1,
                ),
            );

            if let (Some(fix_cfg), Some(fix_rx)) = (fix_config, &self.fix_regex) {
                if let Some(fix_match) = fix_rx.find(line_content) {
                    let new_text = fix_rx.replace(fix_match.as_str(), &fix_cfg.replacement);
                    diag = diag.with_fix(Fix::simple(
                        format!("Replace with '{}'", new_text),
                        ctx.file.clone(),
                        Range::new(fix_match.start(), fix_match.end()),
                        new_text.to_string(),
                    ));
                }
            }

            diagnostics.push(diag);
        }

        diagnostics
    }

    fn check_regex_require_file(&self, ctx: &LintContext, message: &str) -> Vec<Diagnostic> {
        let regex = match &self.compiled_regex {
            Some(r) => r,
            None => return Vec::new(),
        };

        if regex.is_match(&ctx.content) {
            Vec::new()
        } else {
            vec![Diagnostic::with_categories(
                &self.config.id,
                &self.config.id,
                self.config.categories.clone(),
                self.severity,
                message,
                Location::file(ctx.file.clone()),
            )]
        }
    }

    fn check_regex_require_line(
        &self,
        ctx: &LintContext,
        line_num: u32,
        line_content: &str,
        message: &str,
    ) -> Vec<Diagnostic> {
        let regex = match &self.compiled_regex {
            Some(r) => r,
            None => return Vec::new(),
        };

        if regex.is_match(line_content) {
            Vec::new()
        } else {
            vec![Diagnostic::with_categories(
                &self.config.id,
                &self.config.id,
                self.config.categories.clone(),
                self.severity,
                message,
                Location::line(ctx.file.clone(), line_num),
            )]
        }
    }

    fn check_max_line_length(&self, ctx: &LintContext, max: usize) -> Vec<Diagnostic> {
        ctx.content
            .lines()
            .enumerate()
            .filter(|(_, line)| line.len() > max)
            .map(|(idx, line)| {
                Diagnostic::with_categories(
                    &self.config.id,
                    &self.config.id,
                    self.config.categories.clone(),
                    self.severity,
                    format!("Line exceeds {} characters ({} chars)", max, line.len()),
                    Location::new(
                        ctx.file.clone(),
                        idx as u32 + 1,
                        max as u32 + 1,
                        idx as u32 + 1,
                        line.len() as u32 + 1,
                    ),
                )
            })
            .collect()
    }

    fn check_line_length(
        &self,
        ctx: &LintContext,
        line_num: u32,
        line_content: &str,
        max: usize,
    ) -> Vec<Diagnostic> {
        if line_content.len() > max {
            vec![Diagnostic::with_categories(
                &self.config.id,
                &self.config.id,
                self.config.categories.clone(),
                self.severity,
                format!(
                    "Line exceeds {} characters ({} chars)",
                    max,
                    line_content.len()
                ),
                Location::new(
                    ctx.file.clone(),
                    line_num,
                    max as u32 + 1,
                    line_num,
                    line_content.len() as u32 + 1,
                ),
            )]
        } else {
            Vec::new()
        }
    }

    fn check_max_file_size(&self, ctx: &LintContext, max: usize) -> Vec<Diagnostic> {
        if ctx.content.len() > max {
            vec![Diagnostic::with_categories(
                &self.config.id,
                &self.config.id,
                self.config.categories.clone(),
                self.severity,
                format!("File exceeds {} bytes ({} bytes)", max, ctx.content.len()),
                Location::file(ctx.file.clone()),
            )]
        } else {
            Vec::new()
        }
    }

    fn check_contains(&self, ctx: &LintContext, text: &str, message: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (line_idx, line) in ctx.content.lines().enumerate() {
            if let Some(col) = line.find(text) {
                diagnostics.push(Diagnostic::with_categories(
                    &self.config.id,
                    &self.config.id,
                    self.config.categories.clone(),
                    self.severity,
                    message,
                    Location::new(
                        ctx.file.clone(),
                        line_idx as u32 + 1,
                        col as u32 + 1,
                        line_idx as u32 + 1,
                        (col + text.len()) as u32 + 1,
                    ),
                ));
            }
        }

        diagnostics
    }

    fn check_line_contains(
        &self,
        ctx: &LintContext,
        line_num: u32,
        line_content: &str,
        text: &str,
        message: &str,
    ) -> Vec<Diagnostic> {
        if let Some(col) = line_content.find(text) {
            vec![Diagnostic::with_categories(
                &self.config.id,
                &self.config.id,
                self.config.categories.clone(),
                self.severity,
                message,
                Location::new(
                    ctx.file.clone(),
                    line_num,
                    col as u32 + 1,
                    line_num,
                    (col + text.len()) as u32 + 1,
                ),
            )]
        } else {
            Vec::new()
        }
    }

    fn check_not_contains(&self, ctx: &LintContext, text: &str, message: &str) -> Vec<Diagnostic> {
        if ctx.content.contains(text) {
            Vec::new()
        } else {
            vec![Diagnostic::with_categories(
                &self.config.id,
                &self.config.id,
                self.config.categories.clone(),
                self.severity,
                message,
                Location::file(ctx.file.clone()),
            )]
        }
    }
}

#[async_trait]
impl Linter for CommandLinter {
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
        self.scope
    }

    async fn lint(&self, ctx: &LintContext) -> anyhow::Result<Vec<Diagnostic>> {
        let diagnostics = match self.scope {
            LintScope::File => self.lint_file(ctx),
            LintScope::Line => self.lint_line(ctx),
            LintScope::Symbol => Vec::new(), // Not supported for command linters
        };

        Ok(diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_regex_forbid() {
        let linter = CommandLinter::new(
            "no-todo",
            Category::CodeQuality,
            vec!["**/*.rs".to_string()],
            CommandType::RegexForbid {
                pattern: r"TODO|FIXME".to_string(),
                message: "Unresolved TODO".to_string(),
                fix: None,
            },
        )
        .unwrap();

        let ctx = LintContext::file(
            "test.rs",
            "fn main() {\n    // TODO: fix this\n    println!(\"hello\");\n}",
        );

        let diags = linter.lint(&ctx).await.unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.start_line, 2);
    }

    #[tokio::test]
    async fn test_max_line_length() {
        let linter = CommandLinter::new(
            "max-line",
            Category::Style,
            vec!["**/*".to_string()],
            CommandType::MaxLineLength { max: 10 },
        )
        .unwrap();

        let ctx = LintContext::file("test.txt", "short\nthis line is way too long\nok");

        let diags = linter.lint(&ctx).await.unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.start_line, 2);
    }

    #[tokio::test]
    async fn test_regex_with_fix() {
        let linter = CommandLinter::new(
            "no-unwrap",
            Category::ErrorHandling,
            vec!["**/*.rs".to_string()],
            CommandType::RegexForbid {
                pattern: r"\.unwrap\(\)".to_string(),
                message: "Avoid .unwrap()".to_string(),
                fix: Some(RegexFix {
                    pattern: r"\.unwrap\(\)".to_string(),
                    replacement: "?".to_string(),
                }),
            },
        )
        .unwrap();

        let ctx = LintContext::file("test.rs", "let x = foo.unwrap();");

        let diags = linter.lint(&ctx).await.unwrap();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].is_fixable());
    }
}
