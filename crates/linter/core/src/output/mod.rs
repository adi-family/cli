//! Output formatters for lint results.

pub mod json;
pub mod pretty;

use crate::runner::LintResult;
use std::io::Write;

/// Output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable terminal output.
    #[default]
    Pretty,
    /// JSON output.
    Json,
    /// SARIF format (for IDE integration).
    Sarif,
}

/// Trait for output formatters.
pub trait Formatter {
    /// Format the lint result to the given writer.
    fn format<W: Write>(&self, result: &LintResult, writer: &mut W) -> anyhow::Result<()>;
}

/// Format a lint result to stdout.
pub fn format_to_stdout(result: &LintResult, format: OutputFormat) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout();
    match format {
        OutputFormat::Pretty => pretty::PrettyFormatter::default().format(result, &mut stdout),
        OutputFormat::Json => json::JsonFormatter::default().format(result, &mut stdout),
        OutputFormat::Sarif => {
            // SARIF not implemented yet - fall back to JSON
            json::JsonFormatter::default().format(result, &mut stdout)
        }
    }
}

/// Format a lint result to a string.
pub fn format_to_string(result: &LintResult, format: OutputFormat) -> anyhow::Result<String> {
    let mut buffer = Vec::new();
    match format {
        OutputFormat::Pretty => pretty::PrettyFormatter::default().format(result, &mut buffer)?,
        OutputFormat::Json => json::JsonFormatter::default().format(result, &mut buffer)?,
        OutputFormat::Sarif => json::JsonFormatter::default().format(result, &mut buffer)?,
    }
    Ok(String::from_utf8(buffer)?)
}
