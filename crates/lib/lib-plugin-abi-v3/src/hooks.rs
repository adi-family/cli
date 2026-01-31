//! Lifecycle Hooks
//!
//! Types and executor for lifecycle hooks (pre-up, post-up, pre-down, post-down).
//!
//! Hooks run one-shot tasks at specific points during service startup and shutdown.
//! Hook steps can use any runner plugin (script, docker, compose, etc.) and support
//! sequential and parallel execution.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// Hook Event Types
// ============================================================================

/// Lifecycle hook events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookEvent {
    /// Before service/stack starts
    PreUp,
    /// After service/stack is healthy
    PostUp,
    /// Before service/stack stops
    PreDown,
    /// After service/stack has stopped
    PostDown,
}

impl HookEvent {
    /// Default on_failure behavior for this event
    pub fn default_on_failure(&self) -> OnFailure {
        match self {
            HookEvent::PreUp => OnFailure::Abort,
            HookEvent::PostUp => OnFailure::Abort,
            HookEvent::PreDown => OnFailure::Warn,
            HookEvent::PostDown => OnFailure::Warn,
        }
    }

    /// Human-readable name
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::PreUp => "pre-up",
            HookEvent::PostUp => "post-up",
            HookEvent::PreDown => "pre-down",
            HookEvent::PostDown => "post-down",
        }
    }
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// Failure Behavior
// ============================================================================

/// What to do when a hook step fails
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OnFailure {
    /// Stop the operation (service doesn't start, deployment rolls back)
    Abort,
    /// Log a warning and continue
    Warn,
    /// Retry up to N times, then abort
    Retry,
}

// ============================================================================
// Hook Runner Configuration
// ============================================================================

/// Runner configuration for a hook step (explicit `runner:` block)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRunnerConfig {
    /// Runner plugin type (e.g., "script", "docker", "compose")
    #[serde(rename = "type")]
    pub runner_type: String,
    /// Plugin-specific configuration
    #[serde(flatten)]
    pub config: serde_json::Value,
}

// ============================================================================
// Hook Steps
// ============================================================================

/// A single step in a hook event.
///
/// Steps are mutually exclusive: exactly one of `run`, `runner`, or `parallel`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookStep {
    /// Script command (shorthand for script runner)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,

    /// Explicit runner plugin configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner: Option<HookRunnerConfig>,

    /// Parallel group of steps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel: Option<Vec<HookStep>>,

    /// Working directory (script steps only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    /// Failure behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<OnFailure>,

    /// Maximum execution time (e.g., "60s", "5m")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// Number of retries (only when on_failure = retry)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,

    /// Delay between retries (e.g., "5s")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_delay: Option<String>,

    /// Additional environment variables for this hook
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,
}

impl HookStep {
    /// Create a script step
    pub fn script(command: &str) -> Self {
        Self {
            run: Some(command.to_string()),
            runner: None,
            parallel: None,
            working_dir: None,
            on_failure: None,
            timeout: None,
            retries: None,
            retry_delay: None,
            environment: None,
        }
    }

    /// Create an explicit runner step
    pub fn with_runner(runner_type: &str, config: serde_json::Value) -> Self {
        Self {
            run: None,
            runner: Some(HookRunnerConfig {
                runner_type: runner_type.to_string(),
                config,
            }),
            parallel: None,
            working_dir: None,
            on_failure: None,
            timeout: None,
            retries: None,
            retry_delay: None,
            environment: None,
        }
    }

    /// Create a parallel group step
    pub fn parallel(steps: Vec<HookStep>) -> Self {
        Self {
            run: None,
            runner: None,
            parallel: Some(steps),
            working_dir: None,
            on_failure: None,
            timeout: None,
            retries: None,
            retry_delay: None,
            environment: None,
        }
    }

    /// Determine the step type
    pub fn step_type(&self) -> StepType {
        if self.run.is_some() {
            StepType::Script
        } else if self.runner.is_some() {
            StepType::Runner
        } else if self.parallel.is_some() {
            StepType::Parallel
        } else {
            StepType::Script // fallback
        }
    }

