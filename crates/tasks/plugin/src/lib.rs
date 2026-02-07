//! ADI Tasks Plugin (v3 ABI)
//!
//! Provides CLI commands for task management with dependency tracking.

use lib_plugin_abi_v3::{
    async_trait,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult, SERVICE_CLI_COMMANDS,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use tasks_core::{CreateTask, TaskId, TaskManager, TaskStatus};

// Local result type for command implementations
type CmdResult = std::result::Result<String, String>;

/// ADI Tasks Plugin
pub struct TasksPlugin {
    /// Task manager instance
    tasks: Arc<RwLock<Option<TaskManager>>>,
}

impl TasksPlugin {
    /// Create a new tasks plugin
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(None)),
        }
    }
}

impl Default for TasksPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for TasksPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.tasks".to_string(),
            name: "ADI Tasks".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Task management with dependency tracking".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        // Initialize task manager
        let manager = TaskManager::open_global().ok();
        *self.tasks.write().await = manager;
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS]
    }
}

#[async_trait]
impl CliCommands for TasksPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "list".to_string(),
                description: "List all tasks".to_string(),
                usage: "list [--status <status>] [--ready] [--blocked]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "add".to_string(),
                description: "Add a new task".to_string(),
                usage: "add <title> [--description <desc>]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "show".to_string(),
                description: "Show task details".to_string(),
                usage: "show <id>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "status".to_string(),
                description: "Update task status".to_string(),
                usage: "status <id> <status>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "delete".to_string(),
                description: "Delete a task".to_string(),
                usage: "delete <id> [--force]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "depend".to_string(),
                description: "Add dependency".to_string(),
                usage: "depend <task-id> <depends-on-id>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "undepend".to_string(),
                description: "Remove dependency".to_string(),
                usage: "undepend <task-id> <depends-on-id>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "graph".to_string(),
                description: "Show dependency graph".to_string(),
                usage: "graph [--format <text|dot|json>]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "search".to_string(),
                description: "Search tasks".to_string(),
                usage: "search <query> [--limit <n>]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "blocked".to_string(),
                description: "Show blocked tasks".to_string(),
                usage: "blocked".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "cycles".to_string(),
                description: "Detect dependency cycles".to_string(),
                usage: "cycles".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "stats".to_string(),
                description: "Show task statistics".to_string(),
                usage: "stats".to_string(),
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> {
        let tasks_guard = self.tasks.read().await;
        let tasks = tasks_guard.as_ref().ok_or_else(|| {
            lib_plugin_abi_v3::PluginError::CommandFailed("Tasks not initialized".to_string())
        })?;

        let subcommand = ctx.subcommand.as_deref().unwrap_or("");

        let result = match subcommand {
            "list" => cmd_list(tasks, ctx),
            "add" => cmd_add(tasks, ctx),
            "show" => cmd_show(tasks, ctx),
            "status" => cmd_status(tasks, ctx),
            "delete" => cmd_delete(tasks, ctx),
            "depend" => cmd_depend(tasks, ctx),
            "undepend" => cmd_undepend(tasks, ctx),
            "graph" => cmd_graph(tasks, ctx),
            "search" => cmd_search(tasks, ctx),
            "blocked" => cmd_blocked(tasks),
            "cycles" => cmd_cycles(tasks),
            "stats" => cmd_stats(tasks),
            "" => Ok(get_help()),
            _ => Err(format!("Unknown command: {}", subcommand)),
        };

        match result {
            Ok(output) => Ok(CliResult::success(output)),
            Err(e) => Ok(CliResult::error(e)),
        }
    }
}

// === Plugin Entry Points ===

/// Create the plugin instance (v3 entry point)
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(TasksPlugin::new())
}

/// Create the CLI commands interface (for separate trait object)
#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(TasksPlugin::new())
}

// === Command Implementations ===

fn get_help() -> String {
    "ADI Tasks - Task management with dependency tracking\n\n\
     Commands:\n  \
     list     List all tasks\n  \
     add      Add a new task\n  \
     show     Show task details\n  \
     status   Update task status\n  \
     delete   Delete a task\n  \
     depend   Add dependency\n  \
     undepend Remove dependency\n  \
     graph    Show dependency graph\n  \
     search   Search tasks\n  \
     blocked  Show blocked tasks\n  \
     cycles   Detect dependency cycles\n  \
     stats    Show task statistics\n\n\
     Usage: adi tasks <command> [args]"
        .to_string()
}

