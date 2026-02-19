//! Cross-platform IPC transport abstraction
//!
//! Provides unified IPC that works across platforms:
//! - Unix: Unix domain sockets
//! - Windows: Named pipes
//! - Fallback: TCP localhost

use crate::error::{DaemonError, Result};
use crate::platform::Platform;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tracing::{debug, info};

/// Default timeout for IPC operations
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// IPC endpoint identifier
#[derive(Debug, Clone)]
pub enum IpcEndpoint {
    /// Unix socket path (Linux/macOS)
    UnixSocket(PathBuf),
    /// Named pipe path (Windows) - e.g., `\\.\pipe\my-daemon`
    NamedPipe(String),
    /// TCP localhost with port
    Tcp(u16),
}

impl IpcEndpoint {
    /// Create an appropriate endpoint for the current platform
    ///
    /// On Unix: Uses Unix socket at the given path
    /// On Windows: Uses named pipe with name derived from path
    pub fn for_path<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        
        match Platform::current() {
            Platform::Windows => {
                // Convert path to named pipe name
                // e.g., /var/run/daemon.sock -> \\.\pipe\daemon
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("daemon");
                IpcEndpoint::NamedPipe(format!(r"\\.\pipe\{}", name))
            }
            _ => IpcEndpoint::UnixSocket(path.to_path_buf()),
        }
    }

    /// Create a TCP localhost endpoint
    pub fn tcp(port: u16) -> Self {
        IpcEndpoint::Tcp(port)
    }

    /// Get display string for this endpoint
    pub fn display(&self) -> String {
        match self {
            IpcEndpoint::UnixSocket(path) => path.display().to_string(),
            IpcEndpoint::NamedPipe(name) => name.clone(),
            IpcEndpoint::Tcp(port) => format!("tcp://127.0.0.1:{}", port),
        }
    }
}

/// Unified IPC server that works across platforms
pub struct IpcServer {
    inner: IpcServerInner,
    endpoint: IpcEndpoint,
}

enum IpcServerInner {
    #[cfg(unix)]
    Unix(tokio::net::UnixListener),
    Tcp(tokio::net::TcpListener),
    #[cfg(windows)]
    NamedPipe(WindowsNamedPipeServer),
}