    /// Validate that exactly one of run/runner/parallel is set
    pub fn validate(&self) -> Result<()> {
        let count = [
            self.run.is_some(),
            self.runner.is_some(),
            self.parallel.is_some(),
        ]
        .iter()
        .filter(|&&b| b)
        .count();

        if count == 0 {
            return Err(crate::PluginError::Other(anyhow::anyhow!(
                "Hook step must have exactly one of: run, runner, or parallel"
            )));
        }
        if count > 1 {
            return Err(crate::PluginError::Other(anyhow::anyhow!(
                "Hook step must have exactly one of: run, runner, or parallel (found {})",
                count
            )));
        }

        // Validate parallel children recursively
        if let Some(ref steps) = self.parallel {
            for step in steps {
                step.validate()?;
            }
        }

        // Validate retry config
        if self.on_failure == Some(OnFailure::Retry) && self.parallel.is_some() {
            return Err(crate::PluginError::Other(anyhow::anyhow!(
                "on_failure: retry is not available for parallel groups"
            )));
        }

        Ok(())
    }

    /// Resolve the effective on_failure for this step
    pub fn effective_on_failure(&self, event: HookEvent) -> OnFailure {
        self.on_failure
            .unwrap_or_else(|| event.default_on_failure())
    }

    /// Parse timeout to Duration
    pub fn timeout_duration(&self) -> Duration {
        self.timeout
            .as_deref()
            .and_then(parse_duration)
            .unwrap_or(Duration::from_secs(60))
    }

    /// Parse retry delay to Duration
    pub fn retry_delay_duration(&self) -> Duration {
        self.retry_delay
            .as_deref()
            .and_then(parse_duration)
            .unwrap_or(Duration::from_secs(5))
    }

    /// Get retry count
    pub fn retry_count(&self) -> u32 {
        self.retries.unwrap_or(3)
    }
}

/// Step type discriminant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepType {
    Script,
    Runner,
    Parallel,
}

// ============================================================================
// Hook Configuration
// ============================================================================

/// Complete hooks configuration for a service or global scope
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HooksConfig {
    /// Steps to run before service/stack starts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_up: Vec<HookStep>,

    /// Steps to run after service/stack is healthy
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_up: Vec<HookStep>,

    /// Steps to run before service/stack stops
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_down: Vec<HookStep>,

    /// Steps to run after service/stack has stopped
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_down: Vec<HookStep>,
}

impl HooksConfig {
    /// Check if any hooks are configured
    pub fn is_empty(&self) -> bool {
        self.pre_up.is_empty()
            && self.post_up.is_empty()
            && self.pre_down.is_empty()
            && self.post_down.is_empty()
    }

    /// Get steps for a given event
    pub fn steps_for(&self, event: HookEvent) -> &[HookStep] {
        match event {
            HookEvent::PreUp => &self.pre_up,
            HookEvent::PostUp => &self.post_up,
            HookEvent::PreDown => &self.pre_down,
            HookEvent::PostDown => &self.post_down,
        }
    }

    /// Validate all hook steps
    pub fn validate(&self) -> Result<()> {
        for step in &self.pre_up {
            step.validate()?;
        }
        for step in &self.post_up {
            step.validate()?;
        }
        for step in &self.pre_down {
            step.validate()?;
        }
        for step in &self.post_down {
            step.validate()?;
        }
        Ok(())
    }
}

// ============================================================================
// Hook Execution Context
// ============================================================================

/// Context injected into hook environment variables
#[derive(Debug, Clone)]
pub struct HookContext {
    /// Current hook event
    pub event: HookEvent,
    /// Service name (None for global hooks)
    pub service_name: Option<String>,
    /// Fully qualified name (source:service)
    pub service_fqn: Option<String>,
    /// Source name
    pub source_name: String,
    /// Rollout type (e.g., "recreate", "blue-green")
    pub rollout_type: Option<String>,
    /// Active color for blue-green (e.g., "blue", "green")
    pub rollout_color: Option<String>,
}

