//! Watcher Runner Plugin for Hive
//!
//! Executes a command and restarts it when watched files change.
//! Similar to `cargo-watch` or `nodemon` but integrated into hive.
//!
//! ## Configuration
//!
//! ```yaml
//! runner:
//!   type: watcher
//!   watcher:
//!     run: cargo run -p my-service
//!     working_dir: crates/my-service
//!     watch:
//!       - "src/**/*.rs"
//!       - "Cargo.toml"
//!     debounce_ms: 500
//! ```

use anyhow::{anyhow, Context as AnyhowContext, Result as AnyhowResult};
use dashmap::DashMap;
use lib_plugin_abi_v3::{
    async_trait,
    hooks::HookExitStatus,
    runner::{ProcessHandle, Runner, RuntimeContext},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_RUNNER,
};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

const DEFAULT_DEBOUNCE_MS: u64 = 500;

struct WatcherState {
    pid: Arc<RwLock<Option<u32>>>,
    shutdown_tx: mpsc::Sender<()>,
    logs: Arc<RwLock<Vec<String>>>,
}

pub struct WatcherRunnerPlugin {
    states: Arc<DashMap<String, WatcherState>>,
}

impl Default for WatcherRunnerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl WatcherRunnerPlugin {
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
        }
    }

    fn extract_config(config: &serde_json::Value) -> AnyhowResult<WatcherConfig> {
        let watcher_value = config
            .get("watcher")
            .ok_or_else(|| anyhow!("Missing 'watcher' configuration for watcher runner"))?;

        serde_json::from_value(watcher_value.clone())
            .context("Failed to parse watcher runner configuration")
    }

    async fn spawn_process(
        command: &str,
        working_dir: &Path,
        env: &HashMap<String, String>,
        logs: Arc<RwLock<Vec<String>>>,
    ) -> AnyhowResult<Child> {
        let exec_command = format!("exec {}", command);

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&exec_command)
            .current_dir(working_dir)
            .envs(env.iter())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .with_context(|| format!("Failed to spawn: {}", command))?;

        if let Some(stdout) = child.stdout.take() {
            let logs_clone = logs.clone();
            tokio::spawn(async move {
                capture_output(stdout, logs_clone).await;
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let logs_clone = logs.clone();
            tokio::spawn(async move {
                capture_output(stderr, logs_clone).await;
            });
        }

        Ok(child)
    }

    /// Sends SIGTERM then waits up to 5 s before escalating to SIGKILL.
    async fn kill_process(pid: u32) {
        #[cfg(unix)]
        {
            unsafe { libc::kill(pid as i32, libc::SIGTERM) };

            let timeout = Duration::from_secs(5);
            let start = std::time::Instant::now();
            while start.elapsed() < timeout {
                if unsafe { libc::kill(pid as i32, 0) } != 0 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            warn!("Process {} did not exit gracefully, sending SIGKILL", pid);
            unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        }
    }

    /// Extracts the longest non-wildcard prefix from each glob pattern and resolves it
    /// to a directory to watch. Falls back to `working_dir` when no prefix exists.
    fn resolve_watch_dirs(patterns: &[String], working_dir: &Path) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        for pattern in patterns {
            let path = Path::new(pattern);
            let mut prefix = PathBuf::new();
            for component in path.components() {
                let s = component.as_os_str().to_string_lossy();
                if s.contains('*') || s.contains('?') || s.contains('[') {
                    break;
                }
                prefix.push(component);
            }

            let dir = if prefix.as_os_str().is_empty() {
                working_dir.to_path_buf()
            } else {
                let abs = working_dir.join(&prefix);
                if abs.is_dir() {
                    abs
                } else if let Some(parent) = abs.parent() {
                    if parent.is_dir() {
                        parent.to_path_buf()
                    } else {
                        working_dir.to_path_buf()
                    }
                } else {
                    working_dir.to_path_buf()
                }
            };

            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }

        if dirs.is_empty() {
            dirs.push(working_dir.to_path_buf());
        }

        dirs
    }

    fn matches_patterns(path: &Path, patterns: &[glob::Pattern], working_dir: &Path) -> bool {
        let relative = path.strip_prefix(working_dir).unwrap_or(path);
        let relative_str = relative.to_string_lossy();

        patterns.iter().any(|p| p.matches(&relative_str))
    }
}

