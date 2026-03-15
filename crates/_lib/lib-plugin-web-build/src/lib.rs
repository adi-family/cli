//! Build-time codegen for ADI web plugins.
//!
//! Generates TypeScript files from Cargo.toml metadata and optional `.tsp` definitions:
//! - `config.ts` — plugin constants (PLUGIN_ID, PLUGIN_NAME, PLUGIN_VERSION, PLUGIN_TYPE)
//! - `types.ts` — types-only entry: plugin type re-export, config, generated types, PluginApiRegistry augmentation
//! - `index.ts` — build entry point with PluginShell export (runtime code)
//! - `generated/` — all codegen output from `.tsp` (models, enums, adi-client, bus-types, bus-events)
//!
//! # Example
//! ```ignore
//! // build.rs
//! fn main() {
//!     plugin_web_build::PluginWebBuild::new().run();
//! }
//! ```

use std::fmt::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use convert_case::{Case, Casing};
use typespec_api::codegen::ts_eventbus::EventBusConfig;
use typespec_api::codegen::{Generator, Language, Side};

/// Builder for plugin web codegen in `build.rs` scripts.
pub struct PluginWebBuild {
    tsp_path: PathBuf,
    output_dir: PathBuf,
    plugin_class: Option<String>,
}

impl PluginWebBuild {
    pub fn new() -> Self {
        Self {
            tsp_path: PathBuf::from("../api.tsp"),
            output_dir: PathBuf::from("../web/src"),
            plugin_class: None,
        }
    }

    /// Override the `.tsp` file path (relative to plugin crate).
    pub fn tsp_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.tsp_path = path.into();
        self
    }

    /// Override the web source output directory (relative to plugin crate).
    pub fn output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = dir.into();
        self
    }

    /// Override the TypeScript plugin class name.
    /// If not set, read from Cargo.toml `web_ui.plugin_class`,
    /// or auto-derived from plugin name ("ADI Router" → "RouterPlugin").
    pub fn plugin_class(mut self, name: impl Into<String>) -> Self {
        self.plugin_class = Some(name.into());
        self
    }

    /// Run generation. Panics on error (standard for build.rs).
    pub fn run(self) {
        if let Err(e) = self.run_inner() {
            panic!("plugin web codegen failed: {e}");
        }
    }

    fn run_inner(self) -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().expect("no current dir"));

        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=Cargo.toml");

        let cargo_path = manifest_dir.join("Cargo.toml");
        let manifest_str = std::fs::read_to_string(&cargo_path)
            .with_context(|| format!("read {}", cargo_path.display()))?;
        let meta = PluginMeta::parse(&manifest_str)?;

        let output_dir = manifest_dir.join(&self.output_dir);
        let generated_dir = output_dir.join("generated");

        let plugin_class = self
            .plugin_class
            .or(meta.plugin_class.clone())
            .unwrap_or_else(|| derive_plugin_class(&meta.name));

        // TSP codegen (optional — only if .tsp exists)
        let tsp_path = manifest_dir.join(&self.tsp_path);
        let has_generated = tsp_path.exists();

        if has_generated {
            println!("cargo:rerun-if-changed={}", tsp_path.display());

            // Always generate types (models.ts, enums.ts) into generated/
            generate_types(&tsp_path, &generated_dir, &meta)?;

            // Generate eventbus (bus-types.ts, bus-events.ts) into generated/
            let has_eventbus = generate_eventbus(&tsp_path, &generated_dir, &meta)?;

            // Generate adi-client into generated/
            let has_adi_client = generate_adi_client(&tsp_path, &generated_dir, &meta)?;

            // Generate generated/index.ts re-exporting everything
            write_if_changed(
                &generated_dir.join("index.ts"),
                &generate_generated_index_ts(&generated_dir, has_eventbus, has_adi_client),
            );
        }

        // config.ts
        write_if_changed(
            &output_dir.join("config.ts"),
            &generate_config_ts(&meta),
        );

        // types.ts — types-only entry for cross-plugin imports
        write_if_changed(
            &output_dir.join("types.ts"),
            &generate_types_ts(&plugin_class, &meta.id, has_generated),
        );

        // index.ts — build entry point with PluginShell export
        write_if_changed(
            &output_dir.join("index.ts"),
            &generate_index_ts(&plugin_class, has_generated),
        );

        Ok(())
    }
}

