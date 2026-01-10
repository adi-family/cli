//! CLI command handling

use crate::discovery::{discover_workflows, find_workflow};
use crate::executor::execute_steps;
use crate::parser::{WorkflowScope, load_workflow};
use crate::prompts::collect_inputs;
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
        "" => {
            // Default to help
            Ok(help_text())
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

    println!("Running workflow: {}", workflow.workflow.name);
    if let Some(desc) = &workflow.workflow.description {
        println!("  {}", desc);
    }
    println!();

    // Collect inputs
    let variables = if workflow.inputs.is_empty() {
        std::collections::HashMap::new()
    } else {
        println!("Collecting inputs...\n");
        collect_inputs(&workflow.inputs)?
    };

    // Execute steps
    if workflow.steps.is_empty() {
        return Ok("Workflow has no steps to execute".to_string());
    }

    println!("\nExecuting steps...\n");
    execute_steps(&workflow.steps, &variables)?;

    Ok(format!("\nWorkflow '{}' completed successfully!", name))
}

/// List available commands for discovery
pub fn list_commands() -> serde_json::Value {
    json!([
        {"name": "list", "description": "List available workflows", "usage": "list"},
        {"name": "show", "description": "Show workflow definition", "usage": "show <name>"},
        {"name": "<name>", "description": "Run a workflow by name", "usage": "<name>"}
    ])
}
