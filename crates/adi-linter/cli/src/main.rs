use std::path::PathBuf;

use adi_linter_core::{
    config::LinterConfig,
    output::{format_to_stdout, OutputFormat},
    runner::{Runner, RunnerConfig},
    AutofixConfig, AutofixEngine,
};
use clap::{Parser, Subcommand};
use console::style;
use lib_cli_common::{print_error, print_success};

#[derive(Parser)]
#[command(name = "adi-linter")]
#[command(about = "ADI Linter - Language-agnostic code linting")]
#[command(version)]
struct Cli {
    /// Project directory (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    project: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Files to lint (if no subcommand given)
    #[arg(trailing_var_arg = true)]
    files: Vec<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run linting (default command)
    Run {
        /// Files to lint (defaults to all files)
        #[arg(trailing_var_arg = true)]
        files: Vec<PathBuf>,

        /// Output format (pretty, json, sarif)
        #[arg(short, long, default_value = "pretty")]
        format: String,

        /// Only lint specific categories
        #[arg(long, value_delimiter = ',')]
        category: Vec<String>,

        /// Exclude categories
        #[arg(long, value_delimiter = ',')]
        exclude_category: Vec<String>,

        /// Fail on severity level (error, warning, info, hint)
        #[arg(long, default_value = "error")]
        fail_on: String,

        /// Run in parallel
        #[arg(long, default_value = "true")]
        parallel: bool,
    },

    /// Apply auto-fixes
    Fix {
        /// Files to fix (defaults to all files)
        #[arg(trailing_var_arg = true)]
        files: Vec<PathBuf>,

        /// Dry run (show fixes without applying)
        #[arg(long)]
        dry_run: bool,

        /// Interactive mode (prompt before each fix)
        #[arg(short, long)]
        interactive: bool,

        /// Maximum fix iterations
        #[arg(long, default_value = "10")]
        max_iterations: usize,
    },

    /// List configured linters
    List {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show configuration
    Config,

    /// Initialize linter configuration
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },
}

