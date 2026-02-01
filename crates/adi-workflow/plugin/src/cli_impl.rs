//! CLI command handling

use crate::discovery::{discover_workflows, find_workflow};
use crate::executor::execute_steps;
use crate::options::resolve_options;
use crate::parser::{load_workflow, InputType, WorkflowScope};
use crate::prompts::collect_inputs_with_prefilled;
use lib_console_output::{debug, info, is_interactive, success, Select, SelectOption};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

/// Parsed CLI arguments for workflow execution
#[derive(Debug, Default)]
struct ParsedArgs {
    workflow_name: Option<String>,
    inputs: HashMap<String, String>,
    show_schema: bool,
    show_help: bool,
}

/// Parse CLI arguments, extracting --input/-i flags
fn parse_args(args: &[String]) -> ParsedArgs {
    let mut parsed = ParsedArgs::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "--schema" => {
                parsed.show_schema = true;
                i += 1;
            }
            "--help" | "-h" => {
                parsed.show_help = true;
                i += 1;
            }
            "--input" | "-i" => {
                // Next arg should be key=value
                if i + 1 < args.len() {
                    if let Some((key, value)) = args[i + 1].split_once('=') {
                        parsed.inputs.insert(key.to_string(), value.to_string());
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            arg if arg.starts_with("--input=") => {
                // --input=key=value format
                if let Some(kv) = arg.strip_prefix("--input=") {
                    if let Some((key, value)) = kv.split_once('=') {
                        parsed.inputs.insert(key.to_string(), value.to_string());
                    }
                }
                i += 1;
            }
            arg if arg.starts_with("-i=") => {
                // -i=key=value format
                if let Some(kv) = arg.strip_prefix("-i=") {
                    if let Some((key, value)) = kv.split_once('=') {
                        parsed.inputs.insert(key.to_string(), value.to_string());
                    }
                }
                i += 1;
            }
            _ => {
                // First non-flag argument is the workflow name
                if parsed.workflow_name.is_none() && !arg.starts_with('-') {
                    parsed.workflow_name = Some(arg.clone());
                }
                i += 1;
            }
        }
    }

    parsed
}

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
        "--help" | "-h" => Ok(help_text()),
        "" => {
            // No subcommand - show interactive workflow selector
            cmd_select_and_run(&cwd)
        }
        workflow_name => {
            // Parse remaining args for --input flags
            let remaining_args: Vec<String> = args.iter().skip(1).cloned().collect();
            let parsed = parse_args(&remaining_args);

            if parsed.show_help {
                return Ok(help_text());
            }

            if parsed.show_schema {
                return cmd_schema(&cwd, workflow_name);
            }

            // Run workflow with pre-filled inputs
            cmd_run_with_inputs(&cwd, workflow_name, parsed.inputs)
        }
    }
}

