//! Workflow file discovery
//!
//! Discovers workflows from:
//! 1. ./.adi/workflows/ (local, highest priority)
//! 2. ~/.adi/workflows/ (global, fallback)

use crate::parser::{load_workflow, WorkflowScope, WorkflowSummary};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Get the global workflows directory (<plugin-data-dir>/workflows/)
fn global_workflows_dir() -> PathBuf {
    lib_plugin_prelude::PluginCtx::data_dir().join("workflows")
}

/// Get the local workflows directory (./.adi/workflows/)
fn local_workflows_dir(cwd: &Path) -> PathBuf {
    cwd.join(".adi").join("workflows")
}

/// Discover all available workflows (local first, then global)
pub fn discover_workflows(cwd: &Path) -> Vec<WorkflowSummary> {
    let mut workflows: HashMap<String, WorkflowSummary> = HashMap::new();

    // First, add global workflows
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
pub fn find_workflow(cwd: &Path, name: &str) -> Option<PathBuf> {
    // Check local first
    let local_path = local_workflows_dir(cwd).join(format!("{}.toml", name));
    if local_path.exists() {
        return Some(local_path);
    }

    // Check global
    let global_path = global_workflows_dir().join(format!("{}.toml", name));
    if global_path.exists() {
        return Some(global_path);
    }

    None
}
