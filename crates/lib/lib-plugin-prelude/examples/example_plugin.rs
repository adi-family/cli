//! Example plugin demonstrating the new SDK
//!
//! This example shows how to write a plugin using the simplified SDK macros.

use lib_plugin_prelude::*;

/// Example database type
struct Database;

impl Database {
    fn open(_path: std::path::PathBuf) -> std::io::Result<Self> {
        Ok(Database)
    }

    fn list(&self, _status: Option<String>) -> std::io::Result<Vec<Task>> {
        Ok(vec![
            Task { id: 1, title: "Task 1".into(), status: "todo".into() },
            Task { id: 2, title: "Task 2".into(), status: "done".into() },
        ])
    }
}

impl Default for Database {
    fn default() -> Self {
        Database
    }
}

struct Task {
    id: i64,
    title: String,
    status: String,
}

// === Plugin Definition ===

/// Example Tasks Plugin using the SDK
#[plugin]
pub struct TasksPlugin {
    db: Database,
}

// Manual Plugin trait implementation (required)
#[async_trait]
impl Plugin for TasksPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.example", "Example Plugin", "1.0.0")
            .with_description("Example plugin demonstrating SDK usage")
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        self.db = Database::open(ctx.data_dir.clone()).map_err(|e| PluginError::InitFailed(e.to_string()))?;
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS]
    }
}

// === CLI Commands ===

#[async_trait]
impl CliCommands for TasksPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            Self::__sdk_cmd_meta_list_tasks(),
            Self::__sdk_cmd_meta_add_task(),
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("list") => {
                let status: Option<String> = ctx.option("status");
                self.list_tasks(status).await
                    .map(|s| CliResult::success(s))
                    .map_err(|e| PluginError::CommandFailed(e))
            }
            Some("add") => {
                let title = ctx.arg(0)
                    .ok_or_else(|| PluginError::CommandFailed("Missing title".to_string()))?
                    .to_string();
                self.add_task(title).await
                    .map(|s| CliResult::success(s))
                    .map_err(|e| PluginError::CommandFailed(e))
            }
            _ => Ok(CliResult::error("Unknown command"))
        }
    }
}

impl TasksPlugin {
    /// List all tasks
    #[command(name = "list", description = "List all tasks")]
    async fn list_tasks(&self, status: Option<String>) -> CmdResult {
        let tasks = self.db.list(status).map_err(|e| e.to_string())?;
        
        let output: String = tasks
            .iter()
            .map(|t| format!("#{} {} [{}]", t.id, t.title, t.status))
            .collect::<Vec<_>>()
            .join("\n");
        
        Ok(output)
    }

    /// Add a new task
    #[command(name = "add", description = "Add a new task")]
    async fn add_task(&self, title: String) -> CmdResult {
        // In real implementation, this would save to database
        Ok(format!("Created task: {}", title))
    }
}

fn main() {
    // This file is for documentation/example purposes
    println!("See the source code for example plugin implementation");
}