impl HookContext {
    /// Build environment variables injected by Hive into hooks
    pub fn to_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert(
            "HIVE_HOOK_EVENT".to_string(),
            self.event.as_str().to_string(),
        );
        env.insert("HIVE_SOURCE_NAME".to_string(), self.source_name.clone());

        if let Some(ref name) = self.service_name {
            env.insert("HIVE_SERVICE_NAME".to_string(), name.clone());
        }
        if let Some(ref fqn) = self.service_fqn {
            env.insert("HIVE_SERVICE_FQN".to_string(), fqn.clone());
        }
        if let Some(ref rt) = self.rollout_type {
            env.insert("HIVE_ROLLOUT_TYPE".to_string(), rt.clone());
        }
        if let Some(ref color) = self.rollout_color {
            env.insert("HIVE_ROLLOUT_COLOR".to_string(), color.clone());
        }

        env
    }
}

// ============================================================================
// Hook Execution Result
// ============================================================================

/// Result of executing a hook event (all steps for one event)
#[derive(Debug, Clone)]
pub struct HookEventResult {
    /// The event that was executed
    pub event: HookEvent,
    /// Whether the event succeeded overall
    pub success: bool,
    /// Results of individual steps
    pub step_results: Vec<HookStepResult>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Result of a single hook step execution
#[derive(Debug, Clone)]
pub struct HookStepResult {
    /// Step index within the event
    pub index: usize,
    /// Step type that was executed
    pub step_type: StepType,
    /// Whether the step succeeded
    pub success: bool,
    /// Exit status from the runner
    pub exit_status: Option<HookExitStatus>,
    /// Error message if failed
    pub error: Option<String>,
    /// How long the step took
    pub duration: Duration,
    /// Number of retries attempted
    pub retries_attempted: u32,
}

/// Hook exit status
#[derive(Debug, Clone)]
pub struct HookExitStatus {
    /// Exit code
    pub code: i32,
    /// Standard output
    pub output: Option<String>,
    /// Standard error
    pub stderr: Option<String>,
}

impl HookExitStatus {
    /// Create a successful exit status
    pub fn success() -> Self {
        Self {
            code: 0,
            output: None,
            stderr: None,
        }
    }

    /// Create a failed exit status
    pub fn failed(code: i32) -> Self {
        Self {
            code,
            output: None,
            stderr: None,
        }
    }

    /// Check if the exit was successful
    pub fn is_success(&self) -> bool {
        self.code == 0
    }

    /// Add output to exit status
    pub fn with_output(mut self, output: String) -> Self {
        self.output = Some(output);
        self
    }

    /// Add stderr to exit status
    pub fn with_stderr(mut self, stderr: String) -> Self {
        self.stderr = Some(stderr);
        self
    }
}

// ============================================================================
// Utilities
// ============================================================================

/// Parse a duration string like "60s", "5m", "500ms"
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(ms_str) = s.strip_suffix("ms") {
        ms_str.parse::<u64>().ok().map(Duration::from_millis)
    } else if let Some(s_str) = s.strip_suffix('s') {
        s_str.parse::<u64>().ok().map(Duration::from_secs)
    } else if let Some(m_str) = s.strip_suffix('m') {
        m_str
            .parse::<u64>()
            .ok()
            .map(|m| Duration::from_secs(m * 60))
    } else {
        // Default: try as seconds
        s.parse::<u64>().ok().map(Duration::from_secs)
    }
}

// ============================================================================
// Hook Executor
// ============================================================================

use crate::runner::{Runner, RuntimeContext};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Executes lifecycle hook steps.
///
/// The executor runs steps sequentially (top-level) or concurrently (parallel groups),
/// respecting on_failure policies and timeouts.
pub struct HookExecutor {
    /// Available runner plugins by type name (e.g., "docker" -> DockerRunnerPlugin)
    runners: Arc<RwLock<HashMap<String, Arc<dyn Runner>>>>,
}

