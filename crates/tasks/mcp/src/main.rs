//! MCP (Model Context Protocol) server for ADI Tasks.
//!
//! This provides an MCP interface to the task management system,
//! allowing LLMs to create, manage, and query tasks through the MCP protocol.

use tasks_core::{CreateTask, TaskId, TaskManager, TaskStatus};
use lib_mcp_core::{
    server::{McpRouter, McpServerBuilder},
    transport::stdio::StdioTransport,
    CallToolResult, Tool, ToolInputSchema,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use lib_env_parse::{env_vars, env_opt};

env_vars! {
    ProjectPath => "PROJECT_PATH",
}

/// Shared state for the MCP server.
struct TasksState {
    manager: RwLock<Option<TaskManager>>,
    #[allow(dead_code)]
    project_path: Option<PathBuf>,
}

impl TasksState {
    fn new(project_path: Option<PathBuf>) -> Self {
        let manager = if let Some(ref path) = project_path {
            TaskManager::open(path).ok()
        } else {
            TaskManager::open_global().ok()
        };

        Self {
            manager: RwLock::new(manager),
            project_path,
        }
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

    // Get project path from environment
    let project_path = env_opt(EnvVar::ProjectPath.as_str()).map(PathBuf::from);

    // Create shared state
    let state = Arc::new(TasksState::new(project_path));

    // Build the MCP server with all tools
    let server = build_server(state);

    // Run with stdio transport
    let mut router = McpRouter::new(server);
    router.run(StdioTransport::new()).await
}

fn build_server(state: Arc<TasksState>) -> impl lib_mcp_core::server::McpHandler {
    let s = state.clone();
    let list_tasks = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let status_filter = args
                .get("status")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<TaskStatus>().ok());

            let tasks = if let Some(status) = status_filter {
                manager.get_by_status(status)
            } else {
                manager.list()
            }
            .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(serde_json::to_string_pretty(&tasks)?))
        }
    };

    let s = state.clone();
    let create_task = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("title is required".into()))?;

            let mut input = CreateTask::new(title);

            if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
                input = input.with_description(desc);
            }

            if let Some(deps) = args.get("depends_on").and_then(|v| v.as_array()) {
                let dep_ids: Vec<TaskId> =
                    deps.iter().filter_map(|v| v.as_i64()).map(TaskId).collect();
                input = input.with_dependencies(dep_ids);
            }

            let id = manager
                .create_task(input)
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(
                serde_json::json!({ "id": id.0, "message": "Task created successfully" })
                    .to_string(),
            ))
        }
    };

    let s = state.clone();
    let get_task = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("id is required".into()))?;

            let task = manager
                .get_task_with_dependencies(TaskId(id))
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(serde_json::to_string_pretty(&task)?))
        }
    };

    let s = state.clone();
    let update_task = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("id is required".into()))?;

            let mut task = manager
                .get_task(TaskId(id))
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
                task.title = title.to_string();
            }

            if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
                task.description = Some(desc.to_string());
            }

            if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
                task.status = status_str
                    .parse()
                    .map_err(|_| lib_mcp_core::Error::InvalidParams("Invalid status".into()))?;
            }

            manager
                .update_task(&task)
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(serde_json::to_string_pretty(&task)?))
        }
    };

    let s = state.clone();
    let update_status = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("id is required".into()))?;

            let status_str = args
                .get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("status is required".into()))?;

            let status: TaskStatus = status_str
                .parse()
                .map_err(|_| lib_mcp_core::Error::InvalidParams("Invalid status".into()))?;

            manager
                .update_status(TaskId(id), status)
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(
                serde_json::json!({
                    "id": id,
                    "status": status_str,
                    "message": "Status updated successfully"
                })
                .to_string(),
            ))
        }
    };

    let s = state.clone();
    let delete_task = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let id = args
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("id is required".into()))?;

            manager
                .delete_task(TaskId(id))
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(
                serde_json::json!({ "id": id, "message": "Task deleted successfully" }).to_string(),
            ))
        }
    };

    let s = state.clone();
    let add_dependency = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let task_id = args
                .get("task_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("task_id is required".into()))?;

            let depends_on_id = args
                .get("depends_on_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    lib_mcp_core::Error::InvalidParams("depends_on_id is required".into())
                })?;

            manager
                .add_dependency(TaskId(task_id), TaskId(depends_on_id))
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(
                serde_json::json!({
                    "task_id": task_id,
                    "depends_on_id": depends_on_id,
                    "message": "Dependency added successfully"
                })
                .to_string(),
            ))
        }
    };

    let s = state.clone();
    let remove_dependency = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let task_id = args
                .get("task_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("task_id is required".into()))?;

            let depends_on_id = args
                .get("depends_on_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    lib_mcp_core::Error::InvalidParams("depends_on_id is required".into())
                })?;

            manager
                .remove_dependency(TaskId(task_id), TaskId(depends_on_id))
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(
                serde_json::json!({
                    "task_id": task_id,
                    "depends_on_id": depends_on_id,
                    "message": "Dependency removed successfully"
                })
                .to_string(),
            ))
        }
    };

    let s = state.clone();
    let search_tasks = move |args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| lib_mcp_core::Error::InvalidParams("query is required".into()))?;

            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(10);

            let tasks = manager
                .search(query, limit)
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(serde_json::to_string_pretty(&tasks)?))
        }
    };

    let s = state.clone();
    let get_ready_tasks = move |_args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let tasks = manager
                .get_ready()
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(serde_json::to_string_pretty(&tasks)?))
        }
    };

    let s = state.clone();
    let get_blocked_tasks = move |_args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let tasks = manager
                .get_blocked()
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(serde_json::to_string_pretty(&tasks)?))
        }
    };

    let s = state.clone();
    let get_status = move |_args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let status = manager
                .status()
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            Ok(CallToolResult::text(serde_json::to_string_pretty(&status)?))
        }
    };

    let s = state.clone();
    let detect_cycles = move |_args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let cycles = manager
                .detect_cycles()
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            let cycle_ids: Vec<Vec<i64>> = cycles
                .into_iter()
                .map(|c| c.into_iter().map(|id| id.0).collect())
                .collect();

            Ok(CallToolResult::text(
                serde_json::json!({
                    "cycles": cycle_ids,
                    "has_cycles": !cycle_ids.is_empty()
                })
                .to_string(),
            ))
        }
    };

    let s = state.clone();
    let get_graph = move |_args: std::collections::HashMap<String, serde_json::Value>| {
        let state = s.clone();
        async move {
            let manager = state.manager.read().await;
            let manager = manager
                .as_ref()
                .ok_or_else(|| lib_mcp_core::Error::Internal("Tasks not initialized".into()))?;

            let all_tasks = manager
                .list()
                .map_err(|e| lib_mcp_core::Error::Internal(e.to_string()))?;

            let mut graph = Vec::new();
            for task in all_tasks {
                let deps = manager
                    .get_dependencies(task.id)
                    .unwrap_or_default()
                    .iter()
                    .map(|d| d.id.0)
                    .collect::<Vec<_>>();

                graph.push(serde_json::json!({
                    "task": task,
                    "dependencies": deps
                }));
            }

            Ok(CallToolResult::text(serde_json::to_string_pretty(&graph)?))
        }
    };

    McpServerBuilder::new("adi-tasks-mcp", env!("CARGO_PKG_VERSION"))
        .instructions(
            "ADI Tasks MCP Server - A task management system with dependency graph support. \
             Use these tools to create, manage, and query tasks. Tasks can have dependencies \
             on other tasks, and the system will prevent circular dependencies. \
             Status values: todo, in_progress, done, blocked, cancelled.",
        )
        // List tasks
        .tool(
            Tool::new(
                "list_tasks",
                ToolInputSchema::new().string_property(
                    "status",
                    "Filter by status (todo, in_progress, done, blocked, cancelled)",
                    false,
                ),
            )
            .with_description("List all tasks, optionally filtered by status"),
            list_tasks,
        )
        // Create task
        .tool(
            Tool::new(
                "create_task",
                ToolInputSchema::new()
                    .string_property("title", "Task title (required)", true)
                    .string_property("description", "Task description", false)
                    .property(
                        "depends_on",
                        serde_json::json!({
                            "type": "array",
                            "description": "Array of task IDs this task depends on",
                            "items": { "type": "integer" }
                        }),
                        false,
                    ),
            )
            .with_description("Create a new task with optional description and dependencies"),
            create_task,
        )
        // Get task
        .tool(
            Tool::new(
                "get_task",
                ToolInputSchema::new().integer_property("id", "Task ID (required)", true),
            )
            .with_description("Get a task by ID with its dependencies and dependents"),
            get_task,
        )
        // Update task
        .tool(
            Tool::new(
                "update_task",
                ToolInputSchema::new()
                    .integer_property("id", "Task ID (required)", true)
                    .string_property("title", "New task title", false)
                    .string_property("description", "New task description", false)
                    .string_property(
                        "status",
                        "New status (todo, in_progress, done, blocked, cancelled)",
                        false,
                    ),
            )
            .with_description("Update a task's title, description, or status"),
            update_task,
        )
        // Update status
        .tool(
            Tool::new(
                "update_status",
                ToolInputSchema::new()
                    .integer_property("id", "Task ID (required)", true)
                    .string_property(
                        "status",
                        "New status (todo, in_progress, done, blocked, cancelled) (required)",
                        true,
                    ),
            )
            .with_description("Update only the status of a task"),
            update_status,
        )
        // Delete task
        .tool(
            Tool::new(
                "delete_task",
                ToolInputSchema::new().integer_property("id", "Task ID to delete (required)", true),
            )
            .with_description("Delete a task by ID"),
            delete_task,
        )
        // Add dependency
        .tool(
            Tool::new(
                "add_dependency",
                ToolInputSchema::new()
                    .integer_property(
                        "task_id",
                        "The task that will depend on another (required)",
                        true,
                    )
                    .integer_property(
                        "depends_on_id",
                        "The task that must be completed first (required)",
                        true,
                    ),
            )
            .with_description("Add a dependency between tasks (task_id depends on depends_on_id)"),
            add_dependency,
        )
        // Remove dependency
        .tool(
            Tool::new(
                "remove_dependency",
                ToolInputSchema::new()
                    .integer_property(
                        "task_id",
                        "The task to remove dependency from (required)",
                        true,
                    )
                    .integer_property("depends_on_id", "The dependency to remove (required)", true),
            )
            .with_description("Remove a dependency between tasks"),
            remove_dependency,
        )
        // Search tasks
        .tool(
            Tool::new(
                "search_tasks",
                ToolInputSchema::new()
                    .string_property(
                        "query",
                        "Search query for full-text search (required)",
                        true,
                    )
                    .integer_property("limit", "Maximum number of results (default: 10)", false),
            )
            .with_description("Search tasks using full-text search on title and description"),
            search_tasks,
        )
        // Get ready tasks
        .tool(
            Tool::new("get_ready_tasks", ToolInputSchema::new()).with_description(
                "Get tasks that are ready to work on (no incomplete dependencies)",
            ),
            get_ready_tasks,
        )
        // Get blocked tasks
        .tool(
            Tool::new("get_blocked_tasks", ToolInputSchema::new())
                .with_description("Get tasks that are blocked by incomplete dependencies"),
            get_blocked_tasks,
        )
        // Get status
        .tool(
            Tool::new("get_status", ToolInputSchema::new()).with_description(
                "Get statistics about all tasks (counts by status, dependency info)",
            ),
            get_status,
        )
        // Detect cycles
        .tool(
            Tool::new("detect_cycles", ToolInputSchema::new())
                .with_description("Detect circular dependencies in the task graph"),
            detect_cycles,
        )
        // Get graph
        .tool(
            Tool::new("get_graph", ToolInputSchema::new())
                .with_description("Get the full task dependency graph"),
            get_graph,
        )
        .with_logging()
        .build()
}