impl IpcServer {
    /// Bind to an IPC endpoint
    pub async fn bind(endpoint: IpcEndpoint) -> Result<Self> {
        let inner = match &endpoint {
            #[cfg(unix)]
            IpcEndpoint::UnixSocket(path) => {
                // Remove existing socket
                if path.exists() {
                    std::fs::remove_file(path)?;
                }

                // Ensure parent directory exists
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let listener = tokio::net::UnixListener::bind(path).map_err(|e| {
                    DaemonError::SocketError(format!("Failed to bind Unix socket: {}", e))
                })?;

                // Set permissions (user only)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o600);
                    std::fs::set_permissions(path, perms)?;
                }

                info!("IPC server bound to Unix socket: {}", path.display());
                IpcServerInner::Unix(listener)
            }

            #[cfg(not(unix))]
            IpcEndpoint::UnixSocket(_) => {
                return Err(DaemonError::SocketError(
                    "Unix sockets not supported on this platform".to_string(),
                ));
            }

            IpcEndpoint::Tcp(port) => {
                let addr = format!("127.0.0.1:{}", port);
                let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
                    DaemonError::SocketError(format!("Failed to bind TCP: {}", e))
                })?;
                info!("IPC server bound to TCP: {}", addr);
                IpcServerInner::Tcp(listener)
            }

            #[cfg(windows)]
            IpcEndpoint::NamedPipe(name) => {
                let server = WindowsNamedPipeServer::bind(name)?;
                info!("IPC server bound to named pipe: {}", name);
                IpcServerInner::NamedPipe(server)
            }

            #[cfg(not(windows))]
            IpcEndpoint::NamedPipe(_) => {
                return Err(DaemonError::SocketError(
                    "Named pipes only supported on Windows".to_string(),
                ));
            }
        };

        Ok(Self { inner, endpoint })
    }

    /// Accept a connection
    pub async fn accept(&self) -> Result<IpcStream> {
        match &self.inner {
            #[cfg(unix)]
            IpcServerInner::Unix(listener) => {
                let (stream, _) = listener.accept().await.map_err(|e| {
                    DaemonError::SocketError(format!("Failed to accept: {}", e))
                })?;
                debug!("Accepted Unix socket connection");
                Ok(IpcStream::Unix(stream))
            }

            IpcServerInner::Tcp(listener) => {
                let (stream, addr) = listener.accept().await.map_err(|e| {
                    DaemonError::SocketError(format!("Failed to accept: {}", e))
                })?;
                debug!("Accepted TCP connection from {}", addr);
                Ok(IpcStream::Tcp(stream))
            }

            #[cfg(windows)]
            IpcServerInner::NamedPipe(server) => {
                let stream = server.accept().await?;
                debug!("Accepted named pipe connection");
                Ok(IpcStream::NamedPipe(stream))
            }
        }
    }

    /// Get the endpoint this server is bound to
    pub fn endpoint(&self) -> &IpcEndpoint {
        &self.endpoint
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        // Clean up Unix socket file
        #[cfg(unix)]
        if let IpcEndpoint::UnixSocket(path) = &self.endpoint {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Unified IPC stream (connection)
pub enum IpcStream {
    #[cfg(unix)]
    Unix(tokio::net::UnixStream),
    Tcp(tokio::net::TcpStream),
    #[cfg(windows)]
    NamedPipe(WindowsNamedPipeStream),
}

impl IpcStream {
    /// Split into reader and writer
    pub fn into_split(self) -> (IpcReader, IpcWriter) {
        match self {
            #[cfg(unix)]
            IpcStream::Unix(stream) => {
                let (read, write) = stream.into_split();
                (IpcReader::Unix(read), IpcWriter::Unix(write))
            }
            IpcStream::Tcp(stream) => {
                let (read, write) = stream.into_split();
                (IpcReader::Tcp(read), IpcWriter::Tcp(write))
            }
            #[cfg(windows)]
            IpcStream::NamedPipe(stream) => {
                let (read, write) = stream.into_split();
                (IpcReader::NamedPipe(read), IpcWriter::NamedPipe(write))
            }
        }
    }
}

/// IPC reader half
pub enum IpcReader {
    #[cfg(unix)]
    Unix(tokio::net::unix::OwnedReadHalf),
    Tcp(tokio::net::tcp::OwnedReadHalf),
    #[cfg(windows)]
    NamedPipe(WindowsNamedPipeReadHalf),
}

/// IPC writer half
pub enum IpcWriter {
    #[cfg(unix)]
    Unix(tokio::net::unix::OwnedWriteHalf),
    Tcp(tokio::net::tcp::OwnedWriteHalf),
    #[cfg(windows)]
    NamedPipe(WindowsNamedPipeWriteHalf),
}

/// Unified IPC client
pub struct IpcClient {
    endpoint: IpcEndpoint,
    timeout: Duration,
}

impl IpcClient {
    /// Create a new IPC client
    pub fn new(endpoint: IpcEndpoint) -> Self {
        Self {
            endpoint,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Create client for a path (auto-selects transport based on platform)
    pub fn for_path<P: AsRef<Path>>(path: P) -> Self {
        Self::new(IpcEndpoint::for_path(path))
    }

    /// Set timeout for operations
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if the IPC endpoint is available
    pub fn is_available(&self) -> bool {
        match &self.endpoint {
            IpcEndpoint::UnixSocket(path) => path.exists(),
            IpcEndpoint::NamedPipe(_name) => {
                #[cfg(windows)]
                {
                    // Try to check if pipe exists
                    // For now, just return true and let connect fail if not
                    true
                }
                #[cfg(not(windows))]
                {
                    false
                }
            }
            IpcEndpoint::Tcp(_) => true, // Can't check without connecting
        }
    }

    /// Connect to the IPC endpoint
    pub async fn connect(&self) -> Result<IpcStream> {
        let connect_future = self.connect_inner();
        
        tokio::time::timeout(self.timeout, connect_future)
            .await
            .map_err(|_| DaemonError::Timeout(self.timeout.as_secs()))?
    }

    async fn connect_inner(&self) -> Result<IpcStream> {
        match &self.endpoint {
            #[cfg(unix)]
            IpcEndpoint::UnixSocket(path) => {
                if !path.exists() {
                    return Err(DaemonError::NotRunning);
                }
                let stream = tokio::net::UnixStream::connect(path).await.map_err(|e| {
                    DaemonError::SocketError(format!("Failed to connect: {}", e))
                })?;
                debug!("Connected to Unix socket: {}", path.display());
                Ok(IpcStream::Unix(stream))
            }

            #[cfg(not(unix))]
            IpcEndpoint::UnixSocket(_) => {
                Err(DaemonError::SocketError(
                    "Unix sockets not supported on this platform".to_string(),
                ))
            }

            IpcEndpoint::Tcp(port) => {
                let addr = format!("127.0.0.1:{}", port);
                let stream = tokio::net::TcpStream::connect(&addr).await.map_err(|e| {
                    DaemonError::SocketError(format!("Failed to connect: {}", e))
                })?;
                debug!("Connected to TCP: {}", addr);
                Ok(IpcStream::Tcp(stream))
            }

            #[cfg(windows)]
            IpcEndpoint::NamedPipe(name) => {
                let stream = WindowsNamedPipeStream::connect(name).await?;
                debug!("Connected to named pipe: {}", name);
                Ok(IpcStream::NamedPipe(stream))
            }

            #[cfg(not(windows))]
            IpcEndpoint::NamedPipe(_) => {
                Err(DaemonError::SocketError(
                    "Named pipes only supported on Windows".to_string(),
                ))
            }
        }
    }

    /// Send a request and receive a response (convenience method)
    pub async fn request<Req, Resp>(&self, request: &Req) -> Result<Resp>
    where
        Req: Serialize,
        Resp: for<'de> Deserialize<'de>,
    {
        let stream = self.connect().await?;
        
        // Use the stream for request/response
        match stream {
            #[cfg(unix)]
            IpcStream::Unix(stream) => {
                send_receive(stream, request).await
            }
            IpcStream::Tcp(stream) => {
                send_receive(stream, request).await
            }
            #[cfg(windows)]
            IpcStream::NamedPipe(stream) => {
                send_receive_pipe(stream, request).await
            }
        }
    }
}

/// Helper to send request and receive response over any async stream
async fn send_receive<S, Req, Resp>(stream: S, request: &Req) -> Result<Resp>
where
    S: AsyncRead + AsyncWrite + Unpin,
    Req: Serialize,
    Resp: for<'de> Deserialize<'de>,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    // Send request
    let request_json = serde_json::to_string(request)?;
    writer.write_all(request_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Read response
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    if response_line.is_empty() {
        return Err(DaemonError::ProtocolError("Connection closed".to_string()));
    }

    let response: Resp = serde_json::from_str(response_line.trim())?;
    Ok(response)
}

// Windows named pipe implementation stubs
// These would be implemented using tokio::net::windows::named_pipe in a real implementation

#[cfg(windows)]
struct WindowsNamedPipeServer {
    name: String,
}

#[cfg(windows)]
impl WindowsNamedPipeServer {
    fn bind(name: &str) -> Result<Self> {
        // Would use tokio::net::windows::named_pipe::ServerOptions
        Ok(Self { name: name.to_string() })
    }

    async fn accept(&self) -> Result<WindowsNamedPipeStream> {
        // Would accept connection on named pipe
        todo!("Windows named pipe accept")
    }
}

#[cfg(windows)]
struct WindowsNamedPipeStream {
    // Would hold the pipe handle
}

#[cfg(windows)]
impl WindowsNamedPipeStream {
    async fn connect(name: &str) -> Result<Self> {
        // Would use tokio::net::windows::named_pipe::ClientOptions
        todo!("Windows named pipe connect")
    }

    fn into_split(self) -> (WindowsNamedPipeReadHalf, WindowsNamedPipeWriteHalf) {
        todo!("Windows named pipe split")
    }
}

#[cfg(windows)]
struct WindowsNamedPipeReadHalf;

#[cfg(windows)]
struct WindowsNamedPipeWriteHalf;

#[cfg(windows)]
async fn send_receive_pipe<Req, Resp>(
    _stream: WindowsNamedPipeStream,
    _request: &Req,
) -> Result<Resp>
where
    Req: Serialize,
    Resp: for<'de> Deserialize<'de>,
{
    todo!("Windows named pipe request/response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_for_path_unix() {
        #[cfg(unix)]
        {
            let endpoint = IpcEndpoint::for_path("/var/run/test.sock");
            match endpoint {
                IpcEndpoint::UnixSocket(path) => {
                    assert_eq!(path, PathBuf::from("/var/run/test.sock"));
                }
                _ => panic!("Expected UnixSocket"),
            }
        }
    }

    #[test]
    fn test_endpoint_tcp() {
        let endpoint = IpcEndpoint::tcp(8080);
        match endpoint {
            IpcEndpoint::Tcp(port) => assert_eq!(port, 8080),
            _ => panic!("Expected Tcp"),
        }
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_ipc_server_client() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        let endpoint = IpcEndpoint::UnixSocket(socket_path.clone());

        // Start server
        let server = IpcServer::bind(endpoint.clone()).await.unwrap();

        // Spawn echo handler
        let server_handle = tokio::spawn(async move {
            let stream = server.accept().await.unwrap();
            match stream {
                IpcStream::Unix(s) => {
                    let (reader, mut writer) = s.into_split();
                    let mut reader = BufReader::new(reader);
                    let mut line = String::new();
                    reader.read_line(&mut line).await.unwrap();
                    writer.write_all(line.as_bytes()).await.unwrap();
                }
                _ => panic!("Expected Unix stream"),
            }
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Connect client
        let client = IpcClient::new(IpcEndpoint::UnixSocket(socket_path));
        assert!(client.is_available());

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestMsg {
            data: String,
        }

        let response: TestMsg = client
            .request(&TestMsg {
                data: "hello".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(response.data, "hello");

        server_handle.await.unwrap();
    }
}