impl HookExecutor {
    /// Create a new hook executor with available runner plugins
    pub fn new(runners: HashMap<String, Arc<dyn Runner>>) -> Self {
        Self {
            runners: Arc::new(RwLock::new(runners)),
        }
    }

    /// Execute all steps for a hook event
    pub async fn execute(
        &self,
        event: HookEvent,
        steps: &[HookStep],
        base_env: &HashMap<String, String>,
        hook_ctx: &HookContext,
        runtime_ctx: &RuntimeContext,
    ) -> HookEventResult {
        if steps.is_empty() {
            return HookEventResult {
                event,
                success: true,
                step_results: vec![],
                error: None,
            };
        }

        info!("Running {} hooks ({} steps)", event, steps.len());

        let mut step_results = Vec::new();
        let mut overall_success = true;

        // Merge base env with hook context env
        let mut full_env = base_env.clone();
        full_env.extend(hook_ctx.to_env());

        for (index, step) in steps.iter().enumerate() {
            let result = self
                .execute_step(index, step, event, &full_env, runtime_ctx)
                .await;

            let step_success = result.success;
            step_results.push(result);

            if !step_success {
                let on_failure = step.effective_on_failure(event);
                match on_failure {
                    OnFailure::Abort => {
                        error!(
                            "{} hook step {} failed (on_failure: abort) - stopping",
                            event, index
                        );
                        overall_success = false;
                        break;
                    }
                    OnFailure::Warn => {
                        warn!(
                            "{} hook step {} failed (on_failure: warn) - continuing",
                            event, index
                        );
                    }
                    OnFailure::Retry => {
                        // Retry logic is handled inside execute_step
                        // If we get here, all retries failed -> abort
                        error!(
                            "{} hook step {} failed after retries (on_failure: retry) - stopping",
                            event, index
                        );
                        overall_success = false;
                        break;
                    }
                }
            }
        }

        let error = if overall_success {
            None
        } else {
            Some(format!("{} hooks failed", event))
        };

        info!(
            "{} hooks completed: {}",
            event,
            if overall_success { "success" } else { "FAILED" }
        );

        HookEventResult {
            event,
            success: overall_success,
            step_results,
            error,
        }
    }

    /// Execute a single step (with retry logic)
    async fn execute_step(
        &self,
        index: usize,
        step: &HookStep,
        event: HookEvent,
        env: &HashMap<String, String>,
        runtime_ctx: &RuntimeContext,
    ) -> HookStepResult {
        let start = std::time::Instant::now();
        let step_type = step.step_type();
        let on_failure = step.effective_on_failure(event);

        // Merge step-level environment
        let mut step_env = env.clone();
        if let Some(ref extra_env) = step.environment {
            step_env.extend(extra_env.clone());
        }

        // Apply retry logic for OnFailure::Retry
        let max_attempts = if on_failure == OnFailure::Retry {
            step.retry_count() + 1 // retries + initial attempt
        } else {
            1
        };
        let retry_delay = step.retry_delay_duration();

        let mut last_result = None;
        let mut retries_attempted = 0u32;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                retries_attempted += 1;
                debug!(
                    "{} hook step {}: retry {}/{}",
                    event,
                    index,
                    attempt,
                    step.retry_count()
                );
                tokio::time::sleep(retry_delay).await;
            }

            let timeout = step.timeout_duration();
            let result = tokio::time::timeout(
                timeout,
                self.execute_step_inner(step, &step_env, runtime_ctx),
            )
            .await;

