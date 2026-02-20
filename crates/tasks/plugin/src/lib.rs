use lib_plugin_prelude::*;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use tasks_core::{CreateTask, TaskId, TaskManager, TaskStatus};

#[derive(CliArgs)]
pub struct ListArgs {
    #[arg(long)]
    pub status: Option<String>,

    #[arg(long)]
    pub ready: bool,

    #[arg(long)]
    pub blocked: bool,

    #[arg(long, default = "text".to_string())]
    pub format: String,
}

#[derive(CliArgs)]
pub struct AddArgs {
    #[arg(position = 0)]
    pub title: String,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(long = "depends-on")]
    pub depends_on: Option<String>,
}

#[derive(CliArgs)]
pub struct ShowArgs {
    #[arg(position = 0)]
    pub id: i64,
}

#[derive(CliArgs)]
pub struct StatusArgs {
    #[arg(position = 0)]
    pub id: i64,

    #[arg(position = 1)]
    pub status: String,
}

#[derive(CliArgs)]
pub struct DeleteArgs {
    #[arg(position = 0)]
    pub id: i64,

    #[arg(long)]
    pub force: bool,
}

#[derive(CliArgs)]
pub struct DependArgs {
    #[arg(position = 0)]
    pub task_id: i64,

    #[arg(position = 1)]
    pub depends_on: i64,
}

#[derive(CliArgs)]
pub struct UndependArgs {
    #[arg(position = 0)]
    pub task_id: i64,

    #[arg(position = 1)]
    pub depends_on: i64,
}

#[derive(CliArgs)]
pub struct GraphArgs {
    #[arg(long, default = "text".to_string())]
    pub format: String,
}

#[derive(CliArgs)]
pub struct SearchArgs {
    #[arg(position = 0)]
    pub query: String,

    #[arg(long, default = 10)]
    pub limit: i64,
}

pub struct TasksPlugin {
    tasks: Arc<RwLock<Option<TaskManager>>>,
}

impl TasksPlugin {
    pub fn new() -> Self {
        let manager = TaskManager::open_global().ok();
        Self {
            tasks: Arc::new(RwLock::new(manager)),
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
        PluginMetadata::new("adi.tasks", t!("plugin-name"), env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Core)
            .with_author("ADI Team")
            .with_description(t!("plugin-description"))
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
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
            Self::__sdk_cmd_meta_list(),
            Self::__sdk_cmd_meta_add(),
            Self::__sdk_cmd_meta_show(),
            Self::__sdk_cmd_meta_status(),
            Self::__sdk_cmd_meta_delete(),
            Self::__sdk_cmd_meta_depend(),
            Self::__sdk_cmd_meta_undepend(),
            Self::__sdk_cmd_meta_graph(),
            Self::__sdk_cmd_meta_search(),
            Self::__sdk_cmd_meta_blocked(),
            Self::__sdk_cmd_meta_cycles(),
            Self::__sdk_cmd_meta_stats(),
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("list") => self.__sdk_cmd_handler_list(ctx).await,
            Some("add") => self.__sdk_cmd_handler_add(ctx).await,
            Some("show") => self.__sdk_cmd_handler_show(ctx).await,
            Some("status") => self.__sdk_cmd_handler_status(ctx).await,
            Some("delete") => self.__sdk_cmd_handler_delete(ctx).await,
            Some("depend") => self.__sdk_cmd_handler_depend(ctx).await,
            Some("undepend") => self.__sdk_cmd_handler_undepend(ctx).await,
            Some("graph") => self.__sdk_cmd_handler_graph(ctx).await,
            Some("search") => self.__sdk_cmd_handler_search(ctx).await,
            Some("blocked") => self.__sdk_cmd_handler_blocked(ctx).await,
            Some("cycles") => self.__sdk_cmd_handler_cycles(ctx).await,
            Some("stats") => self.__sdk_cmd_handler_stats(ctx).await,
            Some(cmd) => Ok(CliResult::error(format!("Unknown command: {}", cmd))),
            None => Ok(CliResult::success(self.help())),
        }
    }
}

fn scope_label(task: &tasks_core::Task) -> String {
    if task.is_global() {
        t!("tasks-list-scope-global")
    } else {
        t!("tasks-list-scope-project")
    }
}

