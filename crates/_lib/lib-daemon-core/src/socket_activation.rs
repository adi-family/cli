//! Socket activation support for launchd (macOS) and systemd (Linux).
//!
//! When a service is launched via socket activation, the init system pre-binds
//! sockets and passes them as file descriptors to the process. This module
//! provides a cross-platform API to receive those listeners.
//!
//! ## Parent-to-child fd passing
//!
//! On macOS, `launch_activate_socket` can only be called by the direct launchd
//! job process, not child processes it spawns.  Use
//! `prepare_activated_fds_for_children` in the parent to claim the fds and
//! expose them via the `ADI_ACTIVATED_LISTEN_FDS` environment variable.
//! `receive_activated_listeners` checks that variable first so child processes
//! transparently pick up the inherited sockets.

use std::net::TcpListener;
use tracing::{debug, info, warn};

/// Environment variable used to pass socket activation fd numbers from a
/// parent daemon process to its child service processes.
const ADI_ACTIVATED_LISTEN_FDS: &str = "ADI_ACTIVATED_LISTEN_FDS";

/// A group of activated listeners sharing a name.
pub struct ActivatedListeners {
    /// Socket name (e.g. "ProxyListeners").
    pub name: String,
    /// Pre-bound TCP listeners inherited from the init system.
    pub listeners: Vec<TcpListener>,
}

/// Receive socket-activated listeners from the OS init system.
///
/// Checks for fds inherited from a parent daemon process (via the
/// `ADI_ACTIVATED_LISTEN_FDS` env var) first, then falls back to direct OS
/// socket activation (launchd / systemd).
///
/// Returns an empty vec when the process was *not* socket-activated
/// (e.g. launched manually in dev mode).
pub fn receive_activated_listeners() -> Vec<ActivatedListeners> {
    // Child processes receive fds via env var set by the parent daemon.
    let inherited = receive_inherited_listeners();
    if !inherited.is_empty() {
        return inherited;
    }

    #[cfg(target_os = "macos")]
    {
        receive_launchd()
    }
    #[cfg(target_os = "linux")]
    {
        receive_systemd()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Vec::new()
    }
}

/// Claim socket-activated fds from launchd (macOS) and prepare them for
/// inheritance by child processes.
///
/// Call this in the **direct launchd job process** (e.g. `adi daemon run`)
/// before spawning any child service processes.  The function:
/// 1. Claims the pre-bound fds via `launch_activate_socket`.
/// 2. Clears `FD_CLOEXEC` so they survive `execve` in child processes.
/// 3. Sets `ADI_ACTIVATED_LISTEN_FDS` to the comma-separated fd numbers so
///    child processes can find them via `receive_activated_listeners`.
///
/// The returned raw fd numbers must **not** be closed while child processes are
/// running — keep them alive for the lifetime of the parent daemon.
///
/// Returns an empty vec when not running under launchd socket activation or on
/// non-macOS platforms.
pub fn prepare_activated_fds_for_children() -> Vec<i32> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::io::IntoRawFd;

        let activated = receive_launchd();
        if activated.is_empty() {
            return Vec::new();
        }

        let mut raw_fds: Vec<i32> = Vec::new();
        for group in activated {
            for listener in group.listeners {
                let fd = listener.into_raw_fd();
                // Clear FD_CLOEXEC so the fd is inherited across execve.
                unsafe {
                    libc::fcntl(fd, libc::F_SETFD, 0);
                }
                raw_fds.push(fd);
            }
        }

        if !raw_fds.is_empty() {
            let fds_str = raw_fds
                .iter()
                .map(|fd| fd.to_string())
                .collect::<Vec<_>>()
                .join(",");
            std::env::set_var(ADI_ACTIVATED_LISTEN_FDS, &fds_str);
            info!(
                "launchd: claimed {} socket fd(s) for child processes (fds: {})",
                raw_fds.len(),
                fds_str
            );
        }

        raw_fds
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

