//! CLI command handling

use crate::discovery::{discover_workflows, find_workflow};
use crate::executor::execute_steps;
use crate::parser::{load_workflow, WorkflowScope};
use crate::prompts::collect_inputs;
use lib_console_output::{debug, info, is_interactive, success, Select, SelectOption};
use serde_json::json;
use std::path::PathBuf;

/// Run the CLI command
pub fn run_command(context_json: &str) -> Result<String, String> {
    let context: serde_json::Value =
        serde_json::from_str(context_json).map_err(|e| format!("Invalid context: {}", e))?;

    // Get current working directory
    let cwd: PathBuf = context
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Parse command and args from context
    let args: Vec<String> = context
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("");
    let cmd_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();

    match subcommand {
        "list" => cmd_list(&cwd),
        "show" => cmd_show(&cwd, &cmd_args),
        "--completions" => cmd_completions(&cwd, &cmd_args),
        "" => {
            // No subcommand - show interactive workflow selector
            cmd_select_and_run(&cwd)
        }
        workflow_name => {
            // Run a workflow by name
            cmd_run(&cwd, workflow_name)
        }
    }
}

fn help_text() -> String {
    r#"ADI Workflow - Run workflows defined in TOML files

Commands:
  <name>     Run a workflow by name
  list       List available workflows
  show       Show workflow definition

Workflow locations:
  ./.adi/workflows/<name>.toml  (local, highest priority)
  ~/.adi/workflows/<name>.toml  (global)

Usage:
  adi workflow deploy          # Run the 'deploy' workflow
  adi workflow list            # List available workflows
  adi workflow show deploy     # Show 'deploy' workflow details"#
        .to_string()
}

fn cmd_list(cwd: &PathBuf) -> Result<String, String> {
    let workflows = discover_workflows(cwd);

    if workflows.is_empty() {
        return Ok("No workflows found.\n\nCreate workflows at:\n  ./.adi/workflows/<name>.toml  (local)\n  ~/.adi/workflows/<name>.toml  (global)".to_string());
    }

    let mut output = String::from("Available workflows:\n\n");

    for workflow in workflows {
        let scope_indicator = match workflow.scope {
            WorkflowScope::Local => "[local]",
            WorkflowScope::Global => "[global]",
        };

        output.push_str(&format!("  {} {}\n", workflow.name, scope_indicator));

        if let Some(desc) = &workflow.description {
            output.push_str(&format!("    {}\n", desc));
        }
    }

    Ok(output.trim_end().to_string())
}

/// Interactive workflow selector - shown when `adi workflow` is called without arguments
fn cmd_select_and_run(cwd: &PathBuf) -> Result<String, String> {
    let workflows = discover_workflows(cwd);

    if workflows.is_empty() {
        return Ok("No workflows found.\n\nCreate workflows at:\n  ./.adi/workflows/<name>.toml  (local)\n  ~/.adi/workflows/<name>.toml  (global)".to_string());
    }

    // Check if we're in an interactive terminal
    if !is_interactive() {
        // Non-interactive: show help text
        return Ok(help_text());
    }

    // Build selection options with descriptions
    let options: Vec<SelectOption<String>> = workflows
        .iter()
        .map(|w| {
            let scope = match w.scope {
                WorkflowScope::Local => "[local]",
                WorkflowScope::Global => "[global]",
            };
            let label = match &w.description {
                Some(desc) => format!("{} {} - {}", w.name, scope, desc),
                None => format!("{} {}", w.name, scope),
            };
            SelectOption::new(label, w.name.clone())
        })
        .collect();

    info("Select a workflow to run:");

    let selection = Select::new("Workflow").options(options).default(0).run();

    match selection {
        Some(workflow_name) => cmd_run(cwd, &workflow_name),
        None => Err("Selection cancelled".to_string()),
    }
}