impl TasksPlugin {
    async fn manager(&self) -> std::result::Result<tokio::sync::RwLockReadGuard<'_, Option<TaskManager>>, String> {
        let guard = self.tasks.read().await;
        if guard.is_none() {
            return Err(t!("error-not-initialized"));
        }
        Ok(guard)
    }

    fn help(&self) -> String {
        format!(
            "{}\n\n{}\n  \
             list     {}\n  \
             add      {}\n  \
             show     {}\n  \
             status   {}\n  \
             delete   {}\n  \
             depend   {}\n  \
             undepend {}\n  \
             graph    {}\n  \
             search   {}\n  \
             blocked  {}\n  \
             cycles   {}\n  \
             stats    {}\n\n\
             {}",
            t!("tasks-help-title"),
            t!("tasks-help-commands"),
            t!("cmd-list-help"),
            t!("cmd-add-help"),
            t!("cmd-show-help"),
            t!("cmd-status-help"),
            t!("cmd-delete-help"),
            t!("cmd-depend-help"),
            t!("cmd-undepend-help"),
            t!("cmd-graph-help"),
            t!("cmd-search-help"),
            t!("cmd-blocked-help"),
            t!("cmd-cycles-help"),
            t!("cmd-stats-help"),
            t!("tasks-help-usage"),
        )
    }

    #[command(name = "list", description = "cmd-list-help")]
    async fn list(&self, args: ListArgs) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();

        let task_list = if args.ready {
            tasks.get_ready().map_err(|e| e.to_string())?
        } else if args.blocked {
            tasks.get_blocked().map_err(|e| e.to_string())?
        } else if let Some(ref status_str) = args.status {
            let status: TaskStatus = status_str.parse().map_err(|_| {
                t!("tasks-status-invalid-status", "status" => status_str.as_str())
            })?;
            tasks.get_by_status(status).map_err(|e| e.to_string())?
        } else {
            tasks.list().map_err(|e| e.to_string())?
        };

        if args.format == "json" {
            return serde_json::to_string_pretty(&task_list).map_err(|e| e.to_string());
        }

        if task_list.is_empty() {
            return Ok(t!("tasks-list-empty"));
        }

        let mut output = String::new();
        for task in task_list {
            let scope = scope_label(&task);
            output.push_str(&format!("{} #{} {} {}\n", task.status.icon(), task.id.get(), task.title, scope));
        }
        Ok(output.trim_end().to_string())
    }

    #[command(name = "add", description = "cmd-add-help")]
    async fn add(&self, args: AddArgs) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();

        let depends_on_ids: Vec<i64> = args
            .depends_on
            .map(|s| s.split(',').filter_map(|id| id.trim().parse().ok()).collect())
            .unwrap_or_default();

        let mut input = CreateTask::new(&args.title);
        if let Some(desc) = args.description {
            input = input.with_description(desc);
        }
        if !depends_on_ids.is_empty() {
            input = input.with_dependencies(depends_on_ids.into_iter().map(TaskId::new).collect());
        }

        let id = tasks.create_task(input).map_err(|e| e.to_string())?;
        Ok(t!("tasks-add-created", "id" => id.get().to_string(), "title" => args.title.as_str()))
    }

    #[command(name = "show", description = "cmd-show-help")]
    async fn show(&self, args: ShowArgs) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();

        let task_with_deps = tasks.get_task_with_dependencies(TaskId::new(args.id)).map_err(|e| e.to_string())?;
        let task = &task_with_deps.task;

        let mut output = format!("{}\n", t!("tasks-show-title", "id" => task.id.get().to_string()));
        output.push_str(&format!("  {}\n", t!("tasks-show-field-title", "title" => task.title.as_str())));
        output.push_str(&format!("  {}\n", t!("tasks-show-field-status", "status" => format!("{:?}", task.status))));

        if let Some(ref desc) = task.description {
            output.push_str(&format!("  {}\n", t!("tasks-show-field-description", "description" => desc.as_str())));
        }
        if let Some(symbol_id) = task.symbol_id {
            output.push_str(&format!("  {}\n", t!("tasks-show-field-symbol", "symbol_id" => symbol_id.to_string())));
        }

        let scope = if task.is_global() { "global" } else { "project" };
        output.push_str(&format!("  {}\n", t!("tasks-show-field-scope", "scope" => scope)));

        if !task_with_deps.depends_on.is_empty() {
            output.push_str(&format!("\n  {}\n", t!("tasks-show-dependencies")));
            for dep in &task_with_deps.depends_on {
                output.push_str(&format!("    #{}: {}\n", dep.id.get(), dep.title));
            }
        }
        if !task_with_deps.dependents.is_empty() {
            output.push_str(&format!("\n  {}\n", t!("tasks-show-dependents")));
            for dep in &task_with_deps.dependents {
                output.push_str(&format!("    #{}: {}\n", dep.id.get(), dep.title));
            }
        }

        Ok(output.trim_end().to_string())
    }

    #[command(name = "status", description = "cmd-status-help")]
    async fn status(&self, args: StatusArgs) -> CmdResult {
        let status: TaskStatus = args.status.parse().map_err(|_| {
            t!("tasks-status-invalid-status", "status" => args.status.as_str())
        })?;

        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();
        tasks.update_status(TaskId::new(args.id), status).map_err(|e| e.to_string())?;
        Ok(t!("tasks-status-updated", "id" => args.id.to_string(), "status" => status.to_string()))
    }

    #[command(name = "delete", description = "cmd-delete-help")]
    async fn delete(&self, args: DeleteArgs) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();

        let task = tasks.get_task(TaskId::new(args.id)).map_err(|e| e.to_string())?;

        if !args.force {
            return Ok(format!(
                "{}\n{}",
                t!("tasks-delete-confirm", "id" => args.id.to_string(), "title" => task.title.as_str()),
                t!("tasks-delete-confirm-hint")
            ));
        }

        tasks.delete_task(TaskId::new(args.id)).map_err(|e| e.to_string())?;
        Ok(t!("tasks-delete-success", "id" => args.id.to_string(), "title" => task.title.as_str()))
    }

    #[command(name = "depend", description = "cmd-depend-help")]
    async fn depend(&self, args: DependArgs) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();
        tasks.add_dependency(TaskId::new(args.task_id), TaskId::new(args.depends_on)).map_err(|e| e.to_string())?;
        Ok(t!("tasks-depend-success", "task_id" => args.task_id.to_string(), "depends_on" => args.depends_on.to_string()))
    }

    #[command(name = "undepend", description = "cmd-undepend-help")]
    async fn undepend(&self, args: UndependArgs) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();
        tasks.remove_dependency(TaskId::new(args.task_id), TaskId::new(args.depends_on)).map_err(|e| e.to_string())?;
        Ok(t!("tasks-undepend-success", "task_id" => args.task_id.to_string(), "depends_on" => args.depends_on.to_string()))
    }

    #[command(name = "graph", description = "cmd-graph-help")]
    async fn graph(&self, args: GraphArgs) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();
        let all_tasks = tasks.list().map_err(|e| e.to_string())?;

        if args.format == "json" {
            let mut graph_data = Vec::new();
            for task in &all_tasks {
                let deps = tasks.get_dependencies(task.id).map_err(|e| e.to_string())?;
                graph_data.push(json!({
                    "task": task,
                    "dependencies": deps.iter().map(|d| d.id.get()).collect::<Vec<_>>()
                }));
            }
            return serde_json::to_string_pretty(&graph_data).map_err(|e| e.to_string());
        }

        if args.format == "dot" {
            let mut output = String::from("digraph tasks {\n  rankdir=LR;\n");
            for task in &all_tasks {
                let label = task.title.replace('"', "\\\"");
                output.push_str(&format!("  {} [label=\"{}\" color=\"{}\"];\n", task.id.get(), label, task.status.color()));

                let deps = tasks.get_dependencies(task.id).map_err(|e| e.to_string())?;
                for dep in deps {
                    output.push_str(&format!("  {} -> {};\n", task.id.get(), dep.id.get()));
                }
            }
            output.push_str("}\n");
            return Ok(output);
        }

        // Text format
        if all_tasks.is_empty() {
            return Ok(t!("tasks-graph-empty"));
        }

        let mut output = format!("{}\n\n", t!("tasks-graph-title"));
        for task in &all_tasks {
            output.push_str(&format!("{} #{} {}\n", task.status.icon(), task.id.get(), task.title));

            let deps = tasks.get_dependencies(task.id).map_err(|e| e.to_string())?;
            for (i, dep) in deps.iter().enumerate() {
                let prefix = if i == deps.len() - 1 { "  └─" } else { "  ├─" };
                output.push_str(&format!("{} {}\n", prefix, t!("tasks-graph-depends-on", "id" => dep.id.get().to_string(), "title" => dep.title.as_str())));
            }
        }
        Ok(output.trim_end().to_string())
    }

    #[command(name = "search", description = "cmd-search-help")]
    async fn search(&self, args: SearchArgs) -> CmdResult {
        let limit = args.limit as usize;
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();

        let results = tasks.search(&args.query, limit).map_err(|e| e.to_string())?;

        if results.is_empty() {
            return Ok(t!("tasks-search-empty"));
        }

        let mut output = format!("{}\n\n", t!("tasks-search-results", "count" => results.len().to_string(), "query" => args.query.as_str()));
        for task in results {
            output.push_str(&format!("{} #{} {}\n", task.status.icon(), task.id.get(), task.title));
        }
        Ok(output.trim_end().to_string())
    }

    #[command(name = "blocked", description = "cmd-blocked-help")]
    async fn blocked(&self) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();
        let blocked = tasks.get_blocked().map_err(|e| e.to_string())?;

        if blocked.is_empty() {
            return Ok(t!("tasks-blocked-empty"));
        }

        let mut output = format!("{}\n\n", t!("tasks-blocked-title"));
        for task in blocked {
            output.push_str(&format!("✕ #{} {}\n", task.id.get(), task.title));

            let blockers = tasks.get_dependencies(task.id).map_err(|e| e.to_string())?;
            let incomplete_blockers: Vec<_> = blockers.iter().filter(|t| !t.status.is_complete()).collect();

            for blocker in incomplete_blockers {
                output.push_str(&format!("  └─ {}\n", t!("tasks-blocked-by", 
                    "id" => blocker.id.get().to_string(), 
                    "title" => blocker.title.as_str(), 
                    "status" => format!("{:?}", blocker.status)
                )));
            }
        }
        Ok(output.trim_end().to_string())
    }

    #[command(name = "cycles", description = "cmd-cycles-help")]
    async fn cycles(&self) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();
        let cycles = tasks.detect_cycles().map_err(|e| e.to_string())?;

        if cycles.is_empty() {
            return Ok(t!("tasks-cycles-empty"));
        }

        let mut output = format!("{}\n\n", t!("tasks-cycles-found", "count" => cycles.len().to_string()));
        for (i, cycle) in cycles.iter().enumerate() {
            output.push_str(&format!("  {} ", t!("tasks-cycles-item", "number" => (i + 1).to_string())));
            let cycle_str = cycle.iter().map(|id| format!("#{}", id.get())).collect::<Vec<_>>().join(" -> ");
            output.push_str(&format!("{} -> #{}\n", cycle_str, cycle.first().map(|id| id.get()).unwrap_or(0)));
        }
        Ok(output.trim_end().to_string())
    }

    #[command(name = "stats", description = "cmd-stats-help")]
    async fn stats(&self) -> CmdResult {
        let guard = self.manager().await?;
        let tasks = guard.as_ref().unwrap();
        let status = tasks.status().map_err(|e| e.to_string())?;

        let mut output = format!("{}\n\n", t!("tasks-stats-title"));
        output.push_str(&format!("  {}\n", t!("tasks-stats-total", "count" => status.total_tasks.to_string())));
        output.push_str(&format!("  {}\n", t!("tasks-stats-todo", "count" => status.todo_count.to_string())));
        output.push_str(&format!("  {}\n", t!("tasks-stats-in-progress", "count" => status.in_progress_count.to_string())));
        output.push_str(&format!("  {}\n", t!("tasks-stats-done", "count" => status.done_count.to_string())));
        output.push_str(&format!("  {}\n", t!("tasks-stats-blocked", "count" => status.blocked_count.to_string())));
        output.push_str(&format!("  {}\n", t!("tasks-stats-cancelled", "count" => status.cancelled_count.to_string())));
        output.push_str(&format!("\n  {}\n", t!("tasks-stats-dependencies", "count" => status.total_dependencies.to_string())));

        if status.has_cycles {
            output.push_str(&format!("  {}\n", t!("tasks-stats-cycles-yes")));
        } else {
            output.push_str(&format!("  {}\n", t!("tasks-stats-cycles-no")));
        }

        Ok(output.trim_end().to_string())
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(TasksPlugin::new())
}
