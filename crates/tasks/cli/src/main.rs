use std::path::PathBuf;

use tasks_core::{CreateTask, TaskId, TaskManager, TaskStatus};
use clap::{Parser, Subcommand};
use console::{style, Style};
use lib_cli_common::{print_empty, print_error, print_success, print_warning, OutputFormat};

#[derive(Parser)]
#[command(name = "adi-tasks-cli")]
#[command(about = "ADI Tasks CLI - Task management with dependency graphs")]
#[command(version)]
struct Cli {
    /// Project directory (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    project: PathBuf,

    /// Use global tasks only (no project context)
    #[arg(long)]
    global: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize tasks in the current project
    Init,

    /// Add a new task
    Add {
        /// Task title
        title: String,

        /// Task description
        #[arg(short, long)]
        description: Option<String>,

        /// Task IDs this task depends on
        #[arg(long, value_delimiter = ',')]
        depends_on: Vec<i64>,

        /// Link to a code symbol ID
        #[arg(long)]
        symbol: Option<i64>,
    },

    /// List tasks
    List {
        /// Filter by status (todo, in_progress, done, blocked, cancelled)
        #[arg(long)]
        status: Option<String>,

        /// Show only tasks that are ready to work on
        #[arg(long)]
        ready: bool,

        /// Show only blocked tasks
        #[arg(long)]
        blocked: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show task details
    Show {
        /// Task ID
        id: i64,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Update task status
    Status {
        /// Task ID
        id: i64,

        /// New status (todo, in_progress, done, blocked, cancelled)
        status: String,
    },

    /// Add a dependency between tasks
    Depend {
        /// Task ID that will depend on another
        task_id: i64,

        /// Task ID to depend on (must complete first)
        depends_on: i64,
    },

    /// Remove a dependency between tasks
    Undepend {
        /// Task ID
        task_id: i64,

        /// Dependency to remove
        depends_on: i64,
    },

    /// Show dependency graph
    Graph {
        /// Root task ID (show subgraph from this task)
        #[arg(long)]
        root: Option<i64>,

        /// Output format (text, dot, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Search tasks
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Link a task to a code symbol
    Link {
        /// Task ID
        task_id: i64,

        /// Symbol ID from indexer
        symbol_id: i64,
    },

    /// Unlink a task from its code symbol
    Unlink {
        /// Task ID
        task_id: i64,
    },

    /// Delete a task
    Delete {
        /// Task ID
        id: i64,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Show blocked tasks and their blockers
    Blocked,

    /// Detect cycles in dependency graph
    Cycles,

    /// Show task statistics
    Stats,
}

fn main() {
    lib_cli_common::setup_logging_quiet();

    if let Err(e) = run() {
        print_error(&format!("{}", e));
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let manager = if cli.global {
        TaskManager::open_global()?
    } else {
        let project_path = std::fs::canonicalize(&cli.project)?;
        TaskManager::open(&project_path)?
    };

    match cli.command {
        Commands::Init => cmd_init(&manager)?,
        Commands::Add {
            title,
            description,
            depends_on,
            symbol,
        } => cmd_add(&manager, title, description, depends_on, symbol)?,
        Commands::List {
            status,
            ready,
            blocked,
            format,
        } => cmd_list(&manager, status, ready, blocked, &format)?,
        Commands::Show { id, format } => cmd_show(&manager, TaskId(id), &format)?,
        Commands::Status { id, status } => cmd_status(&manager, TaskId(id), &status)?,
        Commands::Depend {
            task_id,
            depends_on,
        } => cmd_depend(&manager, TaskId(task_id), TaskId(depends_on))?,
        Commands::Undepend {
            task_id,
            depends_on,
        } => cmd_undepend(&manager, TaskId(task_id), TaskId(depends_on))?,
        Commands::Graph { root, format } => cmd_graph(&manager, root.map(TaskId), &format)?,
        Commands::Search {
            query,
            limit,
            format,
        } => cmd_search(&manager, &query, limit, &format)?,
        Commands::Link { task_id, symbol_id } => cmd_link(&manager, TaskId(task_id), symbol_id)?,
        Commands::Unlink { task_id } => cmd_unlink(&manager, TaskId(task_id))?,
        Commands::Delete { id, force } => cmd_delete(&manager, TaskId(id), force)?,
        Commands::Blocked => cmd_blocked(&manager)?,
        Commands::Cycles => cmd_cycles(&manager)?,
        Commands::Stats => cmd_stats(&manager)?,
    }

    Ok(())
}

fn cmd_init(manager: &TaskManager) -> anyhow::Result<()> {
    if manager.is_global() {
        print_success("Global tasks initialized");
    } else {
        print_success("Tasks initialized in project");
    }
    Ok(())
}

fn cmd_add(
    manager: &TaskManager,
    title: String,
    description: Option<String>,
    depends_on: Vec<i64>,
    symbol: Option<i64>,
) -> anyhow::Result<()> {
    let mut input = CreateTask::new(&title);

    if let Some(desc) = description {
        input = input.with_description(desc);
    }

    if !depends_on.is_empty() {
        input = input.with_dependencies(depends_on.into_iter().map(TaskId).collect());
    }

    input.symbol_id = symbol;

    let id = manager.create_task(input)?;

    let scope = if manager.is_global() {
        "global"
    } else {
        "project"
    };
    print_success(&format!(
        "Created {} task #{}: {}",
        scope,
        style(id.0).cyan(),
        title
    ));

    Ok(())
}

fn cmd_list(
    manager: &TaskManager,
    status_filter: Option<String>,
    ready: bool,
    blocked: bool,
    format: &str,
) -> anyhow::Result<()> {
    let task_list = if ready {
        manager.get_ready()?
    } else if blocked {
        manager.get_blocked()?
    } else if let Some(status_str) = status_filter {
        let status: TaskStatus = status_str
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid status: {}", status_str))?;
        manager.get_by_status(status)?
    } else {
        manager.list()?
    };

    let format = OutputFormat::from(format);
    if format.print_if_json(&task_list)? {
        return Ok(());
    }

    if task_list.is_empty() {
        print_empty("No tasks found");
        return Ok(());
    }

    for task in task_list {
        print_task_line(&task);
    }

    Ok(())
}

fn cmd_show(manager: &TaskManager, id: TaskId, format: &str) -> anyhow::Result<()> {
    let task_with_deps = manager.get_task_with_dependencies(id)?;

    let format = OutputFormat::from(format);
    if format.print_if_json(&task_with_deps)? {
        return Ok(());
    }

    let task = &task_with_deps.task;

    println!("{}", style(format!("Task #{}", task.id.0)).bold());
    println!("  Title: {}", task.title);
    println!("  Status: {}", format_status(task.status));

    if let Some(ref desc) = task.description {
        println!("  Description: {}", desc);
    }

    if let Some(symbol_id) = task.symbol_id {
        println!("  Linked symbol: #{}", symbol_id);
    }

    if task.is_global() {
        println!("  Scope: {}", style("global").yellow());
    } else {
        println!(
            "  Scope: {}",
            style(task.project_path.as_deref().unwrap_or("project")).cyan()
        );
    }

    if !task_with_deps.depends_on.is_empty() {
        println!("\n  {} Dependencies:", style("→").dim());
        for dep in &task_with_deps.depends_on {
            print!("    ");
            print_task_line(dep);
        }
    }

    if !task_with_deps.dependents.is_empty() {
        println!("\n  {} Dependents:", style("←").dim());
        for dep in &task_with_deps.dependents {
            print!("    ");
            print_task_line(dep);
        }
    }

    Ok(())
}

fn cmd_status(manager: &TaskManager, id: TaskId, status_str: &str) -> anyhow::Result<()> {
    let status: TaskStatus = status_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid status: {}", status_str))?;

    manager.update_status(id, status)?;

    print_success(&format!(
        "Task #{} status updated to {}",
        id.0,
        format_status(status)
    ));

    Ok(())
}

fn cmd_depend(manager: &TaskManager, task_id: TaskId, depends_on: TaskId) -> anyhow::Result<()> {
    manager.add_dependency(task_id, depends_on)?;

    print_success(&format!(
        "Task #{} now depends on task #{}",
        task_id.0, depends_on.0
    ));

    Ok(())
}

fn cmd_undepend(manager: &TaskManager, task_id: TaskId, depends_on: TaskId) -> anyhow::Result<()> {
    manager.remove_dependency(task_id, depends_on)?;

    print_success(&format!(
        "Removed dependency: #{} → #{}",
        task_id.0, depends_on.0
    ));

    Ok(())
}

fn cmd_graph(manager: &TaskManager, _root: Option<TaskId>, format: &str) -> anyhow::Result<()> {
    let all_tasks = manager.list()?;

    if format == "json" {
        let mut graph_data = Vec::new();
        for task in &all_tasks {
            let deps = manager.get_dependencies(task.id)?;
            graph_data.push(serde_json::json!({
                "task": task,
                "dependencies": deps.iter().map(|d| d.id.0).collect::<Vec<_>>()
            }));
        }
        println!("{}", serde_json::to_string_pretty(&graph_data)?);
        return Ok(());
    }

    if format == "dot" {
        println!("digraph tasks {{");
        println!("  rankdir=LR;");
        for task in &all_tasks {
            let label = task.title.replace('"', "\\\"");
            let color = match task.status {
                TaskStatus::Done => "green",
                TaskStatus::InProgress => "blue",
                TaskStatus::Blocked => "red",
                TaskStatus::Cancelled => "gray",
                TaskStatus::Todo => "black",
            };
            println!("  {} [label=\"{}\" color=\"{}\"];", task.id.0, label, color);

            let deps = manager.get_dependencies(task.id)?;
            for dep in deps {
                println!("  {} -> {};", task.id.0, dep.id.0);
            }
        }
        println!("}}");
        return Ok(());
    }

    // Text format - tree view
    if all_tasks.is_empty() {
        print_empty("No tasks found");
        return Ok(());
    }

    println!("{}", style("Task Dependency Graph").bold());
    println!();

    for task in &all_tasks {
        let deps = manager.get_dependencies(task.id)?;
        print_task_line(task);

        if !deps.is_empty() {
            for (i, dep) in deps.iter().enumerate() {
                let prefix = if i == deps.len() - 1 {
                    "  └─"
                } else {
                    "  ├─"
                };
                println!(
                    "{} {} #{}: {}",
                    style(prefix).dim(),
                    style("depends on").dim(),
                    dep.id.0,
                    dep.title
                );
            }
        }
    }

    Ok(())
}

fn cmd_search(
    manager: &TaskManager,
    query: &str,
    limit: usize,
    format: &str,
) -> anyhow::Result<()> {
    let results = manager.search(query, limit)?;

    let format = OutputFormat::from(format);
    if format.print_if_json(&results)? {
        return Ok(());
    }

    if results.is_empty() {
        print_empty("No tasks found");
        return Ok(());
    }

    println!(
        "{} {} results for \"{}\":",
        style("Found").dim(),
        results.len(),
        query
    );
    println!();

    for task in results {
        print_task_line(&task);
    }

    Ok(())
}

fn cmd_link(manager: &TaskManager, task_id: TaskId, symbol_id: i64) -> anyhow::Result<()> {
    manager.link_to_symbol(task_id, symbol_id)?;

    print_success(&format!(
        "Task #{} linked to symbol #{}",
        task_id.0, symbol_id
    ));

    Ok(())
}

fn cmd_unlink(manager: &TaskManager, task_id: TaskId) -> anyhow::Result<()> {
    manager.unlink_symbol(task_id)?;

    print_success(&format!("Task #{} unlinked from symbol", task_id.0));

    Ok(())
}

fn cmd_delete(manager: &TaskManager, id: TaskId, force: bool) -> anyhow::Result<()> {
    let task = manager.get_task(id)?;

    if !force {
        print_warning(&format!("Delete task #{}: {}?", id.0, task.title));
        println!("  Use --force to confirm deletion");
        return Ok(());
    }

    manager.delete_task(id)?;

    print_success(&format!("Deleted task #{}: {}", id.0, task.title));

    Ok(())
}

fn cmd_blocked(manager: &TaskManager) -> anyhow::Result<()> {
    let blocked = manager.get_blocked()?;

    if blocked.is_empty() {
        print_empty("No blocked tasks");
        return Ok(());
    }

    println!("{}", style("Blocked Tasks").bold());
    println!();

    for task in blocked {
        print_task_line(&task);

        let blockers = manager.get_dependencies(task.id)?;
        let incomplete_blockers: Vec<_> = blockers
            .iter()
            .filter(|t| !t.status.is_complete())
            .collect();

        for blocker in incomplete_blockers {
            println!(
                "  {} blocked by #{}: {} ({})",
                style("└─").dim(),
                blocker.id.0,
                blocker.title,
                format_status(blocker.status)
            );
        }
    }

    Ok(())
}

fn cmd_cycles(manager: &TaskManager) -> anyhow::Result<()> {
    let cycles = manager.detect_cycles()?;

    if cycles.is_empty() {
        print_success("No circular dependencies detected");
        return Ok(());
    }

    println!(
        "{} Found {} circular dependencies:",
        style("!").red().bold(),
        cycles.len()
    );
    println!();

    for (i, cycle) in cycles.iter().enumerate() {
        println!("  Cycle {}: ", i + 1);
        let cycle_str = cycle
            .iter()
            .map(|id| format!("#{}", id.0))
            .collect::<Vec<_>>()
            .join(" → ");
        println!(
            "    {} → #{}",
            cycle_str,
            cycle.first().map(|id| id.0).unwrap_or(0)
        );
    }

    Ok(())
}

fn cmd_stats(manager: &TaskManager) -> anyhow::Result<()> {
    let status = manager.status()?;

    println!("{}", style("Task Statistics").bold());
    println!();
    println!("  Total tasks:     {}", status.total_tasks);
    println!("  Todo:            {}", style(status.todo_count).yellow());
    println!(
        "  In Progress:     {}",
        style(status.in_progress_count).blue()
    );
    println!("  Done:            {}", style(status.done_count).green());
    println!("  Blocked:         {}", style(status.blocked_count).red());
    println!("  Cancelled:       {}", style(status.cancelled_count).dim());
    println!();
    println!("  Dependencies:    {}", status.total_dependencies);

    if status.has_cycles {
        println!(
            "  Cycles:          {}",
            style("Yes (run 'cycles' to see)").red()
        );
    } else {
        println!("  Cycles:          {}", style("None").green());
    }

    Ok(())
}

// --- Helper Functions ---

fn print_task_line(task: &tasks_core::Task) {
    let status_indicator = match task.status {
        TaskStatus::Todo => style("○").white(),
        TaskStatus::InProgress => style("◐").blue(),
        TaskStatus::Done => style("●").green(),
        TaskStatus::Blocked => style("✕").red(),
        TaskStatus::Cancelled => style("○").dim(),
    };

    let title_style = if task.status == TaskStatus::Done || task.status == TaskStatus::Cancelled {
        Style::new().dim()
    } else {
        Style::new()
    };

    let scope = if task.is_global() {
        style("[global]").yellow().dim()
    } else {
        style("[project]").cyan().dim()
    };

    println!(
        "{} #{} {} {}",
        status_indicator,
        style(task.id.0).dim(),
        title_style.apply_to(&task.title),
        scope
    );
}

fn format_status(status: TaskStatus) -> console::StyledObject<&'static str> {
    match status {
        TaskStatus::Todo => style("todo").white(),
        TaskStatus::InProgress => style("in_progress").blue(),
        TaskStatus::Done => style("done").green(),
        TaskStatus::Blocked => style("blocked").red(),
        TaskStatus::Cancelled => style("cancelled").dim(),
    }
}
