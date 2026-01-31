//! Daemon lifecycle management and graceful shutdown

use tokio::sync::mpsc;
use tracing::{debug, info};

/// Coordinator for graceful daemon shutdown
///
/// Provides mechanisms to:
/// - Trigger shutdown from multiple sources
/// - Wait for shutdown signal
/// - Notify multiple subsystems of shutdown
///
/// # Example
///
/// ```no_run
/// use lib_daemon_core::ShutdownCoordinator;
///
/// #[tokio::main]
/// async fn main() {
///     let mut coordinator = ShutdownCoordinator::new();
///     let handle = coordinator.handle();
///
///     // Spawn background task that can trigger shutdown
///     tokio::spawn(async move {
///         // ... some work ...
///         handle.shutdown(); // Trigger shutdown
///     });
///
///     // Main loop
///     coordinator.wait().await;
///     println!("Shutdown signal received");
/// }
/// ```
#[derive(Debug)]
pub struct ShutdownCoordinator {
    tx: mpsc::Sender<()>,
    rx: mpsc::Receiver<()>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);
        Self { tx, rx }
    }

    /// Get a handle that can trigger shutdown
    ///
    /// This handle can be cloned and shared across tasks.
    pub fn handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            tx: self.tx.clone(),
        }
    }

    /// Wait for shutdown signal
    ///
    /// Blocks until shutdown is triggered via a handle.
    pub async fn wait(&mut self) {
        let _ = self.rx.recv().await;
        info!("Shutdown signal received");
    }

    /// Try to receive shutdown signal without blocking
    ///
    /// Returns `true` if shutdown was signaled, `false` otherwise.
    pub fn try_recv(&mut self) -> bool {
        self.rx.try_recv().is_ok()
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for triggering daemon shutdown
///
/// Can be cloned and shared across tasks. Any handle can trigger shutdown.
#[derive(Debug, Clone)]
pub struct ShutdownHandle {
    tx: mpsc::Sender<()>,
}

impl ShutdownHandle {
    /// Trigger shutdown
    ///
    /// This is non-blocking and will return immediately.
    /// Multiple calls are safe and idempotent.
    pub fn shutdown(&self) {
        debug!("Shutdown triggered");
        // Try to send, but don't block if channel is full
        let _ = self.tx.try_send(());
    }

    /// Trigger shutdown asynchronously
    ///
    /// This will wait until the shutdown signal can be sent.
    pub async fn shutdown_async(&self) {
        debug!("Shutdown triggered (async)");
        let _ = self.tx.send(()).await;
    }
}

/// Helper for running a daemon with graceful shutdown
///
/// This function sets up:
/// - Ctrl+C signal handler (Unix: SIGINT, Windows: Ctrl+C)
/// - SIGTERM handler (Unix only)
/// - Custom shutdown logic
///
/// # Example
///
/// ```no_run
/// use lib_daemon_core::lifecycle::run_with_shutdown;
///
/// #[tokio::main]
/// async fn main() {
///     run_with_shutdown(async {
///         // Your daemon logic here
///         loop {
///             tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
///         }
///     }).await.unwrap();
/// }
/// ```
pub async fn run_with_shutdown<F>(daemon_task: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let mut coordinator = ShutdownCoordinator::new();

    // Setup signal handlers
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;

        let handle = coordinator.handle();
        tokio::spawn(async move {
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM");
                    handle.shutdown();
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT");
                    handle.shutdown();
                }
            }
        });
    }

    #[cfg(not(unix))]
    {
        let handle = coordinator.handle();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            info!("Received Ctrl+C");
            handle.shutdown();
        });
    }

    // Run daemon task
    tokio::spawn(daemon_task);

    // Wait for shutdown
    coordinator.wait().await;

    info!("Graceful shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_shutdown_coordinator() {
        let mut coordinator = ShutdownCoordinator::new();
        let handle = coordinator.handle();

        // Spawn task that triggers shutdown after delay
        tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            handle.shutdown();
        });

        // Wait for shutdown (should complete quickly)
        coordinator.wait().await;
        // Test passes if we reach here
    }

    #[tokio::test]
    async fn test_shutdown_handle_clone() {
        let mut coordinator = ShutdownCoordinator::new();
        let handle1 = coordinator.handle();
        let handle2 = handle1.clone();

        // Both handles should be able to trigger shutdown
        tokio::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            handle2.shutdown();
        });

        coordinator.wait().await;
    }

    #[tokio::test]
    async fn test_try_recv() {
        let mut coordinator = ShutdownCoordinator::new();

        // Should return false when no shutdown
        assert!(!coordinator.try_recv());

        // Trigger shutdown
        coordinator.handle().shutdown();

        // Should return true after shutdown
        assert!(coordinator.try_recv());
    }
}
