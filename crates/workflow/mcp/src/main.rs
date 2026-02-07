//! MCP (Model Context Protocol) server for ADI Workflows.
//!
//! This provides an MCP interface to the workflow system,
//! allowing LLMs to discover, inspect, and execute workflows through the MCP protocol.

use lib_mcp_core::{
    server::{McpRouter, McpServerBuilder},
    transport::stdio::StdioTransport,
    CallToolResult, Tool, ToolInputSchema,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// =============================================================================
// Workflow Types (mirrored from plugin)
// =============================================================================

/// Root workflow file structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowFile {
    pub workflow: WorkflowMeta,
    #[serde(default)]
    pub inputs: Vec<Input>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// Workflow metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowMeta {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Input parameter definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Input {
    pub name: String,
    #[serde(rename = "type")]
    pub input_type: InputType,
    pub prompt: String,
    #[serde(default)]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub options_cmd: Option<String>,
    #[serde(default)]
    pub options_source: Option<OptionsSource>,
    #[serde(default)]
    pub autocomplete: Option<bool>,
    #[serde(default)]
    pub autocomplete_count: Option<usize>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub validation: Option<String>,
    #[serde(default)]
    pub env: Option<String>,
    #[serde(rename = "if", default)]
    pub condition: Option<String>,
}

/// Built-in options source providers
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum OptionsSource {
    GitBranches,
    GitTags,
    GitRemotes,
    DockerComposeServices {
        #[serde(default = "default_compose_file")]
        file: String,
    },
    Directories {
        path: String,
        #[serde(default)]
        pattern: Option<String>,
    },
    Files {
        path: String,
        #[serde(default)]
        pattern: Option<String>,
    },
    LinesFromFile {
        path: String,
    },
    CargoWorkspaceMembers,
}

fn default_compose_file() -> String {
    "docker-compose.yml".to_string()
}

/// Input types for interactive prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InputType {
    Select,
    Input,
    Confirm,
    MultiSelect,
    Password,
}

/// Workflow step definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Step {
    pub name: String,
    pub run: String,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

/// Workflow summary for listing
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowSummary {
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub scope: WorkflowScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowScope {
    Local,
    Global,
}

// =============================================================================
// Workflow Functions
// =============================================================================

/// Parse a workflow file from TOML
fn parse_workflow(content: &str) -> Result<WorkflowFile, String> {
    toml::from_str(content).map_err(|e| format!("Failed to parse workflow: {}", e))
}

/// Load and parse a workflow file from path
fn load_workflow(path: &Path) -> Result<WorkflowFile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read workflow file: {}", e))?;
    parse_workflow(&content)
}

/// Get the global workflows directory (~/.adi/workflows/)
fn global_workflows_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".adi").join("workflows"))
}

/// Get the local workflows directory (./.adi/workflows/)
fn local_workflows_dir(cwd: &Path) -> PathBuf {
    cwd.join(".adi").join("workflows")
}

/// Discover all available workflows (local first, then global)
fn discover_workflows(cwd: &Path) -> Vec<WorkflowSummary> {
    let mut workflows: HashMap<String, WorkflowSummary> = HashMap::new();

    // First, add global workflows
    if let Some(global_dir) = global_workflows_dir() {
        if global_dir.exists() {
            for entry in std::fs::read_dir(&global_dir).into_iter().flatten() {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "toml") {
                        if let Some(summary) = load_workflow_summary(&path, WorkflowScope::Global) {
                            workflows.insert(summary.name.clone(), summary);
                        }
                    }
                }
            }
        }
    }

    // Then, add local workflows (overriding global)
    let local_dir = local_workflows_dir(cwd);
    if local_dir.exists() {
        for entry in std::fs::read_dir(&local_dir).into_iter().flatten() {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "toml") {
                    if let Some(summary) = load_workflow_summary(&path, WorkflowScope::Local) {
                        workflows.insert(summary.name.clone(), summary);
                    }
                }
            }
        }
    }

    let mut result: Vec<_> = workflows.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

/// Load workflow summary from a file
fn load_workflow_summary(path: &Path, scope: WorkflowScope) -> Option<WorkflowSummary> {
    let workflow = load_workflow(path).ok()?;
    Some(WorkflowSummary {
        name: workflow.workflow.name,
        description: workflow.workflow.description,
        path: path.to_string_lossy().to_string(),
        scope,
    })
}

