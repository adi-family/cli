use lib_plugin_abi_v3::PluginContext;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

struct PluginCtxInner {
    plugin_id: String,
    data_dir: PathBuf,
    config_dir: PathBuf,
    config: Value,
}

static CTX: OnceLock<PluginCtxInner> = OnceLock::new();

/// Static plugin context accessor.
///
/// Call [`PluginCtx::init`] once in your plugin's `init()` method,
/// then use the static accessors from anywhere (including free functions).
///
/// Each cdylib plugin has its own isolated static — no cross-plugin interference.
pub struct PluginCtx;

impl PluginCtx {
    /// Initialize the static plugin context from the host-provided [`PluginContext`].
    ///
    /// Call this in your plugin's `init()`. Subsequent calls are no-ops.
    pub fn init(ctx: &PluginContext) {
        let _ = CTX.set(PluginCtxInner {
            plugin_id: ctx.plugin_id.clone(),
            data_dir: ctx.data_dir.clone(),
            config_dir: ctx.config_dir.clone(),
            config: ctx.config.clone(),
        });
    }

    fn inner() -> &'static PluginCtxInner {
        CTX.get()
            .expect("PluginCtx not initialized — call PluginCtx::init(ctx) in init()")
    }

    /// Plugin data directory (e.g. `~/.local/share/adi/<plugin-id>/`).
    pub fn data_dir() -> &'static Path {
        &Self::inner().data_dir
    }

    /// Plugin config directory (e.g. `~/.config/adi/<plugin-id>/`).
    pub fn config_dir() -> &'static Path {
        &Self::inner().config_dir
    }

    /// Plugin identifier (e.g. `"adi.hive"`).
    pub fn plugin_id() -> &'static str {
        &Self::inner().plugin_id
    }

    /// Plugin configuration loaded from `config.json`.
    pub fn config() -> &'static Value {
        &Self::inner().config
    }
}
