//! Build-time code generation API for use in `build.rs` scripts.
//!
//! Provides a builder-pattern API that parses `.tsp` files, resolves imports,
//! generates code, and only writes files when content has changed.

use crate::ast::TypeSpecFile;
use crate::codegen::rust::{RustAdiServiceConfig, RustServerConfig};
use crate::codegen::{Generator, Language, Side};
use crate::parse;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Builder for build-time TSP code generation.
///
/// # Example (in build.rs)
/// ```ignore
/// use typespec_api::build::Generate;
/// use typespec_api::Side;
///
/// fn main() {
///     Generate::new("../api.tsp")
///         .side(Side::Types)
///         .package("my-types")
///         .run();
/// }
/// ```
pub struct Generate {
    tsp_path: PathBuf,
    output_dir: PathBuf,
    language: Language,
    side: Side,
    package: String,
    server_config: RustServerConfig,
    adi_config: Option<RustAdiServiceConfig>,
    types_crate: Option<String>,
    /// When set, concatenate all generated files into a single `{name}.rs` in OUT_DIR.
    /// Enables `include!(concat!(env!("OUT_DIR"), "/{name}.rs"))` pattern.
    out_dir_file: Option<String>,
}

impl Generate {
    pub fn new(tsp_path: impl Into<PathBuf>) -> Self {
        Self {
            tsp_path: tsp_path.into(),
            output_dir: PathBuf::from("src/generated"),
            language: Language::Rust,
            side: Side::Both,
            package: "api".into(),
            server_config: RustServerConfig::default(),
            adi_config: None,
            types_crate: None,
            out_dir_file: None,
        }
    }

    pub fn output(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = dir.into();
        self
    }

    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    pub fn package(mut self, name: impl Into<String>) -> Self {
        self.package = name.into();
        self
    }

    pub fn types_crate(mut self, name: impl Into<String>) -> Self {
        self.types_crate = Some(name.into());
        self
    }

    pub fn server_config(mut self, config: RustServerConfig) -> Self {
        self.server_config = config;
        self
    }

    pub fn adi_config(mut self, config: RustAdiServiceConfig) -> Self {
        self.adi_config = Some(config);
        self
    }

    /// Generate to `OUT_DIR/{name}.rs` as a single concatenated file.
    ///
    /// Use with `include!(concat!(env!("OUT_DIR"), "/{name}.rs"))` in `lib.rs`.
    /// This keeps generated code out of the source tree entirely.
    pub fn output_to_out_dir(mut self, name: &str) -> Self {
        self.out_dir_file = Some(name.to_string());
        self
    }

    /// Run generation. Intended for use in `build.rs`.
    ///
    /// - Parses the `.tsp` file with recursive import resolution
    /// - Generates code to a temp directory
    /// - Compares with existing files and only writes if content changed
    /// - Emits `cargo:rerun-if-changed` for source files
    pub fn run(self) {
        if let Err(e) = self.run_inner() {
            panic!("typespec code generation failed: {e}");
        }
    }

