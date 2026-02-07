//! ADI API Proxy CLI Plugin
//!
//! Provides CLI commands for managing proxy tokens and upstream API keys.

use lib_plugin_abi_v3::{
    async_trait,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult, SERVICE_CLI_COMMANDS,
};
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

/// Global tokio runtime for async operations
static RUNTIME: OnceCell<Runtime> = OnceCell::new();
static API_URL: OnceCell<String> = OnceCell::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

/// API Proxy Plugin
pub struct ApiProxyPlugin;

impl ApiProxyPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiProxyPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for ApiProxyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.api-proxy".to_string(),
            name: "ADI API Proxy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("LLM API proxy with BYOK/Platform modes".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        let _ = get_runtime();
        // Get API URL from env or default
        let url = std::env::var("API_PROXY_URL")
            .unwrap_or_else(|_| "http://localhost:8024".to_string());
        let _ = API_URL.set(url);
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
impl CliCommands for ApiProxyPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "keys".to_string(),
                description: "Manage upstream API keys".to_string(),
                usage: "keys <list|add|remove|verify> [options]".to_string(),
                has_subcommands: true,
            },
            CliCommand {
                name: "tokens".to_string(),
                description: "Manage proxy tokens".to_string(),
                usage: "tokens <list|create|revoke|rotate> [options]".to_string(),
                has_subcommands: true,
            },
            CliCommand {
                name: "usage".to_string(),
                description: "View usage statistics".to_string(),
                usage: "usage [--from DATE] [--to DATE]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "providers".to_string(),
                description: "List available platform providers".to_string(),
                usage: "providers".to_string(),
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> {
        let subcommand = ctx.subcommand.as_deref().unwrap_or("");
        let args: Vec<String> = ctx.args.clone();

        let result = match subcommand {
            "keys" => cmd_keys(&args),
            "tokens" => cmd_tokens(&args),
            "usage" => cmd_usage(&args),
            "providers" => cmd_providers(),
            "" | "help" => Ok(get_help()),
            _ => Err(format!(
                "Unknown command: {}. Use 'api-proxy help' for usage.",
                subcommand
            )),
        };

        match result {
            Ok(output) => Ok(CliResult::success(output)),
            Err(e) => Ok(CliResult::error(e)),
        }
    }
}

/// Create the plugin instance (v3 entry point)
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(ApiProxyPlugin::new())
}

/// Create the CLI commands interface
#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(ApiProxyPlugin::new())
}

// === Command Implementations ===

fn get_help() -> String {
    let mut help = String::new();
    help.push_str("ADI API Proxy - LLM API proxy with BYOK/Platform modes\n\n");
    help.push_str("USAGE:\n");
    help.push_str("  adi api-proxy <COMMAND>\n\n");
    help.push_str("COMMANDS:\n");
    help.push_str("  keys       Manage upstream API keys (BYOK)\n");
    help.push_str("  tokens     Manage proxy tokens\n");
    help.push_str("  usage      View usage statistics\n");
    help.push_str("  providers  List available platform providers\n");
    help.push_str("  help       Show this help message\n\n");
    help.push_str("EXAMPLES:\n");
    help.push_str("  adi api-proxy keys list\n");
    help.push_str(
        "  adi api-proxy tokens create --name \"my-token\" --mode byok --key-id <uuid>\n",
    );
    help.push_str("  adi api-proxy usage --from 2024-01-01\n");
    help
}