impl Default for PluginWebBuild {
    fn default() -> Self {
        Self::new()
    }
}

// ── Cargo.toml metadata ────────────────────────────────────

struct PluginMeta {
    id: String,
    name: String,
    version: String,
    plugin_type: String,
    plugin_class: Option<String>,
}

impl PluginMeta {
    fn parse(cargo_toml: &str) -> Result<Self> {
        let table: toml::Value = toml::from_str(cargo_toml).context("parse Cargo.toml")?;

        let pkg = table.get("package").context("missing [package]")?;

        // Handle workspace-inherited version by falling back to CARGO_PKG_VERSION
        let version = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| {
                std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into())
            });

        let meta = pkg
            .get("metadata")
            .and_then(|m| m.get("plugin"))
            .context("missing [package.metadata.plugin]")?;

        let id = meta
            .get("id")
            .and_then(|v| v.as_str())
            .context("missing metadata.plugin.id")?
            .to_string();
        let name = meta
            .get("name")
            .and_then(|v| v.as_str())
            .context("missing metadata.plugin.name")?
            .to_string();
        let plugin_type = meta
            .get("type")
            .and_then(|v| v.as_str())
            .context("missing metadata.plugin.type")?
            .to_string();

        let plugin_class = meta
            .get("web_ui")
            .and_then(|w| w.get("plugin_class"))
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok(Self {
            id,
            name,
            version,
            plugin_type,
            plugin_class,
        })
    }
}

// ── Code generation ─────────────────────────────────────────

/// Generate models.ts and enums.ts into the generated directory.
fn generate_types(tsp_path: &Path, generated_dir: &Path, meta: &PluginMeta) -> Result<()> {
    let source =
        std::fs::read_to_string(tsp_path).with_context(|| format!("read {}", tsp_path.display()))?;
    let file =
        typespec_api::parse(&source).with_context(|| format!("parse {}", tsp_path.display()))?;

    let package_name = meta.id.rsplit('.').next().unwrap_or(&meta.id);

    Generator::new(&file, generated_dir, package_name)
        .generate(Language::TypeScript, Side::Types)
        .context("types codegen failed")?;

    Ok(())
}

/// Generate bus-types.ts and bus-events.ts into the generated directory.
/// Returns true if any @bus interfaces were found and generated.
fn generate_eventbus(tsp_path: &Path, generated_dir: &Path, meta: &PluginMeta) -> Result<bool> {
    let source =
        std::fs::read_to_string(tsp_path).with_context(|| format!("read {}", tsp_path.display()))?;
    let file =
        typespec_api::parse(&source).with_context(|| format!("parse {}", tsp_path.display()))?;

    // Check if there are any @bus interfaces
    let has_bus = file.interfaces().any(|i| i.decorators.iter().any(|d| d.name == "bus"));
    if !has_bus {
        return Ok(false);
    }

    let config = EventBusConfig {
        module_path: "@adi-family/sdk-plugin/types".into(),
        interface_name: "EventRegistry".into(),
        rename: "kebab-case".into(),
    };

    let package_name = meta.id.rsplit('.').next().unwrap_or(&meta.id);

    Generator::new(&file, generated_dir, package_name)
        .with_eventbus_config(config)
        .generate(Language::TypeScript, Side::EventBus)
        .context("eventbus codegen failed")?;

    Ok(true)
}

/// Generate adi-client.ts into the generated directory.
/// Returns true if any @channel interfaces were found and generated.
fn generate_adi_client(tsp_path: &Path, generated_dir: &Path, meta: &PluginMeta) -> Result<bool> {
    let source =
        std::fs::read_to_string(tsp_path).with_context(|| format!("read {}", tsp_path.display()))?;
    let file =
        typespec_api::parse(&source).with_context(|| format!("parse {}", tsp_path.display()))?;

    let package_name = meta.id.rsplit('.').next().unwrap_or(&meta.id);

    let generated = Generator::new(&file, generated_dir, package_name)
        .generate(Language::TypeScript, Side::AdiService)
        .context("adi-client codegen failed")?;

    Ok(!generated.is_empty())
}