/// Receive socket activation fds inherited from a parent daemon process.
///
/// Reads `ADI_ACTIVATED_LISTEN_FDS`, converts each fd number to a
/// `TcpListener`, and clears the env var so grandchild processes do not
/// attempt to re-use the same fds.
fn receive_inherited_listeners() -> Vec<ActivatedListeners> {
    let fds_str = match std::env::var(ADI_ACTIVATED_LISTEN_FDS) {
        Ok(s) if !s.is_empty() => s,
        _ => return Vec::new(),
    };

    let fds: Vec<i32> = fds_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if fds.is_empty() {
        return Vec::new();
    }

    #[cfg(unix)]
    {
        use std::os::unix::io::FromRawFd;

        info!(
            "Received {} inherited socket activation fd(s) from parent daemon",
            fds.len()
        );

        let listeners: Vec<TcpListener> = fds
            .into_iter()
            .filter_map(|fd| {
                // Safety: the parent daemon guarantees these are valid, bound TCP sockets.
                let listener = unsafe { TcpListener::from_raw_fd(fd) };
                if let Err(e) = listener.set_nonblocking(true) {
                    warn!("Failed to set nonblocking on inherited fd {}: {}", fd, e);
                }
                Some(listener)
            })
            .collect();

        // Remove the env var so grandchild processes don't try to claim the fds again.
        std::env::remove_var(ADI_ACTIVATED_LISTEN_FDS);

        vec![ActivatedListeners {
            name: "inherited".to_string(),
            listeners,
        }]
    }
    #[cfg(not(unix))]
    {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// macOS: launchd socket activation via launch_activate_socket()
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn receive_launchd() -> Vec<ActivatedListeners> {
    use std::os::unix::io::FromRawFd;

    // Try well-known socket names that we register in the plist.
    let names = ["ProxyListeners"];
    let mut result = Vec::new();

    for name in &names {
        match activate_launchd_socket(name) {
            Ok(fds) if !fds.is_empty() => {
                info!(
                    "launchd: received {} fd(s) for socket '{}'",
                    fds.len(),
                    name
                );
                let listeners: Vec<TcpListener> = fds
                    .into_iter()
                    .filter_map(|fd| {
                        // Safety: launchd guarantees the fd is a valid, bound socket.
                        let listener = unsafe { TcpListener::from_raw_fd(fd) };
                        if let Err(e) = listener.set_nonblocking(true) {
                            warn!("Failed to set nonblocking on fd {}: {}", fd, e);
                        }
                        Some(listener)
                    })
                    .collect();
                result.push(ActivatedListeners {
                    name: name.to_string(),
                    listeners,
                });
            }
            Ok(_) => {
                debug!("launchd: no fds for socket '{}' (not socket-activated)", name);
            }
            Err(e) => {
                debug!("launchd: launch_activate_socket('{}') failed: {} (not socket-activated)", name, e);
            }
        }
    }

    result
}

/// Call `launch_activate_socket()` and return the file descriptors.
#[cfg(target_os = "macos")]
fn activate_launchd_socket(name: &str) -> Result<Vec<i32>, String> {
    use std::ffi::CString;
    use std::ptr;

    extern "C" {
        fn launch_activate_socket(
            name: *const libc::c_char,
            fds: *mut *mut libc::c_int,
            cnt: *mut libc::size_t,
        ) -> libc::c_int;
    }

    let c_name = CString::new(name).map_err(|e| e.to_string())?;
    let mut fds: *mut libc::c_int = ptr::null_mut();
    let mut cnt: libc::size_t = 0;

    let rc = unsafe { launch_activate_socket(c_name.as_ptr(), &mut fds, &mut cnt) };

    if rc != 0 {
        return Err(format!("launch_activate_socket returned {}", rc));
    }

    let result: Vec<i32> = if cnt > 0 && !fds.is_null() {
        (0..cnt).map(|i| unsafe { *fds.add(i) }).collect()
    } else {
        Vec::new()
    };

    // Free the array allocated by launch_activate_socket.
    if !fds.is_null() {
        unsafe { libc::free(fds as *mut libc::c_void) };
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Linux: systemd socket activation via LISTEN_FDS / LISTEN_PID
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn receive_systemd() -> Vec<ActivatedListeners> {
    use std::os::unix::io::FromRawFd;

    const SD_LISTEN_FDS_START: i32 = 3;

    let listen_pid: u32 = match std::env::var("LISTEN_PID")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        Some(pid) => pid,
        None => return Vec::new(),
    };

    if listen_pid != std::process::id() {
        debug!(
            "systemd: LISTEN_PID={} but our PID={}, ignoring",
            listen_pid,
            std::process::id()
        );
        return Vec::new();
    }

    let listen_fds: usize = match std::env::var("LISTEN_FDS")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        Some(n) => n,
        None => return Vec::new(),
    };

    if listen_fds == 0 {
        return Vec::new();
    }

    info!("systemd: received {} socket-activated fd(s)", listen_fds);

    let listeners: Vec<TcpListener> = (0..listen_fds)
        .filter_map(|i| {
            let fd = SD_LISTEN_FDS_START + i as i32;
            // Safety: systemd guarantees fds starting at 3 are valid, bound sockets.
            let listener = unsafe { TcpListener::from_raw_fd(fd) };
            if let Err(e) = listener.set_nonblocking(true) {
                warn!("Failed to set nonblocking on fd {}: {}", fd, e);
            }
            Some(listener)
        })
        .collect();

    // Unset the env vars so child processes don't try to claim them.
    std::env::remove_var("LISTEN_PID");
    std::env::remove_var("LISTEN_FDS");

    vec![ActivatedListeners {
        name: "systemd".to_string(),
        listeners,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receive_returns_empty_when_not_activated() {
        // When running tests normally, there's no socket activation context.
        let listeners = receive_activated_listeners();
        assert!(listeners.is_empty());
    }
}
