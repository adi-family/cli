//! ADI Linter Plugin
//!
//! Code linting with configurable rules and auto-fix support.

use lib_plugin_prelude::*;
use linter_core::{format_to_string, LinterConfig, OutputFormat};

pub struct LinterPlugin;

#[async_trait]
impl Plugin for LinterPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.linter", "ADI Linter", env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Core)
            .with_author("ADI Team")
            .with_description("Code linting with configurable rules and auto-fix support")
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS]
    }
}

#[async_trait]
impl CliCommands for LinterPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "run".to_string(),
                description: "Run linting on files".to_string(),
                args: vec![CliArg::optional("--format", CliArgType::String)],
                has_subcommands: false,
            },
            CliCommand {
                name: "fix".to_string(),
                description: "Apply auto-fixes".to_string(),
                args: vec![],
                has_subcommands: false,
            },
            CliCommand {
                name: "list".to_string(),
                description: "List configured linters".to_string(),
                args: vec![CliArg::optional("--format", CliArgType::String)],
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("run") => cmd_run(ctx).await,
            Some("fix") => cmd_fix(ctx).await,
            Some("list") => cmd_list(ctx).await,
            Some(cmd) => Ok(CliResult::error(format!("Unknown command: {}", cmd))),
            None => Ok(CliResult::success(help())),
        }
    }
}

fn help() -> String {
    "ADI Linter - Code linting with configurable rules\n\n\
     Commands:\n  \
     run   Run linting on files\n  \
     fix   Apply auto-fixes\n  \
     list  List configured linters\n\n\
     Usage: lint <command> [options]"
        .to_string()
}

async fn cmd_run(ctx: &CliContext) -> Result<CliResult> {
    let format = match ctx.option::<String>("format").as_deref() {
        Some("json") => OutputFormat::Json,
        _ => OutputFormat::Pretty,
    };

    let result = linter_core::lint(&ctx.cwd)
        .await
        .map_err(|e| PluginError::CommandFailed(e.to_string()))?;

    let output = format_to_string(&result, format)
        .map_err(|e| PluginError::CommandFailed(e.to_string()))?;

    if result.has_errors() {
        Ok(CliResult::custom(1, output, String::new()))
    } else {
        Ok(CliResult::success(output))
    }
}

async fn cmd_fix(ctx: &CliContext) -> Result<CliResult> {
    let result = linter_core::lint_and_fix(&ctx.cwd)
        .await
        .map_err(|e| PluginError::CommandFailed(e.to_string()))?;

    let mut output = format!("Applied {} fix(es).", result.fixes_count());
    if result.remaining_count() > 0 {
        output.push_str(&format!("\n{} issue(s) remaining.", result.remaining_count()));
    } else {
        output.push_str("\nAll issues resolved.");
    }

    Ok(CliResult::success(output))
}

async fn cmd_list(ctx: &CliContext) -> Result<CliResult> {
    let config = LinterConfig::load_from_project(&ctx.cwd)
        .map_err(|e| PluginError::Config(e.to_string()))?;

    let registry = config
        .build_registry()
        .map_err(|e| PluginError::Config(e.to_string()))?;

    let linters: Vec<_> = registry.all_linters().collect();

    if linters.is_empty() {
        return Ok(CliResult::success(
            "No linters configured. Add rules to .adi/linters/".to_string(),
        ));
    }

    let use_json = ctx.option::<String>("format").as_deref() == Some("json");

    if use_json {
        let entries: Vec<serde_json::Value> = linters
            .iter()
            .map(|l| {
                serde_json::json!({
                    "id": l.id(),
                    "categories": l.categories().iter().map(|c| format!("{:?}", c)).collect::<Vec<_>>(),
                    "patterns": l.patterns(),
                    "priority": l.priority(),
                })
            })
            .collect();

        let output = serde_json::to_string_pretty(&entries)
            .map_err(|e| PluginError::CommandFailed(e.to_string()))?;
        return Ok(CliResult::success(output));
    }

    let mut output = format!("{} linter(s) configured:\n\n", linters.len());
    for linter in &linters {
        let categories: Vec<_> = linter.categories().iter().map(|c| format!("{:?}", c)).collect();
        output.push_str(&format!(
            "  {} [{}]\n    patterns: {}\n",
            linter.id(),
            categories.join(", "),
            linter.patterns().join(", "),
        ));
    }

    Ok(CliResult::success(output.trim_end().to_string()))
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(LinterPlugin)
}
