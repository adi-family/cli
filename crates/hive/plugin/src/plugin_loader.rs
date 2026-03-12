//! Dynamic loader for hive runner plugins from the installed plugins directory.
//!
//! Scans `~/.local/share/adi/plugins/` for `hive.runner.*` directories,
//! loads their dylibs, and registers them into hive-core's PluginManager.

use lib_plugin_abi_v3::runner::Runner;
use lib_plugin_manifest::PluginManifest;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

/// Discover and load hive plugins from the installed plugins directory.
pub async fn load_hive_plugins() {
    let plugins_dir = default_plugins_dir();
    if !plugins_dir.exists() {
        return;
    }

    hive_core::init_global_plugins().await;

    let entries = match std::fs::read_dir(&plugins_dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Failed to read plugins directory: {}", e);
            return;
        }
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("hive.runner.") {
            continue;
        }

        match load_runner_plugin(&dir).await {
            Ok(()) => info!("Loaded hive runner plugin: {}", name),
            Err(e) => warn!("Failed to load runner plugin {}: {}", name, e),
        }
    }
}

async fn load_runner_plugin(plugin_dir: &Path) -> anyhow::Result<()> {
    let resolved_dir = resolve_plugin_dir(plugin_dir)?;

    let manifest_path = resolved_dir.join("plugin.toml");
    if !manifest_path.exists() {
        anyhow::bail!("No plugin.toml found in {}", resolved_dir.display());
    }
    let manifest = PluginManifest::from_file(&manifest_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse plugin manifest: {}", e))?;

    let lib_path = resolve_binary_path(&manifest, &resolved_dir)?;

    // Load the dynamic library (catch_unwind guards against broken dylibs)
    let library = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        libloading::Library::new(&lib_path)
    }))
    .map_err(|_| anyhow::anyhow!("Library::new panicked for {}", lib_path.display()))?
    .map_err(|e| anyhow::anyhow!("Failed to load library {}: {}", lib_path.display(), e))?;

    // ABI version check
    if let Ok(abi_fn) = unsafe {
        library.get::<extern "C" fn() -> u32>(b"plugin_abi_version")
    } {
        let version = abi_fn();
        if version != lib_plugin_abi_v3::PLUGIN_API_VERSION {
            anyhow::bail!(
                "ABI mismatch: plugin v{}, host v{}. Reinstall the plugin.",
                version,
                lib_plugin_abi_v3::PLUGIN_API_VERSION
            );
        }
    }

    // Load the runner-specific FFI symbol
    let create_fn: libloading::Symbol<fn() -> Box<dyn Runner>> = unsafe {
        library.get(b"plugin_create_runner")
    }
    .map_err(|e| anyhow::anyhow!(
        "Plugin does not export plugin_create_runner: {}. Reinstall the plugin.",
        e
    ))?;

    let runner = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| create_fn()))
        .map_err(|_| anyhow::anyhow!("plugin_create_runner panicked — likely ABI-incompatible"))?;

    let runner: Arc<dyn Runner> = Arc::from(runner);
    hive_core::plugin_manager()
        .register_dynamic_runner(runner)
        .await;

    // Keep the library loaded for the lifetime of the process.
    // Dropping it would unload the dylib and invalidate the runner's vtable.
    std::mem::forget(library);

    Ok(())
}

/// Resolve the plugin binary path, trying platform-specific name candidates.
fn resolve_binary_path(manifest: &PluginManifest, dir: &Path) -> anyhow::Result<PathBuf> {
    let name = &manifest.binary.name;
    let candidates = if cfg!(target_os = "macos") {
        vec![format!("lib{name}.dylib"), format!("{name}.dylib")]
    } else if cfg!(target_os = "windows") {
        vec![format!("{name}.dll")]
    } else {
        vec![format!("lib{name}.so"), format!("{name}.so")]
    };
    for candidate in &candidates {
        let path = dir.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("Plugin binary not found in {} (tried: {})", dir.display(), candidates.join(", "))
}

/// Resolve versioned plugin directory (follows `.version` file or `latest` symlink).
fn resolve_plugin_dir(plugin_dir: &Path) -> anyhow::Result<PathBuf> {
    let latest_link = plugin_dir.join("latest");
    if latest_link.is_symlink() {
        if let Ok(resolved) = std::fs::canonicalize(&latest_link) {
            return Ok(resolved);
        }
    }

    let version_file = plugin_dir.join(".version");
    if version_file.exists() {
        if let Ok(version) = std::fs::read_to_string(&version_file) {
            let version = version.trim();
            let versioned_dir = plugin_dir.join(version);
            if versioned_dir.exists() {
                return Ok(versioned_dir);
            }
        }
    }

    Ok(plugin_dir.to_path_buf())
}

fn default_plugins_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("adi")
        .join("plugins")
}