            match result {
                Ok(Ok(exit_status)) if exit_status.is_success() => {
                    return HookStepResult {
                        index,
                        step_type,
                        success: true,
                        exit_status: Some(exit_status),
                        error: None,
                        duration: start.elapsed(),
                        retries_attempted,
                    };
                }
                Ok(Ok(exit_status)) => {
                    last_result = Some(HookStepResult {
                        index,
                        step_type,
                        success: false,
                        exit_status: Some(exit_status.clone()),
                        error: Some(format!("Hook exited with code {}", exit_status.code)),
                        duration: start.elapsed(),
                        retries_attempted,
                    });
                }
                Ok(Err(e)) => {
                    last_result = Some(HookStepResult {
                        index,
                        step_type,
                        success: false,
                        exit_status: None,
                        error: Some(format!("Hook execution error: {}", e)),
                        duration: start.elapsed(),
                        retries_attempted,
                    });
                }
                Err(_) => {
                    last_result = Some(HookStepResult {
                        index,
                        step_type,
                        success: false,
                        exit_status: None,
                        error: Some(format!(
                            "Hook timed out after {}s",
                            timeout.as_secs()
                        )),
                        duration: start.elapsed(),
                        retries_attempted,
                    });
                }
            }
        }

        last_result.unwrap_or(HookStepResult {
            index,
            step_type,
            success: false,
            exit_status: None,
            error: Some("No execution result".to_string()),
            duration: start.elapsed(),
            retries_attempted,
        })
    }

    /// Execute the inner step logic (no retry/timeout wrapper)
    fn execute_step_inner<'a>(
        &'a self,
        step: &'a HookStep,
        env: &'a HashMap<String, String>,
        runtime_ctx: &'a RuntimeContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<HookExitStatus>> + Send + 'a>>
    {
        Box::pin(async move {
            match step.step_type() {
                StepType::Script => self.execute_script(step, env, runtime_ctx).await,
                StepType::Runner => self.execute_runner(step, env, runtime_ctx).await,
                StepType::Parallel => self.execute_parallel(step, env, runtime_ctx).await,
            }
        })
    }

    /// Execute a script step (built-in)
    async fn execute_script(
        &self,
        step: &HookStep,
        env: &HashMap<String, String>,
        runtime_ctx: &RuntimeContext,
    ) -> crate::Result<HookExitStatus> {
        let command = step
            .run
            .as_ref()
            .ok_or_else(|| crate::PluginError::Other(anyhow::anyhow!("Script step missing 'run' field")))?;

        // Interpolate runtime templates in command
        let interpolated = runtime_ctx.interpolate(command)?;

        // Determine working directory
        let working_dir = if let Some(ref dir) = step.working_dir {
            let path = std::path::Path::new(dir);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                runtime_ctx.working_dir.join(path)
            }
        } else {
            runtime_ctx.working_dir.clone()
        };

        debug!(
            "Executing hook script in {:?}: {}",
            working_dir,
            interpolated.lines().next().unwrap_or(&interpolated)
        );

        // Execute via shell
        let shell = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let output = tokio::process::Command::new(shell.0)
            .arg(shell.1)
            .arg(&interpolated)
            .current_dir(&working_dir)
            .envs(env)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| crate::PluginError::Other(anyhow::anyhow!("Failed to execute hook script: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Log output
        if !stdout.is_empty() {
            for line in stdout.lines() {
                info!("[hook] {}", line);
            }
        }
        if !stderr.is_empty() {
            for line in stderr.lines() {
                warn!("[hook stderr] {}", line);
            }
        }

        let code = output.status.code().unwrap_or(-1);
        Ok(HookExitStatus { code, output: Some(stdout), stderr: Some(stderr) })
    }

    /// Execute a runner step (docker, compose, etc.)
    async fn execute_runner(
        &self,
        step: &HookStep,
        env: &HashMap<String, String>,
        runtime_ctx: &RuntimeContext,
    ) -> crate::Result<HookExitStatus> {
        let runner_config = step
            .runner
            .as_ref()
            .ok_or_else(|| crate::PluginError::Other(anyhow::anyhow!("Runner step missing 'runner' field")))?;

        let runner_type = &runner_config.runner_type;

        // Look up the runner plugin
        let runners = self.runners.read().await;
        let runner = runners
            .get(runner_type)
            .ok_or_else(|| {
                crate::PluginError::Other(anyhow::anyhow!(
                    "Runner plugin '{}' not found. Available runners: {:?}",
                    runner_type,
                    runners.keys().collect::<Vec<_>>()
                ))
            })?
            .clone();

        if !runner.supports_hooks() {
            return Err(crate::PluginError::Other(anyhow::anyhow!(
                "Runner plugin '{}' does not support hook execution",
                runner_type
            )));
        }

        debug!("Executing hook via runner plugin: {}", runner_type);

        runner
            .run_hook(&runner_config.config, env.clone(), runtime_ctx)
            .await
    }

    /// Execute a parallel group of steps
    async fn execute_parallel(
        &self,
        step: &HookStep,
        env: &HashMap<String, String>,
        runtime_ctx: &RuntimeContext,
    ) -> crate::Result<HookExitStatus> {
        let steps = step
            .parallel
            .as_ref()
            .ok_or_else(|| crate::PluginError::Other(anyhow::anyhow!("Parallel step missing 'parallel' field")))?;

        if steps.is_empty() {
            return Ok(HookExitStatus::success());
        }

        debug!("Executing {} parallel hook steps", steps.len());

        // Spawn all steps concurrently
        let mut handles = Vec::new();
        for (i, child_step) in steps.iter().enumerate() {
            let step_clone = child_step.clone();
            let env_clone = env.clone();
            let ctx_clone = runtime_ctx.clone();
            let executor_runners = self.runners.clone();

            let handle = tokio::spawn(async move {
                // Create a temporary executor sharing the same runners
                let temp_executor = HookExecutor {
                    runners: executor_runners,
                };
                let result = temp_executor
                    .execute_step_inner(&step_clone, &env_clone, &ctx_clone)
                    .await;

                (i, result)
            });

            handles.push(handle);
        }

        // Collect results
        let mut all_success = true;
        let mut errors = Vec::new();

        for handle in handles {
            match handle.await {
                Ok((i, Ok(exit_status))) => {
                    if !exit_status.is_success() {
                        all_success = false;
                        errors.push(format!(
                            "parallel step {} exited with code {}",
                            i, exit_status.code
                        ));
                    }
                }
                Ok((i, Err(e))) => {
                    all_success = false;
                    errors.push(format!("parallel step {} error: {}", i, e));
                }
                Err(e) => {
                    all_success = false;
                    errors.push(format!("parallel step panicked: {}", e));
                }
            }
        }

        if all_success {
            Ok(HookExitStatus::success())
        } else {
            let combined_error = errors.join("; ");
            Ok(HookExitStatus::failed(1).with_stderr(combined_error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_display() {
        assert_eq!(HookEvent::PreUp.as_str(), "pre-up");
        assert_eq!(HookEvent::PostUp.as_str(), "post-up");
        assert_eq!(HookEvent::PreDown.as_str(), "pre-down");
        assert_eq!(HookEvent::PostDown.as_str(), "post-down");
    }

    #[test]
    fn test_default_on_failure() {
        assert_eq!(HookEvent::PreUp.default_on_failure(), OnFailure::Abort);
        assert_eq!(HookEvent::PostUp.default_on_failure(), OnFailure::Abort);
        assert_eq!(HookEvent::PreDown.default_on_failure(), OnFailure::Warn);
        assert_eq!(HookEvent::PostDown.default_on_failure(), OnFailure::Warn);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("60s"), Some(Duration::from_secs(60)));
        assert_eq!(parse_duration("5m"), Some(Duration::from_secs(300)));
        assert_eq!(parse_duration("500ms"), Some(Duration::from_millis(500)));
        assert_eq!(parse_duration("30"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration("abc"), None);
    }

    #[test]
    fn test_hook_exit_status() {
        let success = HookExitStatus::success();
        assert!(success.is_success());
        assert_eq!(success.code, 0);

        let failed = HookExitStatus::failed(1);
        assert!(!failed.is_success());
        assert_eq!(failed.code, 1);
    }
}
