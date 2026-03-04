//! # lib-plugin-sdk: Plugin SDK for ADI Ecosystem
//!
//! Procedural macros for simplified plugin development.
//!
//! ## Features
//!
//! - `#[plugin]` - Main plugin struct annotation
//! - `#[command]` - Plugin commands (`adi <plugin> <cmd>`)
//! - `#[global_command]` - Global CLI commands (`adi <cmd>`)
//! - `#[http_routes]` - HTTP server routes (auto-detected)
//! - `#[webrtc_handlers]` - WebRTC message handlers (auto-detected)
//! - `#[daemon_service]` - Background daemon service (auto-detected)
//! - `daemon_cmd!` / `daemon_sudo!` - Daemon command registration
//!
//! ## Example
//!
//! ```rust,ignore
//! use lib_plugin_sdk::*;
//!
//! #[plugin]
//! struct TasksPlugin {
//!     db: Database,
//! }
//!
//! impl TasksPlugin {
//!     async fn init(&mut self, ctx: PluginContext) -> Result<()> {
//!         self.db = Database::open(ctx.data_dir)?;
//!         Ok(())
//!     }
//!
//!     #[command(name = "list")]
//!     async fn list_tasks(&self, status: Option<String>) -> CmdResult {
//!         let tasks = self.db.list(status)?;
//!         Ok(tasks.into())
//!     }
//!
//!     #[global_command(name = "up")]
//!     async fn up(&self, service: Option<String>) -> CmdResult {
//!         // adi up
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ImplItemFn, ItemImpl, ItemStruct};

mod cli_args;
mod command;
mod daemon;
mod http;
mod plugin;
mod webrtc;

use command::{CommandAttr, CommandType};

/// Main plugin macro for struct annotation.
///
/// Generates:
/// - `Plugin` trait implementation
/// - `plugin_create()` entry point
/// - Optional lifecycle hooks
///
/// # Example
///
/// ```rust,ignore
/// #[plugin]
/// struct TasksPlugin {
///     db: Database,
/// }
///
/// impl TasksPlugin {
///     async fn init(&mut self, ctx: PluginContext) -> Result<()> {
///         self.db = Database::open(ctx.data_dir)?;
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    match plugin::expand_plugin(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Command macro for plugin subcommands (`adi <plugin> <cmd>`).
///
/// # Attributes
///
/// - `name = "..."` - Command name (required)
/// - `description = "..."` - Help text (uses translation key by default)
/// - `usage = "..."` - Usage string
///
/// # Example
///
/// ```rust,ignore
/// impl TasksPlugin {
///     #[command(name = "list")]
///     async fn list_tasks(&self, status: Option<String>) -> CmdResult {
///         let tasks = self.db.list(status)?;
///         Ok(tasks.into())
///     }
///
///     #[command(name = "add")]
///     async fn add_task(&self, title: String) -> CmdResult {
///         // adi tasks add "my task"
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as CommandAttr);
    let input = parse_macro_input!(item as ImplItemFn);

    match command::expand_command(attr, input, CommandType::Plugin) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Global command macro for CLI root commands (`adi <cmd>`).
///
/// These commands are registered directly to the `adi` CLI, not under the plugin name.
///
/// # Example
///
/// ```rust,ignore
/// impl HivePlugin {
///     #[global_command(name = "up")]
///     async fn up(&self, service: Option<String>) -> CmdResult {
///         // adi up
///         // adi up myservice
///     }
///
///     #[global_command(name = "down")]
///     async fn down(&self) -> CmdResult {
///         // adi down
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn global_command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as CommandAttr);
    let input = parse_macro_input!(item as ImplItemFn);

    match command::expand_command(attr, input, CommandType::Global) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// HTTP routes macro for plugins providing HTTP endpoints.