    fn run_inner(self) -> Result<()> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().expect("no current dir"));

        let tsp_path = if self.tsp_path.is_relative() {
            manifest_dir.join(&self.tsp_path)
        } else {
            self.tsp_path.clone()
        };

        let tsp_canonical = tsp_path
            .canonicalize()
            .with_context(|| format!("failed to resolve tsp path: {}", tsp_path.display()))?;

        // Emit rerun-if-changed for the main .tsp file and build.rs
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", tsp_canonical.display());

        // Parse with import resolution, collecting all resolved file paths
        let mut resolved_paths = HashSet::new();
        resolved_paths.insert(tsp_canonical.clone());

        let source = std::fs::read_to_string(&tsp_canonical)
            .with_context(|| format!("failed to read {}", tsp_canonical.display()))?;

        let file = parse(&source)
            .with_context(|| format!("failed to parse {}", tsp_canonical.display()))?;

        let base_dir = tsp_canonical.parent().unwrap_or(Path::new("."));
        let combined = resolve_imports(file, base_dir, &mut resolved_paths)?;

        // Emit rerun-if-changed for each resolved import
        for path in &resolved_paths {
            println!("cargo:rerun-if-changed={}", path.display());
        }

        // Generate to a temp directory inside OUT_DIR (available in build scripts)
        let out_dir = std::env::var("OUT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let temp_output = out_dir.join("tsp-gen-tmp");

        // Clean previous temp output to avoid stale files
        let _ = std::fs::remove_dir_all(&temp_output);

        let mut generator = Generator::new(&combined, &temp_output, &self.package)
            .with_rust_config(self.server_config);

        if let Some(tc) = &self.types_crate {
            generator = generator.with_types_crate(tc.clone());
        }
        if let Some(adi) = self.adi_config {
            generator = generator.with_rust_adi_config(adi);
        }

        generator
            .generate(self.language, self.side)
            .context("code generation failed")?;

        if let Some(ref file_name) = self.out_dir_file {
            // OUT_DIR mode: concatenate all generated files into a single file
            let combined = collect_generated_files(&temp_output)?;
            let out_path = out_dir.join(format!("{file_name}.rs"));

            // Only write if content changed
            let needs_write = match std::fs::read_to_string(&out_path) {
                Ok(existing) => existing != combined,
                Err(_) => true,
            };
            if needs_write {
                std::fs::write(&out_path, &combined)
                    .with_context(|| format!("failed to write {}", out_path.display()))?;
                println!("cargo:warning=Regenerated {}", out_path.display());
            }
        } else {
            // Normal mode: sync files to output directory
            let output_dir = if self.output_dir.is_relative() {
                manifest_dir.join(&self.output_dir)
            } else {
                self.output_dir.clone()
            };

            std::fs::create_dir_all(&output_dir)
                .with_context(|| format!("failed to create output dir: {}", output_dir.display()))?;

            sync_generated_files(&temp_output, &output_dir)?;
        }

        Ok(())
    }
}

/// Recursively resolve imports from a TypeSpec file, tracking all resolved paths.
fn resolve_imports(
    file: TypeSpecFile,
    base_path: &Path,
    resolved: &mut HashSet<PathBuf>,
) -> Result<TypeSpecFile> {
    let mut combined = TypeSpecFile {
        imports: Vec::new(),
        usings: file.usings,
        namespace: file.namespace,
        declarations: file.declarations,
    };

    for import in file.imports {
        if import.path.starts_with("@typespec/") {
            continue;
        }

        let import_path = base_path.join(&import.path);
        let import_path = if import_path.extension().is_none() {
            import_path.with_extension("tsp")
        } else {
            import_path
        };
        let import_path = import_path.canonicalize().unwrap_or(import_path);

        if resolved.contains(&import_path) {
            continue;
        }
        resolved.insert(import_path.clone());

        if import_path.exists() {
            let source = std::fs::read_to_string(&import_path)
                .with_context(|| format!("failed to read import {}", import_path.display()))?;

            let imported = parse(&source)
                .with_context(|| format!("failed to parse import {}", import_path.display()))?;

            let import_dir = import_path.parent().unwrap_or(Path::new("."));
            let resolved_import = resolve_imports(imported, import_dir, resolved)?;

            combined.usings.extend(resolved_import.usings);
            combined.declarations.extend(resolved_import.declarations);
        }
    }

    Ok(combined)
}

/// Recursively collect all `.rs` files from a directory into a single concatenated string.
fn collect_generated_files(dir: &Path) -> Result<String> {
    let mut parts = Vec::new();
    collect_files_recursive(dir, &mut parts)?;
    parts.sort_by(|(a, _), (b, _)| a.cmp(b));
    Ok(parts.into_iter().map(|(_, content)| content).collect::<Vec<_>>().join("\n\n"))
}

fn collect_files_recursive(dir: &Path, parts: &mut Vec<(PathBuf, String)>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).context("failed to read generated dir")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, parts)?;
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)?;
            parts.push((path, content));
        }
    }
    Ok(())
}

/// Recursively sync files from `src_dir` to `dst_dir`, only writing when content differs.
fn sync_generated_files(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    if !src_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(src_dir).context("failed to read temp output dir")? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let src_path = entry.path();
        let dst_path = dst_dir.join(&name);

        if file_type.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
            sync_generated_files(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            let new_content = std::fs::read_to_string(&src_path)?;
            let needs_write = match std::fs::read_to_string(&dst_path) {
                Ok(existing) => existing != new_content,
                Err(_) => true,
            };

            if needs_write {
                std::fs::write(&dst_path, &new_content)?;
                println!(
                    "cargo:warning=Regenerated {}",
                    dst_path.display()
                );
            }
        }
    }

    Ok(())
}