fn cmd_show(cwd: &PathBuf, args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("Missing workflow name. Usage: show <name>".to_string());
    }

    let name = args[0];
    let path = find_workflow(cwd, name).ok_or_else(|| format!("Workflow '{}' not found", name))?;

    let workflow = load_workflow(&path)?;

    let mut output = format!("Workflow: {}\n", workflow.workflow.name);

    if let Some(desc) = &workflow.workflow.description {
        output.push_str(&format!("Description: {}\n", desc));
    }

    output.push_str(&format!("Path: {}\n", path.display()));

    if !workflow.inputs.is_empty() {
        output.push_str("\nInputs:\n");
        for input in &workflow.inputs {
            output.push_str(&format!(
                "  {} ({:?}): {}\n",
                input.name, input.input_type, input.prompt
            ));

            if let Some(options) = &input.options {
                output.push_str(&format!("    Options: {}\n", options.join(", ")));
            }

            if let Some(default) = &input.default {
                output.push_str(&format!("    Default: {}\n", default));
            }
        }
    }

    if !workflow.steps.is_empty() {
        output.push_str("\nSteps:\n");
        for (i, step) in workflow.steps.iter().enumerate() {
            output.push_str(&format!("  {}. {}\n", i + 1, step.name));

            if let Some(condition) = &step.condition {
                output.push_str(&format!("     if: {}\n", condition));
            }

            // Show first line of command
            let first_line = step.run.lines().next().unwrap_or(&step.run);
            output.push_str(&format!("     run: {}\n", first_line));
        }
    }

    Ok(output.trim_end().to_string())
}

fn cmd_run(cwd: &PathBuf, name: &str) -> Result<String, String> {
    let path = find_workflow(cwd, name).ok_or_else(|| format!("Workflow '{}' not found", name))?;

    let workflow = load_workflow(&path)?;

    info(&format!("Running workflow: {}", workflow.workflow.name));
    if let Some(desc) = &workflow.workflow.description {
        debug(&format!("  {}", desc));
    }

    // Collect inputs
    let variables = if workflow.inputs.is_empty() {
        std::collections::HashMap::new()
    } else {
        info("Collecting inputs...");
        collect_inputs(&workflow.inputs)?
    };

    // Execute steps
    if workflow.steps.is_empty() {
        return Ok("Workflow has no steps to execute".to_string());
    }

    info("Executing steps...");
    execute_steps(&workflow.steps, &variables)?;

    success(&format!("Workflow '{}' completed successfully!", name));
    Ok(String::new())
}

/// Generate shell completions for workflow names
fn cmd_completions(cwd: &PathBuf, args: &[&str]) -> Result<String, String> {
    // Parse position (1-based, position of word being completed)
    let position: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);

    // Get the words typed so far (after --completions and position)
    let words: Vec<&str> = args.iter().skip(1).copied().collect();

    // Position 1 = completing the subcommand/workflow name
    if position == 1 {
        let mut completions = Vec::new();

        // Add static subcommands
        completions.push("list\tList available workflows".to_string());
        completions.push("show\tShow workflow definition".to_string());

        // Add workflow names
        let workflows = discover_workflows(cwd);
        for wf in workflows {
            let desc = wf.description.as_deref().unwrap_or("Run workflow");
            completions.push(format!("{}\t{}", wf.name, desc));
        }

        return Ok(completions.join("\n"));
    }

    // Position 2+ = context-dependent completions
    let subcommand = words.first().copied().unwrap_or("");

    match subcommand {
        "show" => {
            // Complete workflow names for 'show'
            if position == 2 {
                let workflows = discover_workflows(cwd);
                let completions: Vec<String> = workflows
                    .iter()
                    .map(|wf| {
                        let desc = wf.description.as_deref().unwrap_or("Show this workflow");
                        format!("{}\t{}", wf.name, desc)
                    })
                    .collect();
                return Ok(completions.join("\n"));
            }
        }
        _ => {
            // For workflow runs or unknown subcommands, no additional completions
        }
    }

    Ok(String::new())
}

/// List available commands for discovery (used by external tooling)
#[allow(dead_code)]
pub fn list_commands() -> serde_json::Value {
    json!([
        {"name": "list", "description": "List available workflows", "usage": "list"},
        {"name": "show", "description": "Show workflow definition", "usage": "show <name>"},
        {"name": "<name>", "description": "Run a workflow by name", "usage": "<name>"}
    ])
}