fn main() {
    lib_cli_common::setup_logging_quiet();

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    if let Err(e) = runtime.block_on(run()) {
        print_error(&format!("{}", e));
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let project_path = std::fs::canonicalize(&cli.project)?;

    match cli.command {
        Some(Commands::Run {
            files,
            format,
            category: _,
            exclude_category: _,
            fail_on,
            parallel,
        }) => {
            cmd_run(&project_path, files, &format, &fail_on, parallel).await?;
        }
        Some(Commands::Fix {
            files,
            dry_run,
            interactive,
            max_iterations,
        }) => {
            cmd_fix(&project_path, files, dry_run, interactive, max_iterations).await?;
        }
        Some(Commands::List { format }) => {
            cmd_list(&project_path, &format)?;
        }
        Some(Commands::Config) => {
            cmd_config(&project_path)?;
        }
        Some(Commands::Init { force }) => {
            cmd_init(&project_path, force)?;
        }
        None => {
            // Default: run linting on provided files or all files
            cmd_run(&project_path, cli.files, "pretty", "error", true).await?;
        }
    }

    Ok(())
}

async fn cmd_run(
    project_path: &PathBuf,
    files: Vec<PathBuf>,
    format: &str,
    fail_on: &str,
    parallel: bool,
) -> anyhow::Result<()> {
    let config = LinterConfig::load_from_project(project_path)?;
    let registry = config.build_registry()?;

    if registry.is_empty() {
        println!(
            "{} No linters configured. Run {} to create a config.",
            style("[!]").yellow(),
            style("adi lint init").cyan()
        );
        return Ok(());
    }

    let runner_config = RunnerConfig::new(project_path).parallel(parallel);
    let runner = Runner::new(registry, runner_config);

    let files = if files.is_empty() {
        None
    } else {
        Some(files)
    };

    let result = runner.run(files).await?;

    let output_format = match format {
        "json" => OutputFormat::Json,
        "sarif" => OutputFormat::Sarif,
        _ => OutputFormat::Pretty,
    };

    format_to_stdout(&result, output_format)?;

    // Exit with error code if issues found
    let fail_severity = match fail_on {
        "warning" => adi_linter_core::Severity::Warning,
        "info" => adi_linter_core::Severity::Info,
        "hint" => adi_linter_core::Severity::Hint,
        _ => adi_linter_core::Severity::Error,
    };

    let has_failures = result
        .diagnostics
        .iter()
        .any(|d| d.severity >= fail_severity);

    if has_failures {
        std::process::exit(1);
    }

    Ok(())
}

async fn cmd_fix(
    project_path: &PathBuf,
    files: Vec<PathBuf>,
    dry_run: bool,
    interactive: bool,
    max_iterations: usize,
) -> anyhow::Result<()> {
    let config = LinterConfig::load_from_project(project_path)?;
    let registry = config.build_registry()?;

    if registry.is_empty() {
        println!(
            "{} No linters configured. Run {} to create a config.",
            style("[!]").yellow(),
            style("adi lint init").cyan()
        );
        return Ok(());
    }

    let runner_config = RunnerConfig::new(project_path);
    let runner = Runner::new(registry, runner_config);

    let autofix_config = AutofixConfig {
        max_iterations,
        dry_run,
        interactive,
    };

    let engine = AutofixEngine::new(&runner, autofix_config);

    let files = if files.is_empty() {
        None
    } else {
        Some(files)
    };

    let result = engine.run(files).await?;

    if dry_run {
        println!(
            "{} Dry run: {} fixes would be applied",
            style("[i]").cyan(),
            result.fixes_count()
        );
    } else {
        print_success(&format!(
            "Applied {} fixes in {} iterations",
            result.fixes_count(),
            result.iterations
        ));
    }

    if !result.remaining_diagnostics.is_empty() {
        println!(
            "{} {} issues remaining ({} fixable)",
            style("[!]").yellow(),
            result.remaining_count(),
            result
                .remaining_diagnostics
                .iter()
                .filter(|d| d.is_fixable())
                .count()
        );
    }

    if result.max_iterations_reached {
        println!(
            "{} Max iterations reached. Run again to continue fixing.",
            style("[!]").yellow()
        );
    }

    Ok(())
}

fn cmd_list(project_path: &PathBuf, format: &str) -> anyhow::Result<()> {
    let config = LinterConfig::load_from_project(project_path)?;
    let registry = config.build_registry()?;

    if format == "json" {
        let linters: Vec<_> = registry
            .all_linters()
            .map(|l| {
                serde_json::json!({
                    "id": l.id(),
                    "category": l.category().display_name(),
                    "priority": l.priority(),
                    "patterns": l.patterns(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&linters)?);
        return Ok(());
    }

    if registry.is_empty() {
        println!("{} No linters configured", style("[!]").yellow());
        return Ok(());
    }

    println!("{}", style("Configured Linters").bold());
    println!();

    for linter in registry.by_priority() {
        println!(
            "  {} {} {}",
            style(linter.category().icon()).dim(),
            style(linter.id()).cyan(),
            style(format!("(priority: {})", linter.priority())).dim()
        );

        for pattern in linter.patterns() {
            println!("      {} {}", style("->").dim(), pattern);
        }
    }

    Ok(())
}

fn cmd_config(project_path: &PathBuf) -> anyhow::Result<()> {
    let config_path = project_path.join(".adi").join("linter.toml");

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        println!("{}", style(format!("Config: {}", config_path.display())).dim());
        println!();
        println!("{}", content);
    } else {
        let alt_path = project_path.join("linter.toml");
        if alt_path.exists() {
            let content = std::fs::read_to_string(&alt_path)?;
            println!("{}", style(format!("Config: {}", alt_path.display())).dim());
            println!();
            println!("{}", content);
        } else {
            println!("{} No config found. Run {} to create one.",
                style("[!]").yellow(),
                style("adi lint init").cyan()
            );
        }
    }

    Ok(())
}

fn cmd_init(project_path: &PathBuf, force: bool) -> anyhow::Result<()> {
    let config_dir = project_path.join(".adi");
    let config_path = config_dir.join("linter.toml");

    if config_path.exists() && !force {
        println!(
            "{} Config already exists at {}",
            style("[!]").yellow(),
            config_path.display()
        );
        println!("  Use --force to overwrite");
        return Ok(());
    }

    std::fs::create_dir_all(&config_dir)?;

    let default_config = r#"# ADI Linter Configuration

[linter]
parallel = true
fail_fast = false
timeout = 30

[autofix]
enabled = true
max_iterations = 10

# Category configuration
[categories]
security = { enabled = true, fail_on = "warning" }
correctness = { enabled = true }
error-handling = { enabled = true }
architecture = { enabled = true }
performance = { enabled = true }
code-quality = { enabled = true }
best-practices = { enabled = true }
testing = { enabled = true }
documentation = { enabled = false }
naming = { enabled = true }
style = { enabled = true, priority = 50 }

# Example command linter
# [[rules.command]]
# id = "no-todo"
# category = "code-quality"
# type = "regex-forbid"
# pattern = "TODO|FIXME"
# message = "Unresolved TODO comment"
# glob = "**/*.rs"
# severity = "warning"

# Example external linter
# [[rules.exec]]
# id = "shellcheck"
# category = "correctness"
# exec = "shellcheck -f json {file}"
# glob = "**/*.sh"
# output = "json"
"#;

    std::fs::write(&config_path, default_config)?;

    print_success(&format!("Created {}", config_path.display()));
    println!("  Edit the file to add your linting rules.");

    Ok(())
}
