use crate::observability::{EventCollector, LogLevel, LogStream, ObservabilityEvent};
use crate::runtime_db::RuntimeDb;
use crate::sqlite_backend::RuntimeState;
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub enum ProcessType {
    Process,
    Docker { container_name: String },
}

pub struct ProcessHandle {
    process_type: ProcessType,
    pid: Option<u32>,
    container_id: Option<String>,
    child: Option<Child>,
    logs: Arc<RwLock<Vec<String>>>,
}

impl ProcessHandle {
    pub fn from_pid(pid: u32) -> Self {
        Self {
            process_type: ProcessType::Process,
            pid: Some(pid),
            container_id: None,
            child: None,
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn docker(container_name: String) -> Self {
        Self {
            process_type: ProcessType::Docker { container_name },
            pid: None,
            container_id: None,
            child: None,
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn container_id(&self) -> Option<&str> {
        self.container_id.as_deref()
    }

    pub fn process_type(&self) -> &ProcessType {
        &self.process_type
    }

    pub fn is_docker(&self) -> bool {
        matches!(self.process_type, ProcessType::Docker { .. })
    }

    pub fn container_name(&self) -> Option<&str> {
        match &self.process_type {
            ProcessType::Docker { container_name } => Some(container_name),
            _ => None,
        }
    }

    pub async fn is_running(&mut self) -> bool {
        match &self.process_type {
            ProcessType::Process => {
                if let Some(child) = &mut self.child {
                    child.try_wait().ok().flatten().is_none()
                } else {
                    false
                }
            }
            ProcessType::Docker { .. } => {
                // Docker running status is checked via the runner plugin
                true
            }
        }
    }

    pub fn set_container_id(&mut self, id: String) {
        self.container_id = Some(id);
    }
}

impl From<ProcessHandle> for lib_plugin_abi_v3::runner::ProcessHandle {
    fn from(h: ProcessHandle) -> Self {
        match h.process_type {
            ProcessType::Docker { container_name } => Self::docker(container_name),
            ProcessType::Process => h
                .pid
                .map(Self::script)
                .unwrap_or_else(|| Self {
                    id: "unknown".into(),
                    runner_type: "script".into(),
                    pid: None,
                    container_name: None,
                    metadata: std::collections::HashMap::new(),
                }),
        }
    }
}

impl From<lib_plugin_abi_v3::runner::ProcessHandle> for ProcessHandle {
    fn from(h: lib_plugin_abi_v3::runner::ProcessHandle) -> Self {
        if let Some(name) = h.container_name {
            Self::docker(name)
        } else if let Some(pid) = h.pid {
            Self::from_pid(pid)
        } else {
            Self::from_pid(0)
        }
    }
}

pub struct ProcessManager {
    project_root: PathBuf,
    runtime_db: Arc<RuntimeDb>,
    event_collector: Option<Arc<EventCollector>>,
    source_name: String,
}

/// Fallback chain: config value > $SHELL > "sh"
pub fn resolve_shell(config_shell: Option<&str>) -> String {
    lib_plugin_abi_v3::utils::resolve_shell(config_shell, false).program
}

impl ProcessManager {
    pub fn new(project_root: PathBuf, runtime_db: Arc<RuntimeDb>) -> Self {
        Self {
            project_root,
            runtime_db,
            event_collector: None,
            source_name: "cli".to_string(),
        }
    }

    pub fn with_event_collector(
        project_root: PathBuf,
        runtime_db: Arc<RuntimeDb>,
        event_collector: Arc<EventCollector>,
        source_name: String,
    ) -> Self {
        Self {
            project_root,
            runtime_db,
            event_collector: Some(event_collector),
            source_name,
        }
    }

    /// For late binding.
    pub fn set_event_collector(&mut self, collector: Arc<EventCollector>) {
        self.event_collector = Some(collector);
    }

    pub fn set_source_name(&mut self, name: String) {
        self.source_name = name;
    }

    pub fn runtime_db(&self) -> &Arc<RuntimeDb> {
        &self.runtime_db
    }

    pub fn is_pid_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            unsafe { libc::kill(pid as i32, 0) == 0 }
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            false
        }
    }

    pub fn is_service_running(&self, service_name: &str) -> Option<u32> {
        if let Some(pid) = self.runtime_db.read_pid(service_name) {
            if Self::is_pid_running(pid) {
                return Some(pid);
            } else {
                // Stale PID — clear it
                self.runtime_db.clear_pid(service_name).ok();
            }
        }
        None
    }

    pub async fn run_command(
        &self,
        command: &str,
        working_dir: &Path,
        env: &HashMap<String, String>,
        shell: &str,
    ) -> Result<std::process::Output> {
        let output = Command::new(shell)
            .args(["-l", "-c"])
            .arg(command)
            .current_dir(working_dir)
            .envs(env.iter())
            .output()
            .await
            .with_context(|| format!("Failed to run command: {}", command))?;

        Ok(output)
    }

    pub async fn spawn(
        &self,
        command: &str,
        working_dir: &Path,
        env: HashMap<String, String>,
        service_name: &str,
        shell: &str,
    ) -> Result<ProcessHandle> {
        debug!(
            "Spawning process for {}: {} in {:?}",
            service_name, command, working_dir
        );

        // exec replaces the shell so the PID matches the actual process
        let exec_command = format!("exec {}", command);

        let log_dir = self.project_root.join(".adi/hive/logs");
        std::fs::create_dir_all(&log_dir).ok();
        let log_file = log_dir.join(format!("{}.log", service_name));

        // With event collector: pipe stdout/stderr for streaming.
        // Without: write directly to log file (legacy).
        if let Some(event_collector) = &self.event_collector {
            let log_file_handle = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)
                .with_context(|| format!("Failed to create log file: {:?}", log_file))?;

            let mut child = Command::new(shell)
                .args(["-l", "-c"])
                .arg(&exec_command)
                .current_dir(working_dir)
                .envs(env.iter())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                // New process group so it survives parent exit
                .process_group(0)
                .spawn()
                .with_context(|| format!("Failed to spawn process for {}", service_name))?;

            let pid = child.id();
            let logs = Arc::new(RwLock::new(Vec::new()));

            let service_fqn = format!("{}:{}", self.source_name, service_name);

            if let Some(stdout) = child.stdout.take() {
                let collector = event_collector.clone();
                let fqn = service_fqn.clone();
                let log_file_clone = log_file_handle
                    .try_clone()
                    .with_context(|| "Failed to clone log file handle for stdout")?;
                tokio::spawn(async move {
                    Self::capture_output(stdout, collector, fqn, LogStream::Stdout, log_file_clone)
                        .await;
                });
            }

            if let Some(stderr) = child.stderr.take() {
                let collector = event_collector.clone();
                let fqn = service_fqn.clone();
                let log_file_clone = log_file_handle
                    .try_clone()
                    .with_context(|| "Failed to clone log file handle for stderr")?;
                tokio::spawn(async move {
                    Self::capture_output(stderr, collector, fqn, LogStream::Stderr, log_file_clone)
                        .await;
                });
            }

            info!(
                "Process started for {} with PID {:?} (logs: {:?}, streaming enabled)",
                service_name, pid, log_file
            );

            if let Some(pid) = pid {
                self.runtime_db.save_pid(service_name, pid)?;
            }

            Ok(ProcessHandle {
                process_type: ProcessType::Process,
                pid,
                container_id: None,
                child: Some(child),
                logs,
            })
        } else {
            // Legacy: write directly to log file
            let log_out = std::fs::File::create(&log_file)
                .with_context(|| format!("Failed to create log file: {:?}", log_file))?;
            let log_err = log_out
                .try_clone()
                .with_context(|| "Failed to clone log file handle")?;

            let child = Command::new(shell)
                .args(["-l", "-c"])
                .arg(&exec_command)
                .current_dir(working_dir)
                .envs(env.iter())
                .stdout(Stdio::from(log_out))
                .stderr(Stdio::from(log_err))
                // New process group so it survives parent exit
                .process_group(0)
                .spawn()
                .with_context(|| format!("Failed to spawn process for {}", service_name))?;

            let pid = child.id();
            let logs = Arc::new(RwLock::new(Vec::new()));

            info!(
                "Process started for {} with PID {:?} (logs: {:?})",
                service_name, pid, log_file
            );

            if let Some(pid) = pid {
                self.runtime_db.save_pid(service_name, pid)?;
            }

            Ok(ProcessHandle {
                process_type: ProcessType::Process,
                pid,
                container_id: None,
                child: Some(child),
                logs,
            })
        }
    }

    async fn capture_output<R: tokio::io::AsyncRead + Unpin>(
        reader: R,
        event_collector: Arc<EventCollector>,
        service_fqn: String,
        stream: LogStream,
        mut log_file: std::fs::File,
    ) {
        use std::io::Write;

        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let message = line.trim_end();
                    if message.is_empty() {
                        continue;
                    }

                    let level = Self::detect_log_level(message);

                    event_collector.emit(ObservabilityEvent::log(
                        &service_fqn,
                        level,
                        message,
                        stream,
                    ));

                    let _ = writeln!(log_file, "{}", message);
                }
                Err(e) => {
                    debug!("Error reading output for {}: {}", service_fqn, e);
                    break;
                }
            }
        }
    }

    /// Uses word-boundary matching to avoid false positives like "0 errors found".
    fn detect_log_level(message: &str) -> LogLevel {
        // Bracketed indicators are most reliable
        let lower = message.to_lowercase();
        if lower.contains("[err]") || lower.contains("[error]") || lower.contains("[fatal]") {
            return LogLevel::Error;
        }
        if lower.contains("[wrn]") || lower.contains("[warn]") || lower.contains("[warning]") {
            return LogLevel::Warn;
        }
        if lower.contains("[dbg]") || lower.contains("[debug]") {
            return LogLevel::Debug;
        }
        if lower.contains("[trc]") || lower.contains("[trace]") {
            return LogLevel::Trace;
        }
        if lower.contains("[inf]") || lower.contains("[info]") {
            return LogLevel::Info;
        }

        // Word-boundary check avoids false positives like "errorCount", "0 errors"
        let has_level_prefix = |level: &str| -> bool {
            lower.starts_with(level)
                || lower.contains(&format!(" {}", level))
                || lower.contains(&format!(":{}", level))
                || lower.contains(&format!("]{}", level))
        };

        if has_level_prefix("error:") || has_level_prefix("error ") || has_level_prefix("fatal") {
            LogLevel::Error
        } else if has_level_prefix("warn:") || has_level_prefix("warn ") || has_level_prefix("warning") {
            LogLevel::Warn
        } else if has_level_prefix("debug:") || has_level_prefix("debug ") {
            LogLevel::Debug
        } else if has_level_prefix("trace:") || has_level_prefix("trace ") {
            LogLevel::Trace
        } else {
            LogLevel::Info
        }
    }

    /// SIGTERM then SIGKILL after 10s timeout.
    pub async fn stop(&self, mut handle: ProcessHandle, service_name: &str) -> Result<()> {
        if let Some(mut child) = handle.child.take() {
            #[cfg(unix)]
            {
                if let Some(pid) = handle.pid {
                    unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                }
            }

            let timeout = tokio::time::Duration::from_secs(10);
            match tokio::time::timeout(timeout, child.wait()).await {
                Ok(result) => {
                    debug!("Process exited gracefully: {:?}", result);
                }
                Err(_) => {
                    warn!("Process did not exit gracefully, sending SIGKILL");
                    child.kill().await.ok();
                }
            }
        }

        self.runtime_db.update_state(service_name, &RuntimeState {
            state: "stopped".to_string(),
            pid: None,
            container_id: None,
            started_at: None,
            stopped_at: Some(Utc::now().to_rfc3339()),
            restart_count: 0,
            last_exit_code: None,
            last_error: None,
        })?;

        Ok(())
    }

    pub async fn stop_by_name(&self, service_name: &str) -> Result<()> {
        if let Some(pid) = self.runtime_db.read_pid(service_name) {
            info!("Stopping {} (PID: {})", service_name, pid);
            self.kill_pid(pid, service_name).await;
            self.runtime_db.clear_pid(service_name).ok();
        }

        Ok(())
    }

    /// Finds PID via lsof.
    pub async fn stop_by_port(&self, port: u16, service_name: &str) -> Result<bool> {
        if let Some(pid) = Self::find_pid_by_port(port).await {
            info!("Stopping {} on port {} (PID: {})", service_name, port, pid);
            self.kill_pid(pid, service_name).await;
            self.runtime_db.clear_pid(service_name).ok();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn find_pid_by_port(port: u16) -> Option<u32> {
        #[cfg(unix)]
        {
            let output = tokio::process::Command::new("lsof")
                .args(["-i", &format!(":{}", port), "-t", "-sTCP:LISTEN"])
                .output()
                .await
                .ok()?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // lsof -t returns PIDs one per line
                stdout
                    .lines()
                    .next()
                    .and_then(|line| line.trim().parse().ok())
            } else {
                None
            }
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    pub fn is_port_in_use(port: u16) -> bool {
        use std::net::TcpListener;
        // Bind both interfaces to catch processes on any address
        TcpListener::bind(("0.0.0.0", port)).is_err()
            || TcpListener::bind(("127.0.0.1", port)).is_err()
    }

    async fn kill_pid(&self, pid: u32, service_name: &str) {
        #[cfg(unix)]
        {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }

            let timeout = std::time::Duration::from_secs(10);
            let start = std::time::Instant::now();
            while Self::is_pid_running(pid) && start.elapsed() < timeout {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            if Self::is_pid_running(pid) {
                warn!(
                    "Process {} did not exit gracefully, sending SIGKILL",
                    service_name
                );
                unsafe {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
        }
    }

    pub async fn get_logs(
        &self,
        handle: &ProcessHandle,
        lines: Option<usize>,
    ) -> Result<Vec<String>> {
        let logs = handle.logs.read().await;

        match lines {
            Some(n) => {
                let start = logs.len().saturating_sub(n);
                Ok(logs[start..].to_vec())
            }
            None => Ok(logs.clone()),
        }
    }

    pub async fn wait(&self, handle: &mut ProcessHandle) -> Result<Option<i32>> {
        if let Some(child) = &mut handle.child {
            let status = child.wait().await?;
            Ok(status.code())
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pm(tmp: &tempfile::TempDir) -> ProcessManager {
        let runtime_db = Arc::new(RuntimeDb::open(tmp.path()).unwrap());
        ProcessManager::new(tmp.path().to_path_buf(), runtime_db)
    }

    #[tokio::test]
    async fn test_run_command() {
        let tmp = tempfile::tempdir().unwrap();
        let pm = make_pm(&tmp);
        let output = pm
            .run_command("echo hello", &tmp.path().to_path_buf(), &HashMap::new(), "sh")
            .await
            .unwrap();

        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
    }

    #[tokio::test]
    async fn test_spawn_and_stop() {
        let tmp = tempfile::tempdir().unwrap();
        let pm = make_pm(&tmp);
        let handle = pm
            .spawn("sleep 60", &tmp.path().to_path_buf(), HashMap::new(), "test", "sh")
            .await
            .unwrap();

        assert!(handle.pid.is_some());

        let pid = pm.runtime_db().read_pid("test");
        assert!(pid.is_some());

        pm.stop(handle, "test").await.unwrap();
    }
}
