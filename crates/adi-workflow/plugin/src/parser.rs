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
    #[serde(default)]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub validation: Option<String>,
    #[serde(default)]
    pub env: Option<String>,
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
}
