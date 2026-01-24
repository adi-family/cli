//! Configuration for the WebRTC test peer
//!
//! Supports CLI args, environment variables, and programmatic configuration.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// WebRTC Test Peer - Minimal cocoon simulator for E2E testing
#[derive(Parser, Debug, Clone)]
#[command(name = "webrtc-test-peer")]
#[command(about = "A minimal WebRTC peer for testing web-app connections")]
pub struct CliArgs {
    /// Signaling server WebSocket URL
    #[arg(short, long, default_value = "ws://localhost:8080/ws")]
    pub signaling_url: String,

    /// Device ID to register with (auto-generated if not provided)
    #[arg(short, long)]
    pub device_id: Option<String>,

    /// Secret for device registration (auto-generated if not provided)
    #[arg(long)]
    pub secret: Option<String>,

    /// Setup token for authenticated registration (optional)
    #[arg(long)]
    pub setup_token: Option<String>,

    /// Display name for the test peer
    #[arg(long, default_value = "test-cocoon")]
    pub name: String,

    /// Bypass authentication (for local testing without auth)
    #[arg(long, default_value = "false")]
    pub bypass_auth: bool,

    /// Directory to serve as mock filesystem root
    #[arg(long)]
    pub mock_fs_root: Option<PathBuf>,

    /// Path to test scenario script (JSON)
    #[arg(long)]
    pub script: Option<PathBuf>,

    /// ICE candidate drop rate (0.0 - 1.0) for testing failures
    #[arg(long, default_value = "0.0")]
    pub drop_ice_rate: f64,

    /// Data channel message drop rate (0.0 - 1.0)
    #[arg(long, default_value = "0.0")]
    pub drop_data_rate: f64,

    /// Artificial latency to add to responses (milliseconds)
    #[arg(long, default_value = "0")]
    pub latency_ms: u64,

    /// Don't send WebRTC answer (for testing connection timeouts)
    #[arg(long, default_value = "false")]
    pub no_answer: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Exit after first WebRTC session completes
    #[arg(long, default_value = "false")]
    pub one_shot: bool,

    /// Timeout for waiting for connections (seconds, 0 = no timeout)
    #[arg(long, default_value = "0")]
    pub timeout_secs: u64,
}

/// Programmatic configuration (superset of CLI args)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Signaling server WebSocket URL
    pub signaling_url: String,

    /// Device ID to register with
    pub device_id: String,

    /// Secret for device registration
    pub secret: String,

    /// Setup token for authenticated registration
    pub setup_token: Option<String>,

    /// Display name
    pub name: String,

    /// Bypass authentication
    pub bypass_auth: bool,

    /// Mock filesystem root
    pub mock_fs_root: Option<PathBuf>,

    /// Test scenario configuration
    pub scenario: TestScenario,

    /// Exit after first session
    pub one_shot: bool,

    /// Connection timeout
    pub timeout: Option<Duration>,
}

/// Test scenario configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestScenario {
    /// ICE candidate drop rate (0.0 - 1.0)
    #[serde(default)]
    pub drop_ice_rate: f64,

    /// Data channel message drop rate (0.0 - 1.0)
    #[serde(default)]
    pub drop_data_rate: f64,

    /// Artificial latency for responses
    #[serde(default)]
    pub latency: Duration,

    /// Don't send WebRTC answer
    #[serde(default)]
    pub no_answer: bool,

    /// PTY behavior configuration
    #[serde(default)]
    pub pty: PtyScenario,

    /// Silk behavior configuration
    #[serde(default)]
    pub silk: SilkScenario,

    /// Filesystem behavior configuration
    #[serde(default)]
    pub filesystem: FilesystemScenario,
}

/// PTY mock behavior
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PtyScenario {
    /// Echo back input as output
    #[serde(default = "default_true")]
    pub echo: bool,

    /// Prefix to add to echoed output
    #[serde(default)]
    pub echo_prefix: Option<String>,

    /// Fixed responses for specific commands
    #[serde(default)]
    pub responses: Vec<CommandResponse>,

    /// Simulate slow output (chars per second, 0 = instant)
    #[serde(default)]
    pub output_rate: u32,
}