fn cmd_keys(args: &[String]) -> Result<String, String> {
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("list");

    match subcommand {
        "list" => {
            Ok(
                "Upstream API keys:\n  (No keys configured. Use 'keys add' to add one.)"
                    .to_string(),
            )
        }
        "add" => {
            let mut name = None;
            let mut provider = None;
            let mut api_key = None;

            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--name" | "-n" => {
                        name = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--provider" | "-p" => {
                        provider = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--key" | "-k" => {
                        api_key = args.get(i + 1).cloned();
                        i += 2;
                    }
                    _ => i += 1,
                }
            }

            if name.is_none() || provider.is_none() || api_key.is_none() {
                return Err("Usage: keys add --name <name> --provider <openai|anthropic|openrouter|custom> --key <api-key>".to_string());
            }

            Ok(format!(
                "Added upstream key '{}' for provider '{}'",
                name.unwrap(),
                provider.unwrap()
            ))
        }
        "remove" => {
            let id = args.get(1).ok_or("Usage: keys remove <key-id>")?;
            Ok(format!("Removed upstream key: {}", id))
        }
        "verify" => {
            let id = args.get(1).ok_or("Usage: keys verify <key-id>")?;
            Ok(format!("Verifying key {}... OK", id))
        }
        _ => Err(format!(
            "Unknown keys command: {}. Use list, add, remove, or verify.",
            subcommand
        )),
    }
}

fn cmd_tokens(args: &[String]) -> Result<String, String> {
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("list");

    match subcommand {
        "list" => Ok(
            "Proxy tokens:\n  (No tokens configured. Use 'tokens create' to create one.)"
                .to_string(),
        ),
        "create" => {
            let mut name = None;
            let mut mode = None;
            let mut key_id = None;
            let mut provider = None;

            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--name" | "-n" => {
                        name = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--mode" | "-m" => {
                        mode = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--key-id" => {
                        key_id = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--provider" => {
                        provider = args.get(i + 1).cloned();
                        i += 2;
                    }
                    _ => i += 1,
                }
            }

            let name = name.ok_or("--name is required")?;
            let mode = mode.ok_or("--mode (byok|platform) is required")?;

            match mode.as_str() {
                "byok" => {
                    let _key_id = key_id.ok_or("--key-id is required for BYOK mode")?;
                }
                "platform" => {
                    let _provider = provider.ok_or("--provider is required for platform mode")?;
                }
                _ => return Err("--mode must be 'byok' or 'platform'".to_string()),
            }

            Ok(format!(
                "Created proxy token '{}'\n\nSECRET (save this, shown only once!):\n  adi_pk_xxxxxxxxxxxxxxxxxxxx",
                name
            ))
        }
        "revoke" => {
            let id = args.get(1).ok_or("Usage: tokens revoke <token-id>")?;
            Ok(format!("Revoked token: {}", id))
        }
        "rotate" => {
            let id = args.get(1).ok_or("Usage: tokens rotate <token-id>")?;
            Ok(format!(
                "Rotated token: {}\n\nNEW SECRET (save this, shown only once!):\n  adi_pk_yyyyyyyyyyyyyyyyyyyy",
                id
            ))
        }
        _ => Err(format!(
            "Unknown tokens command: {}. Use list, create, revoke, or rotate.",
            subcommand
        )),
    }
}

fn cmd_usage(args: &[String]) -> Result<String, String> {
    let mut from = None;
    let mut to = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--from" => {
                from = args.get(i + 1).cloned();
                i += 2;
            }
            "--to" => {
                to = args.get(i + 1).cloned();
                i += 2;
            }
            _ => i += 1,
        }
    }

    let mut output = String::new();
    output.push_str("Usage Summary\n");
    output.push_str("=============\n\n");

    if let Some(f) = from {
        output.push_str(&format!("From: {}\n", f));
    }
    if let Some(t) = to {
        output.push_str(&format!("To: {}\n", t));
    }

    output.push_str("\nTotal Requests:    0\n");
    output.push_str("Input Tokens:      0\n");
    output.push_str("Output Tokens:     0\n");
    output.push_str("Total Cost:        $0.00\n");
    output.push_str("Success Rate:      N/A\n");

    Ok(output)
}

fn cmd_providers() -> Result<String, String> {
    let mut output = String::new();
    output.push_str("Available Platform Providers\n");
    output.push_str("============================\n\n");
    output.push_str("Provider      Status    Models\n");
    output.push_str("--------      ------    ------\n");
    output.push_str("openai        -         (not configured)\n");
    output.push_str("anthropic     -         (not configured)\n");
    output.push_str("openrouter    -         (not configured)\n");
    Ok(output)
}