#[async_trait]
impl Plugin for WatcherRunnerPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.runner.watcher".to_string(),
            name: "Watcher Runner".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some(
                "File watcher runner - restarts services on file changes".to_string(),
            ),
            category: Some(PluginCategory::Runner),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        let keys: Vec<String> = self.states.iter().map(|e| e.key().clone()).collect();
        for key in keys {
            if let Some((_, state)) = self.states.remove(&key) {
                let _ = state.shutdown_tx.send(()).await;
                if let Some(pid) = *state.pid.read().await {
                    Self::kill_process(pid).await;
                }
            }
        }
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_RUNNER]
    }
}

#[async_trait]
impl Runner for WatcherRunnerPlugin {
    async fn start(
        &self,
        service_name: &str,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> PluginResult<ProcessHandle> {
        let watcher_config = Self::extract_config(config)?;

        let working_dir = match &watcher_config.working_dir {
            Some(dir) => ctx.working_dir.join(dir),
            None => ctx.working_dir.clone(),
        };

        let compiled_patterns: Vec<glob::Pattern> = watcher_config
            .watch
            .iter()
            .map(|p| {
                glob::Pattern::new(p)
                    .with_context(|| format!("Invalid glob pattern: {}", p))
            })
            .collect::<AnyhowResult<Vec<_>>>()?;

        let debounce = Duration::from_millis(
            watcher_config.debounce_ms.unwrap_or(DEFAULT_DEBOUNCE_MS),
        );

        let logs = Arc::new(RwLock::new(Vec::new()));
        let current_pid: Arc<RwLock<Option<u32>>> = Arc::new(RwLock::new(None));
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let child =
            Self::spawn_process(&watcher_config.run, &working_dir, &env, logs.clone()).await?;
        let initial_pid = child.id();
        *current_pid.write().await = initial_pid;

        info!(
            "Watcher started for {} (PID: {:?}, watching {} patterns)",
            service_name,
            initial_pid,
            watcher_config.watch.len()
        );

        self.states.insert(
            service_name.to_string(),
            WatcherState {
                pid: current_pid.clone(),
                shutdown_tx: shutdown_tx.clone(),
                logs: logs.clone(),
            },
        );

        let command = watcher_config.run.clone();
        let watch_dirs = Self::resolve_watch_dirs(&watcher_config.watch, &working_dir);
        let wd = working_dir.clone();
        let env_clone = env.clone();
        let pid_clone = current_pid.clone();
        let logs_clone = logs.clone();
        let svc_name = service_name.to_string();

        tokio::spawn(async move {
            let (fs_tx, mut fs_rx) = mpsc::unbounded_channel::<()>();

            let _watcher = {
                let fs_tx = fs_tx.clone();
                let patterns = compiled_patterns.clone();
                let working_dir = wd.clone();

                let watcher_result = RecommendedWatcher::new(
                    move |res: std::result::Result<Event, notify::Error>| {
                        if let Ok(event) = res {
                            // Only react to create/modify/remove events
                            let dominated =
                                matches!(event.kind, notify::EventKind::Create(_)
                                    | notify::EventKind::Modify(_)
                                    | notify::EventKind::Remove(_));
                            if !dominated {
                                return;
                            }

                            let any_match = event.paths.iter().any(|p| {
                                Self::matches_patterns(p, &patterns, &working_dir)
                            });

                            if any_match {
                                let _ = fs_tx.send(());
                            }
                        }
                    },
                    notify::Config::default(),
                );

                let mut watcher = match watcher_result {
                    Ok(w) => w,
                    Err(e) => {
                        error!("Failed to create file watcher for {}: {}. Service will run without auto-restart.", svc_name, e);
                        shutdown_rx.recv().await;
                        return;
                    }
                };

                for dir in &watch_dirs {
                    if dir.exists() {
                        if let Err(e) = watcher.watch(dir, RecursiveMode::Recursive) {
                            warn!("Failed to watch directory {:?}: {}", dir, e);
                        } else {
                            debug!("Watching directory: {:?}", dir);
                        }
                    }
                }

                watcher
            };

            let child_handle: Arc<RwLock<Option<Child>>> = Arc::new(RwLock::new(Some(child)));

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        debug!("Watcher shutdown for {}", svc_name);
                        break;
                    }
                    Some(()) = fs_rx.recv() => {
                        tokio::time::sleep(debounce).await;
                        while fs_rx.try_recv().is_ok() {}

                        info!("File change detected, restarting {}", svc_name);

                        if let Some(pid) = *pid_clone.read().await {
                            Self::kill_process(pid).await;
                        }

                        if let Some(mut ch) = child_handle.write().await.take() {
                            let _ = ch.wait().await;
                        }

                        match Self::spawn_process(&command, &wd, &env_clone, logs_clone.clone()).await {
                            Ok(new_child) => {
                                let new_pid = new_child.id();
                                *pid_clone.write().await = new_pid;
                                *child_handle.write().await = Some(new_child);
                                info!("Restarted {} (PID: {:?})", svc_name, new_pid);
                            }
                            Err(e) => {
                                warn!("Failed to restart {}: {}", svc_name, e);
                                *pid_clone.write().await = None;
                            }
                        }
                    }
                }
            }
        });

        let handle = ProcessHandle::script(initial_pid.unwrap_or(0))
            .with_metadata("runner", "watcher")
            .with_metadata("command", &watcher_config.run);

        Ok(handle)
    }

    async fn stop(&self, handle: &ProcessHandle) -> PluginResult<()> {
        let mut found_key = None;
        for entry in self.states.iter() {
            let state_pid = *entry.value().pid.read().await;
            if state_pid == handle.pid {
                found_key = Some(entry.key().clone());
                break;
            }
        }

        if let Some(key) = found_key {
            if let Some((_, state)) = self.states.remove(&key) {
                let _ = state.shutdown_tx.send(()).await;
            }
        }

        if let Some(pid) = handle.pid {
            info!("Stopping watcher process (PID: {})", pid);
            Self::kill_process(pid).await;
        }

        Ok(())
    }

    async fn is_running(&self, handle: &ProcessHandle) -> bool {
        match handle.pid {
            Some(pid) => {
                #[cfg(unix)]
                {
                    unsafe { libc::kill(pid as i32, 0) == 0 }
                }
                #[cfg(not(unix))]
                {
                    false
                }
            }
            None => false,
        }
    }

    async fn logs(
        &self,
        handle: &ProcessHandle,
        lines: Option<usize>,
    ) -> PluginResult<Vec<String>> {
        for entry in self.states.iter() {
            let state_pid = *entry.value().pid.read().await;
            if state_pid == handle.pid {
                let logs = entry.value().logs.read().await;
                return match lines {
                    Some(n) => {
                        let start = logs.len().saturating_sub(n);
                        Ok(logs[start..].to_vec())
                    }
                    None => Ok(logs.clone()),
                };
            }
        }

        Ok(vec![])
    }

    fn supports_hooks(&self) -> bool {
        true
    }

    async fn run_hook(
        &self,
        config: &serde_json::Value,
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> PluginResult<HookExitStatus> {
        let watcher_config = Self::extract_config(config)?;

        let working_dir = match &watcher_config.working_dir {
            Some(dir) => ctx.working_dir.join(dir),
            None => ctx.working_dir.clone(),
        };

        let output = Command::new("sh")
            .arg("-c")
            .arg(&watcher_config.run)
            .current_dir(&working_dir)
            .envs(env.iter())
            .output()
            .await
            .with_context(|| format!("Failed to run hook: {}", watcher_config.run))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(HookExitStatus {
            code: output.status.code().unwrap_or(-1),
            output: if stdout.is_empty() {
                None
            } else {
                Some(stdout)
            },
            stderr: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
        })
    }
}

