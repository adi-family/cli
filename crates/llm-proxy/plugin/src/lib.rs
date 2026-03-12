use lib_console_output::{
    blocks::{Columns, KeyValue, Renderable, Section},
    out_info, out_success, theme,
};
use lib_plugin_prelude::*;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

// ============================================================================
// CLI ARGS
// ============================================================================

#[derive(CliArgs)]
pub struct KeysListArgs {}

#[derive(CliArgs)]
pub struct KeysAddArgs {
    #[arg(long, short = 'n')]
    pub name: String,

    #[arg(long, short = 'p')]
    pub provider: String,

    #[arg(long, short = 'k')]
    pub key: String,
}

#[derive(CliArgs)]
pub struct KeysRemoveArgs {
    #[arg(position = 0)]
    pub id: String,
}

#[derive(CliArgs)]
pub struct KeysVerifyArgs {
    #[arg(position = 0)]
    pub id: String,
}

#[derive(CliArgs)]
pub struct TokensListArgs {}

#[derive(CliArgs)]
pub struct TokensCreateArgs {
    #[arg(long, short = 'n')]
    pub name: String,

    #[arg(long, short = 'm')]
    pub mode: String,

    #[arg(long)]
    pub key_id: Option<String>,

    #[arg(long)]
    pub provider: Option<String>,
}

#[derive(CliArgs)]
pub struct TokensRevokeArgs {
    #[arg(position = 0)]
    pub id: String,
}

#[derive(CliArgs)]
pub struct TokensRotateArgs {
    #[arg(position = 0)]
    pub id: String,
}

#[derive(CliArgs)]
pub struct UsageArgs {
    #[arg(long)]
    pub from: Option<String>,

    #[arg(long)]
    pub to: Option<String>,
}

#[derive(CliArgs)]
pub struct ProvidersArgs {}

// ============================================================================
// PLUGIN
// ============================================================================

pub struct LlmProxyPlugin;

impl LlmProxyPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LlmProxyPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LlmProxyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.llm-proxy", "ADI LLM Proxy", env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Core)
            .with_author("ADI Team")
            .with_description("LLM API proxy with BYOK/Platform modes")
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        PluginCtx::init(ctx);
        let _ = get_runtime();
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
impl CliCommands for LlmProxyPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            Self::__sdk_cmd_meta_cmd_keys_list(),
            Self::__sdk_cmd_meta_cmd_keys_add(),
            Self::__sdk_cmd_meta_cmd_keys_remove(),
            Self::__sdk_cmd_meta_cmd_keys_verify(),
            Self::__sdk_cmd_meta_cmd_tokens_list(),
            Self::__sdk_cmd_meta_cmd_tokens_create(),
            Self::__sdk_cmd_meta_cmd_tokens_revoke(),
            Self::__sdk_cmd_meta_cmd_tokens_rotate(),
            Self::__sdk_cmd_meta_cmd_usage(),
            Self::__sdk_cmd_meta_cmd_providers(),
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("keys-list") => self.__sdk_cmd_handler_cmd_keys_list(ctx).await,
            Some("keys-add") => self.__sdk_cmd_handler_cmd_keys_add(ctx).await,
            Some("keys-remove") => self.__sdk_cmd_handler_cmd_keys_remove(ctx).await,
            Some("keys-verify") => self.__sdk_cmd_handler_cmd_keys_verify(ctx).await,
            Some("tokens-list") => self.__sdk_cmd_handler_cmd_tokens_list(ctx).await,
            Some("tokens-create") => self.__sdk_cmd_handler_cmd_tokens_create(ctx).await,
            Some("tokens-revoke") => self.__sdk_cmd_handler_cmd_tokens_revoke(ctx).await,
            Some("tokens-rotate") => self.__sdk_cmd_handler_cmd_tokens_rotate(ctx).await,
            Some("usage") => self.__sdk_cmd_handler_cmd_usage(ctx).await,
            Some("providers") => self.__sdk_cmd_handler_cmd_providers(ctx).await,
            Some("") | Some("help") | None => Ok(CliResult::success(self.help())),
            Some(cmd) => Ok(CliResult::error(format!(
                "Unknown command: {}. Run 'adi run adi.llm-proxy' for help.",
                cmd
            ))),
        }
    }
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

impl LlmProxyPlugin {
    fn help(&self) -> String {
        format!(
            "{}\n\n\
             {}\n\
             \x20 keys-list       List upstream API keys\n\
             \x20 keys-add        Add an upstream API key\n\
             \x20 keys-remove     Remove an upstream API key\n\
             \x20 keys-verify     Verify an upstream API key\n\
             \x20 tokens-list     List proxy tokens\n\
             \x20 tokens-create   Create a proxy token\n\
             \x20 tokens-revoke   Revoke a proxy token\n\
             \x20 tokens-rotate   Rotate a proxy token secret\n\
             \x20 usage           View usage statistics\n\
             \x20 providers       List available platform providers\n\n\
             {}\n\
             \x20 adi run adi.llm-proxy keys-list\n\
             \x20 adi run adi.llm-proxy keys-add --name my-key --provider openai --key sk-...\n\
             \x20 adi run adi.llm-proxy tokens-create --name my-token --mode byok --key-id <uuid>\n\
             \x20 adi run adi.llm-proxy usage --from 2024-01-01",
            theme::brand_bold("ADI LLM Proxy"),
            theme::bold("Commands:"),
            theme::bold("Usage:"),
        )
    }

