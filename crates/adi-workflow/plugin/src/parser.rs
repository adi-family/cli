//! TOML workflow file parsing and types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

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
    /// Static options list
    #[serde(default)]
    pub options: Option<Vec<String>>,
    /// Shell command that outputs options (one per line)
    #[serde(default)]
    pub options_cmd: Option<String>,
    /// Built-in options provider (git-branches, git-tags, plugins, services, directories, files)
    #[serde(default)]
    pub options_source: Option<OptionsSource>,
    /// Enable fuzzy search/autocomplete for select inputs
    #[serde(default)]
    pub autocomplete: Option<bool>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub validation: Option<String>,
    #[serde(default)]
    pub env: Option<String>,
    /// Conditional expression (Jinja2 template that evaluates to truthy/falsy)
    #[serde(rename = "if", default)]
    pub condition: Option<String>,
}

/// Built-in options source providers
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum OptionsSource {
    /// Git branches from current repository
    GitBranches,
    /// Git tags from current repository
    GitTags,
    /// Git remotes from current repository
    GitRemotes,
    /// Services from docker-compose.yml
    DockerComposeServices {
        #[serde(default = "default_compose_file")]
        file: String,
    },
    /// Directories matching a glob pattern
    Directories {
        path: String,
        #[serde(default)]
        pattern: Option<String>,
    },
    /// Files matching a glob pattern
    Files {
        path: String,
        #[serde(default)]
        pattern: Option<String>,
    },
    /// Lines from a file
    LinesFromFile { path: String },
    /// Cargo workspace members
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

/// Parse a workflow file from TOML
pub fn parse_workflow(content: &str) -> Result<WorkflowFile, String> {
    toml::from_str(content).map_err(|e| format!("Failed to parse workflow: {}", e))
}

/// Load and parse a workflow file from path
pub fn load_workflow(path: &Path) -> Result<WorkflowFile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read workflow file: {}", e))?;
    parse_workflow(&content)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workflow() {
        let toml = r#"
[workflow]
name = "test"
description = "A test workflow"

[[inputs]]
name = "env"
type = "select"
prompt = "Select environment"
options = ["dev", "prod"]

[[steps]]
name = "Build"
run = "cargo build"
"#;

        let workflow = parse_workflow(toml).unwrap();
        assert_eq!(workflow.workflow.name, "test");
        assert_eq!(workflow.inputs.len(), 1);
        assert_eq!(workflow.steps.len(), 1);
    }

    #[test]
    fn test_parse_workflow_with_dynamic_options() {
        let toml = r#"
[workflow]
name = "dynamic-test"
description = "Test dynamic options"

[[inputs]]
name = "branch"
type = "select"
prompt = "Select branch"
options_source = { type = "git-branches" }
autocomplete = true

[[inputs]]
name = "dir"
type = "select"
prompt = "Select directory"
options_source = { type = "directories", path = "crates" }

[[inputs]]
name = "service"
type = "select"
prompt = "Select service"
options_source = { type = "docker-compose-services", file = "docker-compose.dev.yml" }

[[steps]]
name = "Run"
run = "echo {{ branch }} {{ dir }} {{ service }}"
"#;

        let workflow = parse_workflow(toml).unwrap();
        assert_eq!(workflow.workflow.name, "dynamic-test");
        assert_eq!(workflow.inputs.len(), 3);

        // Check branch input
        let branch_input = &workflow.inputs[0];
        assert!(branch_input.options_source.is_some());
        assert_eq!(branch_input.autocomplete, Some(true));

        // Check dir input
        let dir_input = &workflow.inputs[1];
        assert!(dir_input.options_source.is_some());

        // Check service input
        let service_input = &workflow.inputs[2];
        assert!(service_input.options_source.is_some());
    }

    #[test]
    fn test_parse_options_source_variants() {
        let toml = r#"
[workflow]
name = "source-variants"

[[inputs]]
name = "dir"
type = "select"
prompt = "Select directory"
options_source = { type = "directories", path = "crates", pattern = "adi-*" }

[[inputs]]
name = "file"
type = "select"
prompt = "Select file"
options_source = { type = "files", path = ".", pattern = "*.toml" }

[[inputs]]
name = "line"
type = "select"
prompt = "Select from file"
options_source = { type = "lines-from-file", path = ".adi/services.txt" }

[[steps]]
name = "Done"
run = "echo done"
"#;

        let workflow = parse_workflow(toml).unwrap();
        assert_eq!(workflow.inputs.len(), 3);
    }
}