/// Appends lines from `reader` into `logs`, capping the buffer at 10 000 lines.
async fn capture_output<R: tokio::io::AsyncRead + Unpin>(reader: R, logs: Arc<RwLock<Vec<String>>>) {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    const MAX_LOG_LINES: usize = 10_000;

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim_end().to_string();
                if !trimmed.is_empty() {
                    let mut logs = logs.write().await;
                    logs.push(trimmed);
                    if logs.len() > MAX_LOG_LINES {
                        let drain_count = logs.len() - MAX_LOG_LINES;
                        logs.drain(..drain_count);
                    }
                }
            }
            Err(e) => {
                debug!("Error reading output: {}", e);
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WatcherConfig {
    pub run: String,
    /// Relative to the project root.
    pub working_dir: Option<String>,
    pub watch: Vec<String>,
    /// Default: 500 ms.
    pub debounce_ms: Option<u64>,
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(WatcherRunnerPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = WatcherRunnerPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.runner.watcher");
        assert_eq!(meta.name, "Watcher Runner");
    }

    #[test]
    fn test_extract_config() {
        let config = serde_json::json!({
            "watcher": {
                "run": "cargo run",
                "working_dir": "crates/my-service",
                "watch": ["src/**/*.rs", "Cargo.toml"],
                "debounce_ms": 300
            }
        });

        let watcher_config = WatcherRunnerPlugin::extract_config(&config).unwrap();
        assert_eq!(watcher_config.run, "cargo run");
        assert_eq!(watcher_config.working_dir.as_deref(), Some("crates/my-service"));
        assert_eq!(watcher_config.watch.len(), 2);
        assert_eq!(watcher_config.debounce_ms, Some(300));
    }

    #[test]
    fn test_extract_config_defaults() {
        let config = serde_json::json!({
            "watcher": {
                "run": "npm start",
                "watch": ["src/**/*.ts"]
            }
        });

        let watcher_config = WatcherRunnerPlugin::extract_config(&config).unwrap();
        assert_eq!(watcher_config.run, "npm start");
        assert!(watcher_config.working_dir.is_none());
        assert!(watcher_config.debounce_ms.is_none());
    }

    #[test]
    fn test_resolve_watch_dirs() {
        let working_dir = PathBuf::from("/project");

        // Pattern with static prefix
        let dirs = WatcherRunnerPlugin::resolve_watch_dirs(
            &["src/**/*.rs".to_string()],
            &working_dir,
        );
        // Will resolve to /project/src if it doesn't exist, falls back
        assert!(!dirs.is_empty());

        // Pattern with no prefix (wildcard at root)
        let dirs = WatcherRunnerPlugin::resolve_watch_dirs(
            &["*.toml".to_string()],
            &working_dir,
        );
        assert_eq!(dirs[0], working_dir);
    }

    #[test]
    fn test_matches_patterns() {
        let working_dir = PathBuf::from("/project");
        let patterns = vec![
            glob::Pattern::new("src/**/*.rs").unwrap(),
            glob::Pattern::new("Cargo.toml").unwrap(),
        ];

        assert!(WatcherRunnerPlugin::matches_patterns(
            Path::new("/project/src/main.rs"),
            &patterns,
            &working_dir,
        ));
        assert!(WatcherRunnerPlugin::matches_patterns(
            Path::new("/project/Cargo.toml"),
            &patterns,
            &working_dir,
        ));
        assert!(!WatcherRunnerPlugin::matches_patterns(
            Path::new("/project/README.md"),
            &patterns,
            &working_dir,
        ));
    }
}
