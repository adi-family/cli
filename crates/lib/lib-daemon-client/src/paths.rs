//! Default daemon socket/PID/log paths

use std::path::PathBuf;

use lib_env_parse::{env_opt, env_vars};

env_vars! {
    AdiDaemonSocket    => "ADI_DAEMON_SOCKET",
    AdiDaemonPid       => "ADI_DAEMON_PID",
    AdiDaemonLog       => "ADI_DAEMON_LOG",
    AdiDaemonTcpPort   => "ADI_DAEMON_TCP_PORT",
}

const ADI_SUBDIR: &str = "adi";
const DEFAULT_DAEMON_TCP_PORT: u16 = 14731;

/// ADI data directory (~/.local/share/adi)
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join(ADI_SUBDIR)
}

/// Daemon socket path ($ADI_DAEMON_SOCKET or ~/.local/share/adi/daemon.sock)
pub fn daemon_socket_path() -> PathBuf {
    env_opt(EnvVar::AdiDaemonSocket.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir().join("daemon.sock"))
}

/// Daemon PID file path ($ADI_DAEMON_PID or ~/.local/share/adi/daemon.pid)
pub fn daemon_pid_path() -> PathBuf {
    env_opt(EnvVar::AdiDaemonPid.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir().join("daemon.pid"))
}

/// Daemon log file path ($ADI_DAEMON_LOG or ~/.local/share/adi/logs/daemon.log)
pub fn daemon_log_path() -> PathBuf {
    env_opt(EnvVar::AdiDaemonLog.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir().join("logs").join("daemon.log"))
}

/// Daemon TCP port for non-Unix platforms
pub fn daemon_tcp_port() -> u16 {
    env_opt(EnvVar::AdiDaemonTcpPort.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_DAEMON_TCP_PORT)
}