fn cmd_list(tasks: &TaskManager, ctx: &CliContext) -> CmdResult {
    let status_filter: Option<String> = ctx.option("status");
    let ready = ctx.has_flag("ready");
    let blocked = ctx.has_flag("blocked");
    let format: Option<String> = ctx.option("format");
    let format = format.as_deref().unwrap_or("text");

    let task_list = if ready {
        tasks.get_ready().map_err(|e| e.to_string())?
    } else if blocked {
        tasks.get_blocked().map_err(|e| e.to_string())?
    } else if let Some(ref status_str) = status_filter {
        let status: TaskStatus = status_str
            .parse()
            .map_err(|_| format!("Invalid status: {}", status_str))?;
        tasks.get_by_status(status).map_err(|e| e.to_string())?
    } else {
        tasks.list().map_err(|e| e.to_string())?
    };

    if format == "json" {
        return serde_json::to_string_pretty(&task_list).map_err(|e| e.to_string());
    }

    if task_list.is_empty() {
        return Ok("No tasks found".to_string());
    }

    let mut output = String::new();
    for task in task_list {
        let status_icon = match task.status {
            TaskStatus::Todo => "○",
            TaskStatus::InProgress => "◐",
            TaskStatus::Done => "●",
            TaskStatus::Blocked => "✕",
            TaskStatus::Cancelled => "○",
        };
        let scope = if task.is_global() {
            "[global]"
        } else {
            "[project]"
        };
        output.push_str(&format!(
            "{} #{} {} {}\n",
            status_icon, task.id.0, task.title, scope
        ));
    }
    Ok(output.trim_end().to_string())
}

fn cmd_add(tasks: &TaskManager, ctx: &CliContext) -> CmdResult {
    let title = ctx.arg(0).ok_or_else(|| {
        "Missing title. Usage: add <title> [--description <desc>]".to_string()
    })?;

    let description: Option<String> = ctx.option("description");
    let depends_on_str: Option<String> = ctx.option("depends-on");
    let depends_on: Vec<i64> = depends_on_str
        .map(|s| {
            s.split(',')
                .filter_map(|id| id.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();

    let mut input = CreateTask::new(title);
    if let Some(desc) = description {
        input = input.with_description(desc);
    }
    if !depends_on.is_empty() {
        input = input.with_dependencies(depends_on.into_iter().map(TaskId).collect());
    }

    let id = tasks.create_task(input).map_err(|e| e.to_string())?;
    Ok(format!("Created task #{}: {}", id.0, title))
}

fn cmd_show(tasks: &TaskManager, ctx: &CliContext) -> CmdResult {
    let id_str = ctx
        .arg(0)
        .ok_or_else(|| "Missing task ID. Usage: show <id>".to_string())?;
    let id: i64 = id_str.parse().map_err(|_| "Invalid task ID")?;

    let task_with_deps = tasks
        .get_task_with_dependencies(TaskId(id))
        .map_err(|e| e.to_string())?;
    let task = &task_with_deps.task;

    let mut output = format!("Task #{}\n", task.id.0);
    output.push_str(&format!("  Title: {}\n", task.title));
    output.push_str(&format!("  Status: {:?}\n", task.status));

    if let Some(ref desc) = task.description {
        output.push_str(&format!("  Description: {}\n", desc));
    }

    if let Some(symbol_id) = task.symbol_id {
        output.push_str(&format!("  Linked symbol: #{}\n", symbol_id));
    }

    let scope = if task.is_global() { "global" } else { "project" };
    output.push_str(&format!("  Scope: {}\n", scope));

    if !task_with_deps.depends_on.is_empty() {
        output.push_str("\n  Dependencies:\n");
        for dep in &task_with_deps.depends_on {
            output.push_str(&format!("    #{}: {}\n", dep.id.0, dep.title));
        }
    }

    if !task_with_deps.dependents.is_empty() {
        output.push_str("\n  Dependents:\n");
        for dep in &task_with_deps.dependents {
            output.push_str(&format!("    #{}: {}\n", dep.id.0, dep.title));
        }
    }

    Ok(output.trim_end().to_string())
}

fn cmd_status(tasks: &TaskManager, ctx: &CliContext) -> CmdResult {
    let id_str = ctx.arg(0).ok_or_else(|| {
        "Missing arguments. Usage: status <id> <status>".to_string()
    })?;
    let status_str = ctx.arg(1).ok_or_else(|| {
        "Missing arguments. Usage: status <id> <status>".to_string()
    })?;

    let id: i64 = id_str.parse().map_err(|_| "Invalid task ID")?;
    let status: TaskStatus = status_str
        .parse()
        .map_err(|_| format!("Invalid status: {}", status_str))?;

    tasks
        .update_status(TaskId(id), status)
        .map_err(|e| e.to_string())?;
    Ok(format!("Task #{} status updated to {:?}", id, status))
}

fn cmd_delete(tasks: &TaskManager, ctx: &CliContext) -> CmdResult {
    let id_str = ctx
        .arg(0)
        .ok_or_else(|| "Missing task ID. Usage: delete <id> [--force]".to_string())?;
    let id: i64 = id_str.parse().map_err(|_| "Invalid task ID")?;
    let force = ctx.has_flag("force");

    let task = tasks.get_task(TaskId(id)).map_err(|e| e.to_string())?;

    if !force {
        return Ok(format!(
            "Delete task #{}: {}?\nUse --force to confirm deletion",
            id, task.title
        ));
    }

    tasks.delete_task(TaskId(id)).map_err(|e| e.to_string())?;
    Ok(format!("Deleted task #{}: {}", id, task.title))
}

fn cmd_depend(tasks: &TaskManager, ctx: &CliContext) -> CmdResult {
    let task_id_str = ctx.arg(0).ok_or_else(|| {
        "Missing arguments. Usage: depend <task-id> <depends-on-id>".to_string()
    })?;
    let depends_on_str = ctx.arg(1).ok_or_else(|| {
        "Missing arguments. Usage: depend <task-id> <depends-on-id>".to_string()
    })?;

    let task_id: i64 = task_id_str.parse().map_err(|_| "Invalid task ID")?;
    let depends_on: i64 = depends_on_str.parse().map_err(|_| "Invalid depends-on ID")?;

    tasks
        .add_dependency(TaskId(task_id), TaskId(depends_on))
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "Task #{} now depends on task #{}",
        task_id, depends_on
    ))
}

fn cmd_undepend(tasks: &TaskManager, ctx: &CliContext) -> CmdResult {
    let task_id_str = ctx.arg(0).ok_or_else(|| {
        "Missing arguments. Usage: undepend <task-id> <depends-on-id>".to_string()
    })?;
    let depends_on_str = ctx.arg(1).ok_or_else(|| {
        "Missing arguments. Usage: undepend <task-id> <depends-on-id>".to_string()
    })?;

    let task_id: i64 = task_id_str.parse().map_err(|_| "Invalid task ID")?;
    let depends_on: i64 = depends_on_str.parse().map_err(|_| "Invalid depends-on ID")?;

    tasks
        .remove_dependency(TaskId(task_id), TaskId(depends_on))
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "Removed dependency: #{} -> #{}",
        task_id, depends_on
    ))
}

