//! TypeSpec Code Generator Plugin (v3 ABI)
//!
//! ADI plugin for generating code from TypeSpec definitions with file watching support.

use lib_plugin_abi_v3::{
    async_trait,
    cli::{CliCommand, CliCommands, CliContext, CliResult},
    Plugin, PluginContext, PluginMetadata, PluginType, Result as PluginResult,
    SERVICE_CLI_COMMANDS,
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use chrono::Local;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;
use typespec_api::{
    codegen::{protocol::RustProtocolConfig, Generator, Language, Side},
    parse, TypeSpecFile,
};

// Local result type for command implementations
type CmdResult = std::result::Result<String, String>;

/// Global flag for watch mode termination
static RUNNING: AtomicBool = AtomicBool::new(true);

/// TypeSpec Generator Plugin
pub struct TspGenPlugin;

impl TspGenPlugin {
    /// Create a new plugin instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for TspGenPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for TspGenPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.tsp-gen".to_string(),
            name: "TypeSpec Generator".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Extension,
            author: Some("ADI Team".to_string()),
            description: Some("Generate code from TypeSpec definitions with file watching".to_string()),
            category: None,
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
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
impl CliCommands for TspGenPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "generate".to_string(),
                description: "Generate code from TypeSpec files".to_string(),
                args: vec![],
                has_subcommands: false,
            },
            CliCommand {
                name: "languages".to_string(),
                description: "List supported languages".to_string(),
                args: vec![],
                has_subcommands: false,
            },
            CliCommand {
                name: "help".to_string(),
                description: "Show help information".to_string(),
                args: vec![],
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> PluginResult<CliResult> {
        let subcommand = ctx.subcommand.as_deref().unwrap_or("");
        
        let result = match subcommand {
            "generate" | "gen" => cmd_generate(&ctx.args),
            "languages" | "langs" => cmd_languages(),
            "help" | "" => cmd_help(),
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
    Box::new(TspGenPlugin::new())
}

/// Create the CLI commands interface (for separate trait object)
#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(TspGenPlugin::new())
}

// === Command Implementations ===

/// Parsed generation options
struct GenerateOptions {
    input_files: Vec<PathBuf>,
    output_dir: PathBuf,
    language: Language,
    side: Side,
    package: String,
    watch: bool,
    protocol_tag: String,
    protocol_rename: String,
    protocol_enum_name: String,
}

fn cmd_generate(args: &[String]) -> CmdResult {
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let opts = parse_generate_args(&args_refs)?;

    if opts.watch {
        cmd_generate_watch(&opts)
    } else {
        do_generate(&opts)
    }
}

fn parse_generate_args(args: &[&str]) -> Result<GenerateOptions, String> {
    let mut input_files: Vec<PathBuf> = Vec::new();
    let mut output_dir = PathBuf::from("generated");
    let mut language: Option<Language> = None;
    let mut side = Side::Both;
    let mut package = String::from("api");
    let mut watch = false;
    let mut protocol_tag = String::from("type");
    let mut protocol_rename = String::from("snake_case");
    let mut protocol_enum_name = String::from("SignalingMessage");

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-l" | "--language" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --language".to_string());
                }
                language = Some(parse_language(args[i + 1])?);
                i += 2;
            }
            "-o" | "--output" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --output".to_string());
                }
                output_dir = PathBuf::from(args[i + 1]);
                i += 2;
            }
            "-s" | "--side" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --side".to_string());
                }
                side = parse_side(args[i + 1])?;
                i += 2;
            }
            "-p" | "--package" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --package".to_string());
                }
                package = args[i + 1].to_string();
                i += 2;
            }
            "--protocol-tag" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --protocol-tag".to_string());
                }
                protocol_tag = args[i + 1].to_string();
                i += 2;
            }
            "--protocol-rename" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --protocol-rename".to_string());
                }
                protocol_rename = args[i + 1].to_string();
                i += 2;
            }
            "--protocol-enum-name" => {
                if i + 1 >= args.len() {
                    return Err("Missing value for --protocol-enum-name".to_string());
                }
                protocol_enum_name = args[i + 1].to_string();
                i += 2;
            }
            "-w" | "--watch" => {
                watch = true;
                i += 1;
            }
            arg if arg.starts_with('-') => {
                return Err(format!("Unknown option: {}", arg));
            }
            _ => {
                input_files.push(PathBuf::from(args[i]));
                i += 1;
            }
        }
    }

    if input_files.is_empty() {
        return Err(
            "No input files specified. Usage: generate <input...> -l <language>".to_string(),
        );
    }

    let language = language.ok_or("Missing required option: --language (-l)")?;

    Ok(GenerateOptions {
        input_files,
        output_dir,
        language,
        side,
        package,
        watch,
        protocol_tag,
        protocol_rename,
        protocol_enum_name,
    })
}