fn generate_config_ts(meta: &PluginMeta) -> String {
    let mut out = String::new();
    writeln!(out, "/**").unwrap();
    writeln!(out, " * Auto-generated plugin config from Cargo.toml.").unwrap();
    writeln!(out, " * DO NOT EDIT.").unwrap();
    writeln!(out, " */").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "export const PLUGIN_ID = '{}';", meta.id).unwrap();
    writeln!(out, "export const PLUGIN_NAME = '{}';", meta.name).unwrap();
    writeln!(out, "export const PLUGIN_VERSION = '{}';", meta.version).unwrap();
    writeln!(out, "export const PLUGIN_TYPE = '{}';", meta.plugin_type).unwrap();
    out
}

fn generate_types_ts(
    plugin_class: &str,
    plugin_id: &str,
    has_generated: bool,
) -> String {
    let mut out = String::new();
    writeln!(out, "/**").unwrap();
    writeln!(out, " * Auto-generated plugin types.").unwrap();
    writeln!(out, " * Import via: import '@adi-family/plugin-xxx'").unwrap();
    writeln!(out, " * DO NOT EDIT.").unwrap();
    writeln!(out, " */").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "import type {{ {plugin_class} }} from './plugin';").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "export type {{ {plugin_class} }};").unwrap();
    writeln!(out, "export * from './config';").unwrap();

    if has_generated {
        writeln!(out, "export * from './generated';").unwrap();
    }

    writeln!(out).unwrap();
    writeln!(out, "declare module '@adi-family/sdk-plugin' {{").unwrap();
    writeln!(out, "  interface PluginApiRegistry {{").unwrap();
    writeln!(out, "    '{plugin_id}': {plugin_class}['api'];").unwrap();
    writeln!(out, "  }}").unwrap();
    writeln!(out, "}}").unwrap();
    out
}

fn generate_index_ts(plugin_class: &str, has_generated: bool) -> String {
    let mut out = String::new();
    writeln!(out, "/**").unwrap();
    writeln!(out, " * Auto-generated plugin build entry.").unwrap();
    writeln!(out, " * DO NOT EDIT.").unwrap();
    writeln!(out, " */").unwrap();
    writeln!(out).unwrap();

    if has_generated {
        writeln!(out, "import './generated';").unwrap();
        writeln!(out, "export * from './generated';").unwrap();
    }

    writeln!(out, "export * from './config';").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "export {{ {plugin_class}, {plugin_class} as PluginShell }} from './plugin';"
    )
    .unwrap();
    out
}

/// Generate the index.ts for the generated/ directory.
fn generate_generated_index_ts(
    generated_dir: &Path,
    has_eventbus: bool,
    has_adi_client: bool,
) -> String {
    let mut out = String::new();
    writeln!(out, "/**").unwrap();
    writeln!(out, " * Auto-generated from TypeSpec.").unwrap();
    writeln!(out, " * DO NOT EDIT.").unwrap();
    writeln!(out, " */").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "export * from './models';").unwrap();
    writeln!(out, "export * from './enums';").unwrap();

    if has_eventbus {
        // Prefer bus/ directory (complete output) over flat bus-types.ts
        if generated_dir.join("bus").is_dir() {
            writeln!(out, "export * from './bus';").unwrap();
        } else {
            writeln!(out, "export * from './bus-types';").unwrap();
        }
        writeln!(out, "import './bus-events';").unwrap();
    }

    if has_adi_client {
        // Re-export adi-client under a namespace to avoid name collisions
        writeln!(out, "export * as adiClient from './adi-client';").unwrap();
    }

    out
}

/// Derive plugin class name from display name.
/// "ADI Router" → "RouterPlugin", "ADI Debug Screen" → "DebugScreenPlugin"
fn derive_plugin_class(name: &str) -> String {
    let stripped = name.strip_prefix("ADI ").unwrap_or(name);
    let pascal = stripped.to_case(Case::Pascal);
    format!("{pascal}Plugin")
}

/// Write file only if content differs. Creates parent directories.
pub fn write_if_changed(path: &Path, content: &str) {
    let needs_write = match std::fs::read_to_string(path) {
        Ok(existing) => existing != content,
        Err(_) => true,
    };
    if needs_write {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dirs");
        }
        std::fs::write(path, content).expect("write generated file");
    }
}
