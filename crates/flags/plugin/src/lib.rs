use flags_core::{self as core, CheckMode, FileStatus};
use lib_console_output::{
    blocks::{Columns, List, Renderable, Section},
    out_error, out_info, out_success, out_warn, theme,
};
use lib_plugin_abi_v3::cli::{CliCommand, CliCommands, CliContext, CliResult};
use lib_plugin_abi_v3::*;

pub struct FlagsPlugin;

#[async_trait]
impl Plugin for FlagsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.flags".to_string(),
            name: "ADI Flags".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Track file review freshness across named states".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl CliCommands for FlagsPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "set".to_string(),
                description: "Flag files as clean for a state".to_string(),
                usage: "flags set <state> <files...>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "status".to_string(),
                description: "Show dirty files".to_string(),
                usage: "flags status [state]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "list".to_string(),
                description: "List all tracked files".to_string(),
                usage: "flags list [state]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "clear".to_string(),
                description: "Remove flags".to_string(),
                usage: "flags clear <state> [files...]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "states".to_string(),
                description: "List configured states".to_string(),
                usage: "flags states".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "init".to_string(),
                description: "Create default .adi/flags.toml".to_string(),
                usage: "flags init".to_string(),
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        let result = match ctx.subcommand.as_deref() {
            Some("set") => cmd_set(ctx),
            Some("status") => cmd_status(ctx),
            Some("list") => cmd_list(ctx),
            Some("clear") => cmd_clear(ctx),
            Some("states") => cmd_states(ctx),
            Some("init") => cmd_init(ctx),
            Some(other) => {
                out_error!("Unknown command: {}", other);
                return Ok(CliResult::error(format!("Unknown command: {other}")));
            }
            None => {
                out_info!("Usage: adi flags <set|status|list|clear|states|init>");
                return Ok(CliResult::success(""));
            }
        };

        match result {
            Ok(()) => Ok(CliResult::success("")),
            Err(e) => {
                out_error!("{}", e);
                Ok(CliResult::error(e.to_string()))
            }
        }
    }
}

fn cmd_set(ctx: &CliContext) -> core::Result<()> {
    let config = core::load_config(&ctx.cwd)?;
    let state = ctx.arg(0).ok_or_else(|| {
        core::Error::Other("Usage: adi flags set <state> <files...>".to_string())
    })?;
    let files: Vec<String> = ctx.args[1..].to_vec();
    if files.is_empty() {
        return Err(core::Error::Other(
            "No files specified. Usage: adi flags set <state> <files...>".to_string(),
        ));
    }

    let flagged = core::flag_files(&ctx.cwd, &config, state, &files)?;
    out_success!(
        "Flagged {} file(s) as '{}'",
        flagged.len(),
        theme::brand_bold(state)
    );
    Ok(())
}

fn cmd_status(ctx: &CliContext) -> core::Result<()> {
    let config = core::load_config(&ctx.cwd)?;
    let filter_state = ctx.arg(0);

    let states: Vec<String> = match filter_state {
        Some(s) => {
            core::validate_state(&config, s)?;
            vec![s.to_string()]
        }
        None => config.states.keys().cloned().collect(),
    };

    let mut any_dirty = false;

    for state in &states {
        let results = core::check_status(&ctx.cwd, state, config.check)?;
        if results.is_empty() {
            continue;
        }

        let dirty_count = results
            .iter()
            .filter(|(_, s)| *s != FileStatus::Clean)
            .count();
        let total = results.len();

        Section::new(&format!(
            "{} ({} dirty / {} total)",
            theme::brand_bold(state.as_str()),
            dirty_count,
            total
        ))
        .print();

        let rows: Vec<[String; 2]> = results
            .iter()
            .map(|(entry, status)| {
                let icon = match status {
                    FileStatus::Clean => theme::success(theme::icons::SUCCESS).to_string(),
                    FileStatus::Dirty => theme::error(theme::icons::ERROR).to_string(),
                    FileStatus::Missing => theme::warning("?").to_string(),
                };
                let label = match status {
                    FileStatus::Clean => entry.path.clone(),
                    FileStatus::Dirty => {
                        format!("{}  {}", entry.path, theme::muted("(modified)"))
                    }
                    FileStatus::Missing => {
                        format!("{}  {}", entry.path, theme::muted("(missing)"))
                    }
                };
                [icon, label]
            })
            .collect();

        let mut cols = Columns::new();
        for row in &rows {
            cols = cols.row([row[0].as_str(), row[1].as_str()]);
        }
        cols.print();
        println!();

        if dirty_count > 0 {
            any_dirty = true;
        }
    }

    if !any_dirty {
        out_success!("All flagged files are clean");
    }

    Ok(())
}

fn cmd_list(ctx: &CliContext) -> core::Result<()> {
    let config = core::load_config(&ctx.cwd)?;
    let filter_state = ctx.arg(0);

    let states: Vec<String> = match filter_state {
        Some(s) => {
            core::validate_state(&config, s)?;
            vec![s.to_string()]
        }
        None => config.states.keys().cloned().collect(),
    };

    for state in &states {
        let index = core::load_index(&ctx.cwd, state)?;
        if index.is_empty() {
            out_info!("{}: no files flagged", theme::brand_bold(state.as_str()));
            continue;
        }

        Section::new(&format!(
            "{} ({} files)",
            theme::brand_bold(state.as_str()),
            index.len()
        ))
        .print();

        let items: Vec<String> = index.keys().cloned().collect();
        List::new().items(items).print();
        println!();
    }

    Ok(())
}

fn cmd_clear(ctx: &CliContext) -> core::Result<()> {
    let config = core::load_config(&ctx.cwd)?;
    let state = ctx.arg(0).ok_or_else(|| {
        core::Error::Other("Usage: adi flags clear <state> [files...]".to_string())
    })?;
    let files: Vec<String> = ctx.args[1..].to_vec();

    let removed = core::clear_flags(&ctx.cwd, &config, state, &files)?;
    if files.is_empty() {
        out_success!(
            "Cleared all {} flag(s) from '{}'",
            removed,
            theme::brand_bold(state)
        );
    } else {
        out_success!(
            "Cleared {} flag(s) from '{}'",
            removed,
            theme::brand_bold(state)
        );
    }
    Ok(())
}

fn cmd_states(ctx: &CliContext) -> core::Result<()> {
    let config = core::load_config(&ctx.cwd)?;

    if config.states.is_empty() {
        out_warn!("No states configured in .adi/flags.toml");
        return Ok(());
    }

    Section::new("Configured States").print();

    let mut cols = Columns::new().header(["State", "Description"]);
    for (name, state) in &config.states {
        cols = cols.row([
            &theme::brand_bold(name.as_str()).to_string(),
            &state.description,
        ]);
    }
    cols.print();

    out_info!(
        "Check mode: {}",
        theme::bold(match config.check {
            CheckMode::Fast => "fast (mtime only)",
            CheckMode::Strict => "strict (mtime + hash)",
        })
    );

    Ok(())
}

fn cmd_init(ctx: &CliContext) -> core::Result<()> {
    core::init_config(&ctx.cwd)?;
    out_success!("Created {}", theme::muted(".adi/flags.toml"));
    Ok(())
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(FlagsPlugin)
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(FlagsPlugin)
}
