//! Unix socket server and client utilities

use crate::error::{DaemonError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info};

/// Unix socket server for daemon IPC
pub struct UnixSocketServer {
    listener: UnixListener,
    socket_path: PathBuf,
}

impl UnixSocketServer {
    /// Bind to a Unix socket path
    ///
    /// Creates parent directories if needed and sets socket permissions to 0600 (user only).
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the socket file
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Cannot create parent directory
    /// - Cannot bind socket
    /// - Cannot set permissions
    pub async fn bind<P: AsRef<Path>>(path: P) -> Result<Self> {
        let socket_path = path.as_ref().to_path_buf();

        // Remove existing socket file
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        // Ensure directory exists
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Bind socket
        let listener = UnixListener::bind(&socket_path).map_err(|e| {
            DaemonError::SocketError(format!("Failed to bind socket: {}", e))
        })?;

        // Set permissions (user only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&socket_path, perms)?;
        }

        info!("Unix socket bound: {}", socket_path.display());

        Ok(Self {
            listener,
            socket_path,
        })
    }

    /// Accept a connection and return a stream
    pub async fn accept(&self) -> Result<UnixStream> {
        let (stream, _) = self.listener.accept().await.map_err(|e| {
            DaemonError::SocketError(format!("Failed to accept connection: {}", e))
        })?;
        debug!("Accepted connection");
        Ok(stream)
    }

    /// Get the socket path
    pub fn path(&self) -> &Path {
        &self.socket_path
    }

    /// Close the server and remove the socket file
    pub fn close(self) -> Result<()> {
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
            debug!("Socket removed: {}", self.socket_path.display());
        }
        Ok(())
    }
}

impl Drop for UnixSocketServer {
    fn drop(&mut self) {
        // Clean up socket file on drop
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}

/// Unix socket client for daemon IPC
pub struct UnixSocketClient {
    socket_path: PathBuf,
}

impl UnixSocketClient {
    /// Create a new client for the given socket path
    pub fn new<P: Into<PathBuf>>(socket_path: P) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    /// Check if daemon is available (socket exists)
    pub fn is_available(&self) -> bool {
        self.socket_path.exists()
    }

    /// Send a request and receive a response
    ///
    /// # Type Parameters
    ///
    /// * `Req` - Request type (must implement Serialize)
    /// * `Resp` - Response type (must implement Deserialize)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Cannot connect to socket
    /// - Cannot serialize request
    /// - Cannot deserialize response
    /// - Connection is closed
    pub async fn send<Req, Resp>(&self, request: &Req) -> Result<Resp>
    where
        Req: Serialize,
        Resp: for<'de> Deserialize<'de>,
    {
        if !self.is_available() {
            return Err(DaemonError::NotRunning);
        }

        // Connect to socket
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| {
                DaemonError::SocketError(format!("Failed to connect: {}", e))
            })?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Send request
        let request_json = serde_json::to_string(request)?;
        writer.write_all(request_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;

        // Read response
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        if response_line.is_empty() {
            return Err(DaemonError::ProtocolError(
                "Connection closed".to_string(),
            ));
        }

        // Deserialize response
        let response: Resp = serde_json::from_str(response_line.trim())?;
        Ok(response)
    }

    /// Get the socket path
    pub fn path(&self) -> &Path {
        &self.socket_path
    }
}

/// Connection handler that processes requests line-by-line
///
/// This is a helper for implementing request/response handlers over Unix sockets.
///
/// # Example
///
/// ```no_run
/// use lib_daemon_core::socket::ConnectionHandler;
/// use tokio::net::UnixStream;
///
/// async fn handle_connection(stream: UnixStream) {
///     let handler = ConnectionHandler::new(stream);
///
///     handler.process_requests(|request_json| async move {
///         // Parse request, process it, return response
///         Ok(format!(r#"{{"type":"ok"}}"#))
///     }).await.ok();
/// }
/// ```
pub struct ConnectionHandler {
    stream: UnixStream,
}

impl ConnectionHandler {
    /// Create a new connection handler
    pub fn new(stream: UnixStream) -> Self {
        Self { stream }
    }

    /// Process requests line-by-line with a handler function
    ///
    /// The handler function receives the request JSON string and should return
    /// a response JSON string (or error).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that processes each request
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Cannot read from stream
    /// - Cannot write to stream
    pub async fn process_requests<F, Fut>(self, handler: F) -> Result<()>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        let (reader, writer) = self.stream.into_split();
        let mut reader = BufReader::new(reader);
        let writer = std::sync::Arc::new(tokio::sync::Mutex::new(writer));
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                debug!("Connection closed");
                break;
            }

            let request_line = line.trim().to_string();
            if request_line.is_empty() {
                continue;
            }

            // Process request
            let response = match handler(request_line).await {
                Ok(response_json) => response_json,
                Err(e) => {
                    error!("Request handler error: {}", e);
                    // Send error response
                    format!(r#"{{"type":"error","message":"{}"}}"#, e)
                }
            };

            // Send response
            let mut w = writer.lock().await;
            w.write_all(response.as_bytes()).await?;
            w.write_all(b"\n").await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_socket_server_bind() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let server = UnixSocketServer::bind(&socket_path).await.unwrap();
        assert_eq!(server.path(), socket_path);
        assert!(socket_path.exists());
    }

    #[tokio::test]
    async fn test_client_server_communication() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        // Start server in background
        let server_path = socket_path.clone();
        tokio::spawn(async move {
            let server = UnixSocketServer::bind(&server_path).await.unwrap();
            let stream = server.accept().await.unwrap();
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            // Echo server
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            writer.write_all(line.as_bytes()).await.unwrap();
        });

        // Give server time to start
        sleep(Duration::from_millis(50)).await;

        // Connect client
        let client = UnixSocketClient::new(&socket_path);
        assert!(client.is_available());

        // Send request (as plain string for this test)
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestMsg {
            data: String,
        }

        let request = TestMsg {
            data: "hello".to_string(),
        };
        let response: TestMsg = client.send(&request).await.unwrap();
        assert_eq!(response.data, "hello");
    }

    #[tokio::test]
    async fn test_client_not_available() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("nonexistent.sock");

        let client = UnixSocketClient::new(&socket_path);
        assert!(!client.is_available());

        #[derive(Serialize)]
        struct TestReq {
            msg: String,
        }
        #[derive(Deserialize)]
        struct TestResp {
            #[allow(dead_code)]
            msg: String,
        }

        let result: Result<TestResp> = client
            .send(&TestReq {
                msg: "test".to_string(),
            })
            .await;
        assert!(matches!(result, Err(DaemonError::NotRunning)));
    }
}