/// Find a specific workflow by name (local first, then global)
fn find_workflow(cwd: &Path, name: &str) -> Option<PathBuf> {
    // Check local first
    let local_path = local_workflows_dir(cwd).join(format!("{}.toml", name));
    if local_path.exists() {
        return Some(local_path);
    }

    // Check global
    if let Some(global_dir) = global_workflows_dir() {
        let global_path = global_dir.join(format!("{}.toml", name));
        if global_path.exists() {
            return Some(global_path);
        }
    }

    None
}

/// Resolve dynamic options for an input
fn resolve_options(input: &Input, cwd: &Path) -> Vec<String> {
    // Static options take priority
    if let Some(options) = &input.options {
        return options.clone();
    }

    // Shell command
    if let Some(cmd) = &input.options_cmd {
        if let Ok(output) = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(cwd)
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        return vec![];
    }

    // Built-in sources
    if let Some(source) = &input.options_source {
        return resolve_options_source(source, cwd);
    }

    vec![]
}

/// Resolve built-in options source
fn resolve_options_source(source: &OptionsSource, cwd: &Path) -> Vec<String> {
    match source {
        OptionsSource::GitBranches => {
            run_command_lines(cwd, "git", &["branch", "--format=%(refname:short)"])
        }
        OptionsSource::GitTags => run_command_lines(cwd, "git", &["tag", "--list"]),
        OptionsSource::GitRemotes => run_command_lines(cwd, "git", &["remote"]),
        OptionsSource::DockerComposeServices { file } => {
            // Parse docker-compose.yml for services
            let compose_path = cwd.join(file);
            if let Ok(content) = std::fs::read_to_string(&compose_path) {
                // Simple YAML parsing - look for "services:" section
                let mut in_services = false;
                let mut services = vec![];
                for line in content.lines() {
                    if line.trim() == "services:" {
                        in_services = true;
                        continue;
                    }
                    if in_services {
                        if !line.starts_with(' ') && !line.starts_with('\t') && !line.is_empty() {
                            break;
                        }
                        // Service names are keys at 2-space indent
                        if line.starts_with("  ") && !line.starts_with("   ") {
                            if let Some(name) = line.trim().strip_suffix(':') {
                                services.push(name.to_string());
                            }
                        }
                    }
                }
                services
            } else {
                vec![]
            }
        }
        OptionsSource::Directories { path, pattern } => {
            let base = cwd.join(path);
            if !base.exists() {
                return vec![];
            }
            let glob_pattern = match pattern {
                Some(p) => format!("{}/{}", base.display(), p),
                None => format!("{}/*", base.display()),
            };
            glob::glob(&glob_pattern)
                .ok()
                .map(|paths| {
                    paths
                        .filter_map(|p| p.ok())
                        .filter(|p| p.is_dir())
                        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                        .collect()
                })
                .unwrap_or_default()
        }
        OptionsSource::Files { path, pattern } => {
            let base = cwd.join(path);
            if !base.exists() {
                return vec![];
            }
            let glob_pattern = match pattern {
                Some(p) => format!("{}/{}", base.display(), p),
                None => format!("{}/*", base.display()),
            };
            glob::glob(&glob_pattern)
                .ok()
                .map(|paths| {
                    paths
                        .filter_map(|p| p.ok())
                        .filter(|p| p.is_file())
                        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                        .collect()
                })
                .unwrap_or_default()
        }
        OptionsSource::LinesFromFile { path } => {
            let file_path = cwd.join(path);
            std::fs::read_to_string(&file_path)
                .ok()
                .map(|content| {
                    content
                        .lines()
                        .map(|s| s.to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default()
        }
        OptionsSource::CargoWorkspaceMembers => {
            // Parse Cargo.toml for workspace members
            let cargo_path = cwd.join("Cargo.toml");
            if let Ok(content) = std::fs::read_to_string(&cargo_path) {
                if let Ok(toml) = content.parse::<toml::Value>() {
                    if let Some(workspace) = toml.get("workspace") {
                        if let Some(members) = workspace.get("members") {
                            if let Some(arr) = members.as_array() {
                                return arr
                                    .iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect();
                            }
                        }
                    }
                }
            }
            vec![]
        }
    }
}

/// Run a command and return lines of output
fn run_command_lines(cwd: &Path, cmd: &str, args: &[&str]) -> Vec<String> {
    std::process::Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Execute workflow steps (non-interactive, for MCP)
fn execute_workflow(
    workflow: &WorkflowFile,
    variables: &HashMap<String, serde_json::Value>,
    cwd: &Path,
) -> Result<String, String> {
    let env = create_template_env();
    let mut output = String::new();

    for (i, step) in workflow.steps.iter().enumerate() {
        // Check condition if present
        if let Some(condition) = &step.condition {
            if !evaluate_condition(&env, condition, variables)? {
                output.push_str(&format!(
                    "Skipping step {}: {} (condition not met)\n",
                    i + 1,
                    step.name
                ));
                continue;
            }
        }

        output.push_str(&format!("Running step {}: {}\n", i + 1, step.name));

        // Render the command template
        let command = render_template(&env, &step.run, variables)?;

        // Render environment variables if present
        let step_env = if let Some(env_vars) = &step.env {
            Some(render_env_vars(&env, env_vars, variables)?)
        } else {
            None
        };

        // Execute the command
        let step_output = execute_command(&command, step_env.as_ref(), cwd)?;
        output.push_str(&step_output);
        output.push('\n');
    }

    Ok(output)
}

/// Create a template environment with built-in functions
fn create_template_env() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_auto_escape_callback(|_| minijinja::AutoEscape::None);
    env
}

/// Render a template with variables
fn render_template(
    env: &minijinja::Environment,
    template: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    let tmpl = env
        .template_from_str(template)
        .map_err(|e| format!("Template parse error: {}", e))?;
    tmpl.render(variables)
        .map_err(|e| format!("Template render error: {}", e))
}

/// Evaluate a condition expression
fn evaluate_condition(
    env: &minijinja::Environment,
    condition: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<bool, String> {
    let template = format!("{{% if {} %}}true{{% else %}}false{{% endif %}}", condition);
    let result = render_template(env, &template, variables)?;
    Ok(result == "true")
}

/// Render environment variables
fn render_env_vars(
    env: &minijinja::Environment,
    env_vars: &HashMap<String, String>,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::new();
    for (key, value) in env_vars {
        let rendered = render_template(env, value, variables)?;
        result.insert(key.clone(), rendered);
    }
    Ok(result)
}

/// Execute a single shell command
fn execute_command(
    command: &str,
    env_vars: Option<&HashMap<String, String>>,
    cwd: &Path,
) -> Result<String, String> {
    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "bash"
    };

    let shell_arg = if cfg!(target_os = "windows") {
        "/C"
    } else {
        "-c"
    };

    let mut cmd = std::process::Command::new(shell);
    cmd.arg(shell_arg).arg(command).current_dir(cwd);

    if let Some(env) = env_vars {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let mut result = String::new();
    if !output.stdout.is_empty() {
        result.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        return Err(format!(
            "Command failed with exit code {}: {}",
            output.status.code().unwrap_or(-1),
            result
        ));
    }

    Ok(result)
}

// =============================================================================
// MCP Server
// =============================================================================

/// Shared state for the MCP server.
struct WorkflowState {
    project_path: PathBuf,
}

impl WorkflowState {
    fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }
}

#[tokio::main]
async fn main() -> lib_mcp_core::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Get project path from environment or current directory
    let project_path = std::env::var("PROJECT_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Create shared state
    let state = Arc::new(WorkflowState::new(project_path));

    // Build the MCP server with all tools
    let server = build_server(state);

    // Run with stdio transport
    let mut router = McpRouter::new(server);
    router.run(StdioTransport::new()).await
}

fn build_server(state: Arc<WorkflowState>) -> impl lib_mcp_core::server::McpHandler {
    // List workflows tool
    let s = state.clone();
    let list_workflows = move |_args: HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let workflows = discover_workflows(&state.project_path);
            Ok(CallToolResult::text(serde_json::to_string_pretty(
                &workflows,
            )?))
        }
    };

    // Get workflow details tool
    let s = state.clone();
    let get_workflow = move |args: HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("name is required".into()))?;

            let path = find_workflow(&state.project_path, name).ok_or_else(|| {
                lib_mcp_core::Error::Internal(format!("Workflow '{}' not found", name))
            })?;

            let workflow = load_workflow(&path).map_err(|e| lib_mcp_core::Error::Internal(e))?;

            // Include resolved options for inputs
            let mut workflow_info = serde_json::json!({
                "name": workflow.workflow.name,
                "description": workflow.workflow.description,
                "path": path.to_string_lossy(),
                "inputs": [],
                "steps": workflow.steps.iter().map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "condition": s.condition,
                        "env": s.env,
                        "run": s.run
                    })
                }).collect::<Vec<_>>()
            });

            // Resolve options for each input
            let inputs: Vec<serde_json::Value> = workflow
                .inputs
                .iter()
                .map(|input| {
                    let options = resolve_options(input, &state.project_path);
                    serde_json::json!({
                        "name": input.name,
                        "type": input.input_type,
                        "prompt": input.prompt,
                        "options": options,
                        "default": input.default,
                        "condition": input.condition
                    })
                })
                .collect();

            workflow_info["inputs"] = serde_json::json!(inputs);

            Ok(CallToolResult::text(serde_json::to_string_pretty(
                &workflow_info,
            )?))
        }
    };

    // Run workflow tool
    let s = state.clone();
    let run_workflow = move |args: HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("name is required".into()))?;

            let path = find_workflow(&state.project_path, name).ok_or_else(|| {
                lib_mcp_core::Error::Internal(format!("Workflow '{}' not found", name))
            })?;

            let workflow = load_workflow(&path).map_err(|e| lib_mcp_core::Error::Internal(e))?;

            // Extract input variables from args
            let mut variables: HashMap<String, serde_json::Value> = HashMap::new();
            if let Some(inputs) = args.get("inputs") {
                if let Some(obj) = inputs.as_object() {
                    for (k, v) in obj {
                        variables.insert(k.clone(), v.clone());
                    }
                }
            }

            // Add built-in variables
            variables.insert(
                "cwd".to_string(),
                serde_json::json!(state.project_path.to_string_lossy()),
            );

            // Execute the workflow
            let output = execute_workflow(&workflow, &variables, &state.project_path)
                .map_err(|e| lib_mcp_core::Error::Internal(e))?;

            Ok(CallToolResult::text(
                serde_json::json!({
                    "workflow": name,
                    "status": "completed",
                    "output": output
                })
                .to_string(),
            ))
        }
    };

    // Get workflow options tool (resolve dynamic options for a specific input)
    let s = state.clone();
    let get_workflow_options = move |args: HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let workflow_name = args
                .get("workflow")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("workflow is required".into()))?;

            let input_name = args
                .get("input")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("input is required".into()))?;

            let path = find_workflow(&state.project_path, workflow_name).ok_or_else(|| {
                lib_mcp_core::Error::Internal(format!("Workflow '{}' not found", workflow_name))
            })?;

            let workflow = load_workflow(&path).map_err(|e| lib_mcp_core::Error::Internal(e))?;

            let input = workflow
                .inputs
                .iter()
                .find(|i| i.name == input_name)
                .ok_or_else(|| {
                    lib_mcp_core::Error::Internal(format!("Input '{}' not found", input_name))
                })?;

            let options = resolve_options(input, &state.project_path);

            Ok(CallToolResult::text(serde_json::to_string_pretty(
                &options,
            )?))
        }
    };

    // Build the server
    McpServerBuilder::new("adi-workflow-mcp", env!("CARGO_PKG_VERSION"))
        .instructions(
            "ADI Workflow MCP Server - Run workflows defined in TOML files. \
             Workflows are discovered from ./.adi/workflows/ (local) and ~/.adi/workflows/ (global). \
             Each workflow can have inputs (select, input, confirm, multi-select, password) and steps (shell commands). \
             Use list_workflows to discover available workflows, get_workflow to see details and required inputs, \
             and run_workflow to execute a workflow with the specified inputs.",
        )
        // List workflows
        .tool(
            Tool::new("list_workflows", ToolInputSchema::new())
                .with_description("List all available workflows (local and global)"),
            list_workflows,
        )
        // Get workflow details
        .tool(
            Tool::new(
                "get_workflow",
                ToolInputSchema::new()
                    .string_property("name", "Name of the workflow to get details for", true),
            )
            .with_description(
                "Get detailed information about a workflow including inputs, steps, and resolved options",
            ),
            get_workflow,
        )
        // Run workflow
        .tool(
            Tool::new(
                "run_workflow",
                ToolInputSchema::new()
                    .string_property("name", "Name of the workflow to run", true)
                    .property(
                        "inputs",
                        serde_json::json!({
                            "type": "object",
                            "description": "Input values for the workflow (key-value pairs matching the workflow's input definitions)",
                            "additionalProperties": true
                        }),
                        false,
                    ),
            )
            .with_description(
                "Execute a workflow with the specified input values. Get workflow details first to see required inputs.",
            ),
            run_workflow,
        )
        // Get workflow options
        .tool(
            Tool::new(
                "get_workflow_options",
                ToolInputSchema::new()
                    .string_property("workflow", "Name of the workflow", true)
                    .string_property("input", "Name of the input to get options for", true),
            )
            .with_description(
                "Get the available options for a specific workflow input (resolves dynamic options like git branches, directories, etc.)",
            ),
            get_workflow_options,
        )
        .with_logging()
        .build()
}