///
/// When present, the plugin will be detected as needing HTTP support.
/// Port is assigned by host at runtime via `ctx.http_port()`.
///
/// # Example
///
/// ```rust,ignore
/// impl TasksPlugin {
///     #[http_routes]
///     fn routes(&self) -> Router {
///         Router::new()
///             .route("/tasks", get(self.list_handler))
///             .route("/tasks/:id", get(self.get_handler))
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn http_routes(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ImplItemFn);

    match http::expand_http_routes(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// WebRTC handlers macro for plugins handling peer connections.
///
/// Signaling is managed by CLI core. Plugin only defines message handlers.
///
/// # Example
///
/// ```rust,ignore
/// #[webrtc_handlers]
/// impl TasksPlugin {
///     async fn on_connect(&self, peer: Peer) {
///         // peer connected
///     }
///
///     async fn on_message(&self, peer: Peer, msg: Message) {
///         // handle incoming data
///     }
///
///     async fn on_disconnect(&self, peer: Peer) {
///         // cleanup
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn webrtc_handlers(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);

    match webrtc::expand_webrtc_handlers(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Daemon service macro for long-running background services.
///
/// When present, the plugin will be detected as having a daemon component.
///
/// # Example
///
/// ```rust,ignore
/// #[daemon_service]
/// impl TasksPlugin {
///     async fn start(&self, ctx: DaemonContext) -> Result<()> {
///         loop {
///             self.process_queue().await?;
///             tokio::time::sleep(Duration::from_secs(1)).await;
///         }
///     }
///
///     async fn stop(&self) -> Result<()> {
///         // cleanup on stop
///     }
///
///     async fn status(&self) -> ServiceStatus {
///         ServiceStatus::Running
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn daemon_service(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);

    match daemon::expand_daemon_service(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Register a regular daemon command (user-level).
///
/// Commands are collected at compile time and shown during plugin installation.
///
/// # Example
///
/// ```rust,ignore
/// impl TasksPlugin {
///     async fn install_deps(&self, ctx: PluginContext) -> Result<()> {
///         ctx.daemon().exec(daemon_cmd!("brew install ffmpeg")).await?;
///         Ok(())
///     }
/// }
/// ```
#[proc_macro]
pub fn daemon_cmd(input: TokenStream) -> TokenStream {
    daemon::expand_daemon_cmd(input, false)
}

/// Register a sudo daemon command (root-level).
///
/// Commands are collected at compile time and shown during plugin installation.
/// Requires separate user approval.
///
/// # Example
///
/// ```rust,ignore
/// impl TasksPlugin {
///     async fn restart_service(&self, ctx: PluginContext) -> Result<()> {
///         ctx.daemon().exec(daemon_sudo!("systemctl restart nginx")).await?;
///         Ok(())
///     }
/// }
/// ```
#[proc_macro]
pub fn daemon_sudo(input: TokenStream) -> TokenStream {
    daemon::expand_daemon_cmd(input, true)
}

/// Derive macro for CLI arguments.
///
/// Generates `CliArgs` trait implementation providing:
/// - `schema()` - Returns `Vec<CliArg>` for AI agent consumption
/// - `parse(ctx)` - Parses `CliContext` into typed struct
///
/// # Attributes
///
/// - `#[arg(long)]` - Long flag (e.g., --status)
/// - `#[arg(long = "custom-name")]` - Custom long flag name
/// - `#[arg(short = 'c')]` - Short flag (e.g., -c)
/// - `#[arg(position = 0)]` - Positional argument
/// - `#[arg(default = value)]` - Default value
///
/// # Type Mapping
///
/// | Rust Type | Required | CLI Type |
/// |-----------|----------|----------|
/// | `String` | yes | String |
/// | `Option<String>` | no | String |
/// | `i32`, `i64` | yes | Int |
/// | `Option<i32>` | no | Int |
/// | `bool` | no (flag) | Bool |
/// | `f64` | yes | Float |
///
/// # Example
///
/// ```rust,ignore
/// #[derive(CliArgs)]
/// struct ListArgs {
///     #[arg(long)]
///     status: Option<String>,
///
///     #[arg(long, default = 10)]
///     limit: i32,
///
///     #[arg(position = 0)]
///     filter: Option<String>,
/// }
///
/// impl TasksPlugin {
///     #[command(name = "list", args = ListArgs)]
///     async fn list_tasks(&self, args: ListArgs) -> CmdResult {
///         // args.status, args.limit, args.filter are typed!
///         Ok(format!("Limit: {}", args.limit))
///     }
/// }
/// ```
#[proc_macro_derive(CliArgs, attributes(arg))]
pub fn derive_cli_args(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match cli_args::expand_cli_args(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