/// Silk mock behavior
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SilkScenario {
    /// Default working directory
    #[serde(default = "default_cwd")]
    pub default_cwd: String,

    /// Default shell
    #[serde(default = "default_shell")]
    pub default_shell: String,

    /// Fixed responses for specific commands
    #[serde(default)]
    pub responses: Vec<CommandResponse>,

    /// Fail session creation
    #[serde(default)]
    pub fail_session: bool,

    /// Fail specific command IDs
    #[serde(default)]
    pub fail_commands: Vec<String>,
}

/// Filesystem mock behavior
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesystemScenario {
    /// Virtual filesystem structure
    #[serde(default)]
    pub virtual_fs: Vec<VirtualFsEntry>,

    /// Fail reads for specific paths
    #[serde(default)]
    pub fail_paths: Vec<String>,

    /// Simulate slow reads (bytes per second, 0 = instant)
    #[serde(default)]
    pub read_rate: u32,
}

/// Command/response mapping for scripted behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    /// Command pattern (exact match or regex if prefixed with ~)
    pub pattern: String,

    /// Output to return
    pub output: String,

    /// Exit code
    #[serde(default)]
    pub exit_code: i32,

    /// Delay before responding (milliseconds)
    #[serde(default)]
    pub delay_ms: u64,
}

/// Virtual filesystem entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualFsEntry {
    /// Path
    pub path: String,

    /// Is directory
    #[serde(default)]
    pub is_dir: bool,

    /// File content (for files)
    pub content: Option<String>,

    /// File size (auto-calculated from content if not set)
    pub size: Option<u64>,

    /// Children (for directories)
    #[serde(default)]
    pub children: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_cwd() -> String {
    "/home/test".to_string()
}

fn default_shell() -> String {
    "/bin/bash".to_string()
}

impl Config {
    /// Create config from CLI args
    pub fn from_cli(args: CliArgs) -> Self {
        let device_id = args.device_id.unwrap_or_else(|| {
            format!(
                "test-peer-{}",
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
            )
        });

        let secret = args.secret.unwrap_or_else(|| {
            // Generate a cryptographically random 64+ char secret
            // Avoid weak patterns like "secret", "password", "test", etc.
            format!(
                "e2epeer{}{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            )
        });

        let scenario = if let Some(script_path) = &args.script {
            // Load from file
            match std::fs::read_to_string(script_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(e) => {
                    tracing::warn!("Failed to load script {}: {}", script_path.display(), e);
                    TestScenario::default()
                }
            }
        } else {
            TestScenario {
                drop_ice_rate: args.drop_ice_rate,
                drop_data_rate: args.drop_data_rate,
                latency: Duration::from_millis(args.latency_ms),
                no_answer: args.no_answer,
                ..Default::default()
            }
        };

        let timeout = if args.timeout_secs > 0 {
            Some(Duration::from_secs(args.timeout_secs))
        } else {
            None
        };

        Self {
            signaling_url: args.signaling_url,
            device_id,
            secret,
            setup_token: args.setup_token,
            name: args.name,
            bypass_auth: args.bypass_auth,
            mock_fs_root: args.mock_fs_root,
            scenario,
            one_shot: args.one_shot,
            timeout,
        }
    }

    /// Create a minimal config for testing
    pub fn test_default() -> Self {
        Self {
            signaling_url: "ws://localhost:8080/ws".to_string(),
            device_id: format!("e2edev{}", uuid::Uuid::new_v4().simple()),
            secret: format!(
                "e2epeer{}{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ),
            setup_token: None,
            name: "test-cocoon".to_string(),
            bypass_auth: true,
            mock_fs_root: None,
            scenario: TestScenario::default(),
            one_shot: false,
            timeout: None,
        }
    }
}

impl TestScenario {
    /// Check if we should drop this ICE candidate
    pub fn should_drop_ice(&self) -> bool {
        if self.drop_ice_rate <= 0.0 {
            return false;
        }
        rand::random::<f64>() < self.drop_ice_rate
    }

    /// Check if we should drop this data message
    pub fn should_drop_data(&self) -> bool {
        if self.drop_data_rate <= 0.0 {
            return false;
        }
        rand::random::<f64>() < self.drop_data_rate
    }

    /// Get delay duration if configured
    pub fn get_latency(&self) -> Option<Duration> {
        if self.latency.is_zero() {
            None
        } else {
            Some(self.latency)
        }
    }
}