fn help_text() -> String {
    r#"ADI Workflow - Run workflows defined in TOML files

Commands:
  <name>              Run a workflow by name
  list                List available workflows
  show <name>         Show workflow definition

Options:
  -i, --input KEY=VAL  Pre-fill input value (repeatable)
  --schema             Output workflow inputs as JSON schema (for LLM/automation)
  -h, --help           Show this help message

Workflow locations:
  ./.adi/workflows/<name>.toml  (local, highest priority)
  ~/.adi/workflows/<name>.toml  (global)

Examples:
  adi workflow deploy                              # Interactive mode
  adi workflow list                                # List available workflows
  adi workflow show deploy                         # Show workflow details
  adi workflow deploy --schema                     # Get inputs as JSON schema
  adi workflow deploy -i env=prod -i version=1.0  # Non-interactive with inputs"#
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
        Some(workflow_name) => cmd_run_with_inputs(cwd, &workflow_name, HashMap::new()),
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

/// Output workflow inputs as JSON schema for LLM/automation use
fn cmd_schema(cwd: &PathBuf, name: &str) -> Result<String, String> {
    let path = find_workflow(cwd, name).ok_or_else(|| format!("Workflow '{}' not found", name))?;
    let workflow = load_workflow(&path)?;

    let mut inputs_schema = Vec::new();

    for input in &workflow.inputs {
        let mut input_obj = json!({
            "name": input.name,
            "type": format!("{:?}", input.input_type).to_lowercase(),
            "prompt": input.prompt,
        });

        // Add options if available (try to resolve them)
        if input.input_type == InputType::Select || input.input_type == InputType::MultiSelect {
            // Try to resolve options, but don't fail if we can't
            if let Ok(options) = resolve_options(input, &HashMap::new()) {
                input_obj["options"] = json!(options);
            } else if let Some(opts) = &input.options {
                input_obj["options"] = json!(opts);
            } else if input.options_cmd.is_some() {
                input_obj["options_dynamic"] = json!("Run workflow to see dynamic options");
            } else if input.options_source.is_some() {
                input_obj["options_dynamic"] = json!(format!("{:?}", input.options_source));
            }
        }

        if let Some(default) = &input.default {
            input_obj["default"] = default.clone();
        }

        if let Some(env) = &input.env {
            input_obj["env"] = json!(env);
        }

        if let Some(condition) = &input.condition {
            input_obj["condition"] = json!(condition);
        }

        if let Some(validation) = &input.validation {
            input_obj["validation"] = json!(validation);
        }

        inputs_schema.push(input_obj);
    }

    let schema = json!({
        "workflow": workflow.workflow.name,
        "description": workflow.workflow.description,
        "inputs": inputs_schema,
        "usage": format!("adi workflow {} {}", name,
            workflow.inputs.iter()
                .map(|i| format!("-i {}=<value>", i.name))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    });

    Ok(serde_json::to_string_pretty(&schema).unwrap_or_else(|_| schema.to_string()))
}

/// Run workflow with pre-filled inputs from CLI arguments
fn cmd_run_with_inputs(
    cwd: &PathBuf,
    name: &str,
    prefilled: HashMap<String, String>,
) -> Result<String, String> {
    let path = find_workflow(cwd, name).ok_or_else(|| format!("Workflow '{}' not found", name))?;

    let workflow = load_workflow(&path)?;

    info(&format!("Running workflow: {}", workflow.workflow.name));
    if let Some(desc) = &workflow.workflow.description {
        debug(&format!("  {}", desc));
    }

    // Collect inputs with pre-filled values
    let variables = if workflow.inputs.is_empty() {
        HashMap::new()
    } else {
        if !prefilled.is_empty() {
            debug(&format!(
                "Pre-filled inputs: {:?}",
                prefilled.keys().collect::<Vec<_>>()
            ));
        }
        collect_inputs_with_prefilled(&workflow.inputs, prefilled)?
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_empty() {
        let args: Vec<String> = vec![];
        let parsed = parse_args(&args);
        assert!(parsed.workflow_name.is_none());
        assert!(parsed.inputs.is_empty());
        assert!(!parsed.show_schema);
        assert!(!parsed.show_help);
    }

    #[test]
    fn test_parse_args_schema_flag() {
        let args = vec!["--schema".to_string()];
        let parsed = parse_args(&args);
        assert!(parsed.show_schema);
        assert!(!parsed.show_help);
    }

    #[test]
    fn test_parse_args_help_flags() {
        let args = vec!["--help".to_string()];
        let parsed = parse_args(&args);
        assert!(parsed.show_help);

        let args = vec!["-h".to_string()];
        let parsed = parse_args(&args);
        assert!(parsed.show_help);
    }

    #[test]
    fn test_parse_args_input_flag_separate() {
        let args = vec!["-i".to_string(), "key=value".to_string()];
        let parsed = parse_args(&args);
        assert_eq!(parsed.inputs.get("key"), Some(&"value".to_string()));

        let args = vec!["--input".to_string(), "foo=bar".to_string()];
        let parsed = parse_args(&args);
        assert_eq!(parsed.inputs.get("foo"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_parse_args_input_flag_equals() {
        let args = vec!["-i=key=value".to_string()];
        let parsed = parse_args(&args);
        assert_eq!(parsed.inputs.get("key"), Some(&"value".to_string()));

        let args = vec!["--input=foo=bar".to_string()];
        let parsed = parse_args(&args);
        assert_eq!(parsed.inputs.get("foo"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_parse_args_multiple_inputs() {
        let args = vec![
            "-i".to_string(),
            "plugin=adi.workflow".to_string(),
            "-i".to_string(),
            "action=build".to_string(),
            "--input".to_string(),
            "skip_lint=true".to_string(),
        ];
        let parsed = parse_args(&args);
        assert_eq!(parsed.inputs.len(), 3);
        assert_eq!(
            parsed.inputs.get("plugin"),
            Some(&"adi.workflow".to_string())
        );
        assert_eq!(parsed.inputs.get("action"), Some(&"build".to_string()));
        assert_eq!(parsed.inputs.get("skip_lint"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_args_mixed_flags_and_inputs() {
        let args = vec![
            "--schema".to_string(),
            "-i".to_string(),
            "key=value".to_string(),
        ];
        let parsed = parse_args(&args);
        assert!(parsed.show_schema);
        assert_eq!(parsed.inputs.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_parse_args_value_with_equals() {
        // Test value that contains = sign
        let args = vec![
            "-i".to_string(),
            "url=http://example.com?foo=bar".to_string(),
        ];
        let parsed = parse_args(&args);
        assert_eq!(
            parsed.inputs.get("url"),
            Some(&"http://example.com?foo=bar".to_string())
        );
    }
}