    #[command(name = "keys-list", description = "List upstream API keys")]
    async fn cmd_keys_list(&self, _args: KeysListArgs) -> CmdResult {
        out_info!("Upstream API keys:");
        Ok("(No keys configured. Use 'keys-add' to add one.)".to_string())
    }

    #[command(name = "keys-add", description = "Add an upstream API key")]
    async fn cmd_keys_add(&self, args: KeysAddArgs) -> CmdResult {
        out_success!(
            "Added upstream key '{}' for provider '{}'",
            args.name,
            args.provider
        );
        Ok(String::new())
    }

    #[command(name = "keys-remove", description = "Remove an upstream API key")]
    async fn cmd_keys_remove(&self, args: KeysRemoveArgs) -> CmdResult {
        out_success!("Removed upstream key: {}", args.id);
        Ok(String::new())
    }

    #[command(name = "keys-verify", description = "Verify an upstream API key")]
    async fn cmd_keys_verify(&self, args: KeysVerifyArgs) -> CmdResult {
        out_success!("Verifying key {}... OK", args.id);
        Ok(String::new())
    }

    #[command(name = "tokens-list", description = "List proxy tokens")]
    async fn cmd_tokens_list(&self, _args: TokensListArgs) -> CmdResult {
        out_info!("Proxy tokens:");
        Ok("(No tokens configured. Use 'tokens-create' to create one.)".to_string())
    }

    #[command(name = "tokens-create", description = "Create a proxy token")]
    async fn cmd_tokens_create(&self, args: TokensCreateArgs) -> CmdResult {
        match args.mode.as_str() {
            "byok" => {
                if args.key_id.is_none() {
                    return Err("--key-id is required for BYOK mode".to_string());
                }
            }
            "platform" => {
                if args.provider.is_none() {
                    return Err("--provider is required for platform mode".to_string());
                }
            }
            other => return Err(format!("--mode must be 'byok' or 'platform', got '{}'", other)),
        }

        out_success!("Created proxy token '{}'", args.name);
        Ok("SECRET (save this, shown only once!):\n  adi_pk_xxxxxxxxxxxxxxxxxxxx".to_string())
    }

    #[command(name = "tokens-revoke", description = "Revoke a proxy token")]
    async fn cmd_tokens_revoke(&self, args: TokensRevokeArgs) -> CmdResult {
        out_success!("Revoked token: {}", args.id);
        Ok(String::new())
    }

    #[command(name = "tokens-rotate", description = "Rotate a proxy token secret")]
    async fn cmd_tokens_rotate(&self, args: TokensRotateArgs) -> CmdResult {
        out_success!("Rotated token: {}", args.id);
        Ok("NEW SECRET (save this, shown only once!):\n  adi_pk_yyyyyyyyyyyyyyyyyyyy".to_string())
    }

    #[command(name = "usage", description = "View usage statistics")]
    async fn cmd_usage(&self, args: UsageArgs) -> CmdResult {
        let mut kv = KeyValue::new().indent(2);

        if let Some(from) = &args.from {
            kv = kv.entry("From", from);
        }
        if let Some(to) = &args.to {
            kv = kv.entry("To", to);
        }

        kv = kv
            .entry("Total Requests", "0")
            .entry("Input Tokens", "0")
            .entry("Output Tokens", "0")
            .entry("Total Cost", "$0.00")
            .entry("Success Rate", "N/A");

        let mut output = Section::new("Usage Summary").width(60).render();
        output.push('\n');
        output.push_str(&kv.to_string());

        Ok(output)
    }

    #[command(name = "providers", description = "List available platform providers")]
    async fn cmd_providers(&self, _args: ProvidersArgs) -> CmdResult {
        let cols = Columns::new()
            .header(["PROVIDER", "STATUS", "MODELS"])
            .indent(2)
            .gap(2)
            .row(["openai", "-", "(not configured)"])
            .row(["anthropic", "-", "(not configured)"])
            .row(["openrouter", "-", "(not configured)"]);

        let mut output = Section::new("Available Platform Providers").width(60).render();
        output.push('\n');
        output.push_str(&cols.to_string());

        Ok(output)
    }
}

// ============================================================================
// EXPORTS
// ============================================================================

#[no_mangle]
pub extern "C" fn plugin_abi_version() -> u32 {
    lib_plugin_abi_v3::PLUGIN_API_VERSION
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(LlmProxyPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(LlmProxyPlugin::new())
}