fn cmd_graph(tasks: &TaskManager, ctx: &CliContext) -> CmdResult {
    let format: Option<String> = ctx.option("format");
    let format = format.as_deref().unwrap_or("text");
    let all_tasks = tasks.list().map_err(|e| e.to_string())?;

    if format == "json" {
        let mut graph_data = Vec::new();
        for task in &all_tasks {
            let deps = tasks.get_dependencies(task.id).map_err(|e| e.to_string())?;
            graph_data.push(json!({
                "task": task,
                "dependencies": deps.iter().map(|d| d.id.0).collect::<Vec<_>>()
            }));
        }
        return serde_json::to_string_pretty(&graph_data).map_err(|e| e.to_string());
    }

    if format == "dot" {
        let mut output = String::from("digraph tasks {\n  rankdir=LR;\n");
        for task in &all_tasks {
            let label = task.title.replace('"', "\\\"");
            let color = match task.status {
                TaskStatus::Done => "green",
                TaskStatus::InProgress => "blue",
                TaskStatus::Blocked => "red",
                TaskStatus::Cancelled => "gray",
                TaskStatus::Todo => "black",
            };
            output.push_str(&format!(
                "  {} [label=\"{}\" color=\"{}\"];\n",
                task.id.0, label, color
            ));

            let deps = tasks.get_dependencies(task.id).map_err(|e| e.to_string())?;
            for dep in deps {
                output.push_str(&format!("  {} -> {};\n", task.id.0, dep.id.0));
            }
        }
        output.push_str("}\n");
        return Ok(output);
    }

    // Text format
    if all_tasks.is_empty() {
        return Ok("No tasks found".to_string());
    }

    let mut output = String::from("Task Dependency Graph\n\n");
    for task in &all_tasks {
        let status_icon = match task.status {
            TaskStatus::Todo => "○",
            TaskStatus::InProgress => "◐",
            TaskStatus::Done => "●",
            TaskStatus::Blocked => "✕",
            TaskStatus::Cancelled => "○",
        };
        output.push_str(&format!("{} #{} {}\n", status_icon, task.id.0, task.title));

        let deps = tasks.get_dependencies(task.id).map_err(|e| e.to_string())?;
        for (i, dep) in deps.iter().enumerate() {
            let prefix = if i == deps.len() - 1 { "  └─" } else { "  ├─" };
            output.push_str(&format!(
                "{} depends on #{}: {}\n",
                prefix, dep.id.0, dep.title
            ));
        }
    }
    Ok(output.trim_end().to_string())
}

