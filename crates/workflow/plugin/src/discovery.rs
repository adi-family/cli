//! Workflow file discovery

use crate::parser::{load_workflow, WorkflowScope, WorkflowSummary};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn global_workflows_dir() -> PathBuf {
    lib_plugin_prelude::PluginCtx::data_dir().join("workflows")
}

fn local_workflows_dir(cwd: &Path) -> PathBuf {
    cwd.join(".adi").join("workflows")
}

pub fn discover_workflows(cwd: &Path) -> Vec<WorkflowSummary> {
    let mut workflows: HashMap<String, WorkflowSummary> = HashMap::new();

    let global_dir = global_workflows_dir();
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

    // Local workflows override global ones
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

fn load_workflow_summary(path: &Path, scope: WorkflowScope) -> Option<WorkflowSummary> {
    let workflow = load_workflow(path).ok()?;
    Some(WorkflowSummary {
        name: workflow.workflow.name,
        description: workflow.workflow.description,
        path: path.to_string_lossy().to_string(),
        scope,
    })
}

pub fn find_workflow(cwd: &Path, name: &str) -> Option<PathBuf> {
    let local_path = local_workflows_dir(cwd).join(format!("{}.toml", name));
    if local_path.exists() {
        return Some(local_path);
    }

    let global_path = global_workflows_dir().join(format!("{}.toml", name));
    if global_path.exists() {
        return Some(global_path);
    }

    None
}