/// Run code generation in watch mode
fn cmd_generate_watch(opts: &GenerateOptions) -> CmdResult {
    // Reset running flag
    RUNNING.store(true, Ordering::SeqCst);

    // Set up Ctrl+C handler
    let _ = ctrlc::set_handler(|| {
        RUNNING.store(false, Ordering::SeqCst);
    });

    // Collect directories to watch (parent dirs of input files)
    let watch_dirs: HashSet<PathBuf> = opts
        .input_files
        .iter()
        .filter_map(|f| {
            f.canonicalize()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        })
        .collect();

    if watch_dirs.is_empty() {
        return Err("No valid directories to watch".to_string());
    }

    // Initial generation
    println!("TypeSpec Generator - Watch Mode");
    println!("================================\n");

    print!("Running initial generation... ");
    let _ = io::stdout().flush();

    match do_generate(opts) {
        Ok(msg) => println!("done\n{}\n", msg),
        Err(e) => println!("failed\nError: {}\n", e),
    }

    println!(
        "Watching {} director{} for changes:",
        watch_dirs.len(),
        if watch_dirs.len() == 1 { "y" } else { "ies" }
    );
    for dir in &watch_dirs {
        println!("  {}", dir.display());
    }
    println!("\nPress Ctrl+C to stop\n");

    // Create watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        Config::default().with_poll_interval(Duration::from_millis(500)),
    )
    .map_err(|e| format!("Failed to create watcher: {}", e))?;

    // Watch all directories
    for dir in &watch_dirs {
        watcher
            .watch(dir, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch {}: {}", dir.display(), e))?;
    }

    // Watch loop
    while RUNNING.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                // Filter for .tsp file changes
                let tsp_changed = event
                    .paths
                    .iter()
                    .any(|p| p.extension().map(|e| e == "tsp").unwrap_or(false));

                if tsp_changed {
                    let timestamp = Local::now().format("%H:%M:%S");
                    println!("[{}] Change detected, regenerating...", timestamp);

                    match do_generate(opts) {
                        Ok(msg) => println!("{}\n", msg),
                        Err(e) => println!("Error: {}\n", e),
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watch error: {}", e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, continue loop
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    println!("\nWatch stopped.");
    Ok(String::new())
}

/// Perform a single code generation run
fn do_generate(opts: &GenerateOptions) -> CmdResult {
    // Parse all input files with import resolution
    let mut combined = TypeSpecFile::default();
    let mut resolved = HashSet::new();

    for input in &opts.input_files {
        let canonical = input
            .canonicalize()
            .map_err(|e| format!("Failed to resolve path {}: {}", input.display(), e))?;

        // Skip if already processed
        if resolved.contains(&canonical) {
            continue;
        }
        resolved.insert(canonical.clone());

        let source = std::fs::read_to_string(&canonical)
            .map_err(|e| format!("Failed to read {}: {}", input.display(), e))?;

        let file =
            parse(&source).map_err(|e| format!("Failed to parse {}: {}", input.display(), e))?;

        // Resolve imports relative to the input file's directory
        let base_dir = canonical.parent().unwrap_or(Path::new("."));
        let resolved_file = resolve_imports(file, base_dir, &mut resolved)?;

        // Merge declarations
        combined.usings.extend(resolved_file.usings);
        combined.declarations.extend(resolved_file.declarations);

        if resolved_file.namespace.is_some() {
            combined.namespace = resolved_file.namespace;
        }
    }

    // Generate code — protocol side skips language subdirectory
    let output_subdir = if opts.side == Side::Protocol {
        opts.output_dir.clone()
    } else {
        opts.output_dir.join(match opts.language {
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::Rust => "rust",
            Language::OpenApi => "openapi",
        })
    };

    let mut generator = Generator::new(&combined, &output_subdir, &opts.package);

    if opts.side == Side::Protocol {
        generator = generator.with_rust_protocol_config(RustProtocolConfig {
            tag: opts.protocol_tag.clone(),
            rename: opts.protocol_rename.clone(),
            enum_name: opts.protocol_enum_name.clone(),
        });
    }

    let generated = generator
        .generate(opts.language, opts.side)
        .map_err(|e| format!("Code generation failed: {}", e))?;

    let mut output = format!("Generated {} files:", generated.len());
    for path in &generated {
        output.push_str(&format!("\n  {}", path));
    }

    Ok(output)
}

fn cmd_languages() -> CmdResult {
    let output = r#"Supported languages:
  python     - Python client/server code
  typescript - TypeScript client/server code
  rust       - Rust client/server code
  openapi    - OpenAPI 3.0 specification (JSON + YAML)

Aliases:
  py  -> python
  ts  -> typescript
  rs  -> rust
  oas -> openapi"#;
    Ok(output.to_string())
}

fn cmd_help() -> CmdResult {
    let help = r#"TypeSpec Generator - Generate code from TypeSpec definitions

Usage: adi tsp-gen <command> [options]

Commands:
  generate   Generate code from TypeSpec files
  languages  List supported target languages
  help       Show this help message

Generate Options:
  <input...>                     Input TypeSpec file(s)
  -l, --language <lang>          Target language (required)
  -o, --output <dir>             Output directory (default: generated)
  -s, --side <side>              client, server, both, types, adi, protocol
  -p, --package <name>           Package name (default: api)
  -w, --watch                    Watch input files and regenerate on changes

Protocol Options (for -s protocol):
  --protocol-tag <field>         Discriminant field name (default: type)
  --protocol-rename <strategy>   Rename strategy: snake_case, camelCase, PascalCase (default: snake_case)
  --protocol-enum-name <name>    Generated enum/union type name (default: SignalingMessage)

Examples:
  adi tsp-gen generate api.tsp -l python
  adi tsp-gen generate *.tsp -l typescript -o src/generated -s client
  adi tsp-gen generate main.tsp -l rust -p my_api
  adi tsp-gen generate spec.tsp -l openapi
  adi tsp-gen generate api.tsp -l typescript -o ./out --watch
  adi tsp-gen generate signaling.tsp -l typescript -s protocol --protocol-enum-name SignalingMessage
  adi tsp-gen generate signaling.tsp -l rust -s protocol --protocol-tag type"#;
    Ok(help.to_string())
}

// === Helper Functions ===

fn parse_language(s: &str) -> Result<Language, String> {
    match s.to_lowercase().as_str() {
        "python" | "py" => Ok(Language::Python),
        "typescript" | "ts" => Ok(Language::TypeScript),
        "rust" | "rs" => Ok(Language::Rust),
        "openapi" | "oas" => Ok(Language::OpenApi),
        _ => Err(format!(
            "Unknown language: {}. Use: python, typescript, rust, or openapi",
            s
        )),
    }
}

fn parse_side(s: &str) -> Result<Side, String> {
    match s.to_lowercase().as_str() {
        "client" => Ok(Side::Client),
        "server" => Ok(Side::Server),
        "both" => Ok(Side::Both),
        "types" => Ok(Side::Types),
        "adi" | "adi-service" => Ok(Side::AdiService),
        "protocol" | "proto" => Ok(Side::Protocol),
        _ => Err(format!(
            "Unknown side: {}. Use: client, server, both, types, adi, or protocol",
            s
        )),
    }
}

/// Recursively resolve imports from a TypeSpec file
fn resolve_imports(
    file: TypeSpecFile,
    base_path: &Path,
    resolved: &mut HashSet<PathBuf>,
) -> Result<TypeSpecFile, String> {
    let mut combined = TypeSpecFile {
        imports: Vec::new(),
        usings: file.usings,
        namespace: file.namespace,
        declarations: file.declarations,
    };

    // Process each import
    for import in file.imports {
        // Skip TypeSpec standard library imports
        if import.path.starts_with("@typespec/") {
            continue;
        }

        // Resolve the import path relative to the current file
        let import_path = base_path.join(&import.path);

        // Normalize path and add .tsp extension if missing
        let import_path = if import_path.extension().is_none() {
            import_path.with_extension("tsp")
        } else {
            import_path
        };

        // Canonicalize to handle .. and .
        let import_path = import_path.canonicalize().unwrap_or(import_path);

        // Skip if already resolved (prevents circular imports)
        if resolved.contains(&import_path) {
            continue;
        }
        resolved.insert(import_path.clone());

        // Read and parse the imported file
        if import_path.exists() {
            let source = std::fs::read_to_string(&import_path)
                .map_err(|e| format!("Failed to read import {}: {}", import_path.display(), e))?;

            let imported = parse(&source)
                .map_err(|e| format!("Failed to parse import {}: {}", import_path.display(), e))?;

            // Recursively resolve imports from the imported file
            let import_dir = import_path.parent().unwrap_or(Path::new("."));
            let resolved_import = resolve_imports(imported, import_dir, resolved)?;

            // Merge declarations from the imported file
            combined.usings.extend(resolved_import.usings);
            combined.declarations.extend(resolved_import.declarations);
        }
    }

    Ok(combined)
}