fn cmd_search(tasks: &TaskManager, ctx: &CliContext) -> CmdResult {
    let query = ctx
        .arg(0)
        .ok_or_else(|| "Missing query. Usage: search <query> [--limit <n>]".to_string())?;

    let limit: Option<usize> = ctx.option("limit");
    let limit = limit.unwrap_or(10);

    let results = tasks.search(query, limit).map_err(|e| e.to_string())?;

    if results.is_empty() {
        return Ok("No tasks found".to_string());
    }

    let mut output = format!("Found {} results for \"{}\":\n\n", results.len(), query);
    for task in results {
        let status_icon = match task.status {
            TaskStatus::Todo => "○",
            TaskStatus::InProgress => "◐",
            TaskStatus::Done => "●",
            TaskStatus::Blocked => "✕",
            TaskStatus::Cancelled => "○",
        };
        output.push_str(&format!("{} #{} {}\n", status_icon, task.id.0, task.title));
    }
    Ok(output.trim_end().to_string())
}

fn cmd_blocked(tasks: &TaskManager) -> CmdResult {
    let blocked = tasks.get_blocked().map_err(|e| e.to_string())?;

    if blocked.is_empty() {
        return Ok("No blocked tasks".to_string());
    }

    let mut output = String::from("Blocked Tasks\n\n");
    for task in blocked {
        output.push_str(&format!("✕ #{} {}\n", task.id.0, task.title));

        let blockers = tasks.get_dependencies(task.id).map_err(|e| e.to_string())?;
        let incomplete_blockers: Vec<_> = blockers
            .iter()
            .filter(|t| !t.status.is_complete())
            .collect();

        for blocker in incomplete_blockers {
            output.push_str(&format!(
                "  └─ blocked by #{}: {} ({:?})\n",
                blocker.id.0, blocker.title, blocker.status
            ));
        }
    }
    Ok(output.trim_end().to_string())
}

fn cmd_cycles(tasks: &TaskManager) -> CmdResult {
    let cycles = tasks.detect_cycles().map_err(|e| e.to_string())?;

    if cycles.is_empty() {
        return Ok("No circular dependencies detected".to_string());
    }

    let mut output = format!("Found {} circular dependencies:\n\n", cycles.len());
    for (i, cycle) in cycles.iter().enumerate() {
        output.push_str(&format!("  Cycle {}: ", i + 1));
        let cycle_str = cycle
            .iter()
            .map(|id| format!("#{}", id.0))
            .collect::<Vec<_>>()
            .join(" -> ");
        output.push_str(&format!(
            "{} -> #{}\n",
            cycle_str,
            cycle.first().map(|id| id.0).unwrap_or(0)
        ));
    }
    Ok(output.trim_end().to_string())
}

fn cmd_stats(tasks: &TaskManager) -> CmdResult {
    let status = tasks.status().map_err(|e| e.to_string())?;

    let mut output = String::from("Task Statistics\n\n");
    output.push_str(&format!("  Total tasks:     {}\n", status.total_tasks));
    output.push_str(&format!("  Todo:            {}\n", status.todo_count));
    output.push_str(&format!("  In Progress:     {}\n", status.in_progress_count));
    output.push_str(&format!("  Done:            {}\n", status.done_count));
    output.push_str(&format!("  Blocked:         {}\n", status.blocked_count));
    output.push_str(&format!("  Cancelled:       {}\n", status.cancelled_count));
    output.push_str(&format!(
        "\n  Dependencies:    {}\n",
        status.total_dependencies
    ));

    if status.has_cycles {
        output.push_str("  Cycles:          Yes (run 'cycles' to see)\n");
    } else {
        output.push_str("  Cycles:          None\n");
    }

    Ok(output.trim_end().to_string())
}
