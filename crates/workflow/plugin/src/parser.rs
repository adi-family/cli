//! TOML workflow file parsing and types

use lib_plugin_prelude::t;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowFile {
    pub workflow: WorkflowMeta,
    #[serde(default)]
    pub inputs: Vec<Input>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowMeta {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Shell command to run before collecting inputs (e.g. show current state)
    #[serde(default)]
    pub pre_run: Option<String>,
}

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
    LinesFromFile { path: String },
    CargoWorkspaceMembers,
}

fn default_compose_file() -> String {
    "docker-compose.yml".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InputType {
    Select,
    Input,
    Confirm,
    MultiSelect,
    Password,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Step {
    pub name: String,
    pub run: String,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

pub fn parse_workflow(content: &str) -> Result<WorkflowFile, String> {
    toml::from_str(content).map_err(|e| t!("workflow-common-error-parse", "error" => e.to_string()))
}

pub fn load_workflow(path: &Path) -> Result<WorkflowFile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| t!("workflow-common-error-read", "error" => e.to_string()))?;
    parse_workflow(&content)
}

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

        let branch_input = &workflow.inputs[0];
        assert!(branch_input.options_source.is_some());
        assert_eq!(branch_input.autocomplete, Some(true));
        assert_eq!(branch_input.autocomplete_count, None);

        let dir_input = &workflow.inputs[1];
        assert!(dir_input.options_source.is_some());

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

    #[test]
    fn test_parse_autocomplete_count() {
        let toml = r#"
[workflow]
name = "autocomplete-count-test"

[[inputs]]
name = "crate"
type = "select"
prompt = "Select crate"
options_source = { type = "cargo-workspace-members" }
autocomplete = true
autocomplete_count = 5

[[inputs]]
name = "branch"
type = "select"
prompt = "Select branch"
options_source = { type = "git-branches" }
autocomplete = true

[[steps]]
name = "Done"
run = "echo {{ crate }}"
"#;

        let workflow = parse_workflow(toml).unwrap();
        assert_eq!(workflow.inputs.len(), 2);

        let crate_input = &workflow.inputs[0];
        assert_eq!(crate_input.autocomplete, Some(true));
        assert_eq!(crate_input.autocomplete_count, Some(5));

        let branch_input = &workflow.inputs[1];
        assert_eq!(branch_input.autocomplete, Some(true));
        assert_eq!(branch_input.autocomplete_count, None);
    }
}
