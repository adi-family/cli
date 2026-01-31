//! # Signaling Protocol
//!
//! WebSocket message protocol for ADI signaling infrastructure.
//! Used by: hive (cocoon orchestration), cocoon (worker), signaling-server (relay), platform-api.
//!
//! ## Features
//! - Device registration and pairing
//! - Cocoon lifecycle management (spawn, terminate)
//! - WebRTC signaling (offer, answer, ICE candidates)
//! - SSL/TLS certificate management
//! - Browser debugging protocol
//! - Service proxy and query aggregation
//! - Silk terminal protocol (interactive shells)
//!
//! ## Architecture Decision
//! Extracted from `lib-tarminal-sync` to avoid coupling hive/cocoon/platform-api
//! to terminal CRDT synchronization.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

// Re-export common types
pub use chrono;
pub use serde;
pub use serde_json;
pub use uuid;

/// Signaling server messages for device pairing and relay
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    /// Register device with server using client secret
    /// Server derives deterministic device_id from secret using HMAC
    /// On reconnect, device_id must match derived ID (prevents secret theft attacks)
    Register {
        secret: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        device_id: Option<String>,
        /// Cocoon binary version (e.g., "0.1.0")
        version: String,
    },

    /// Register with setup token (one-command install flow)
    /// Server validates JWT, derives device_id, and auto-claims for the token's user
    /// Used by: curl https://adi.dev/cocoon.sh | sh -s -- <setup_token>
    RegisterWithSetupToken {
        secret: String,
        setup_token: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>, // Optional display name for this cocoon
        /// Cocoon binary version (e.g., "0.1.0")
        version: String,
    },

    /// Registration confirmed with derived device ID
    /// Same secret always produces same device_id (persistent sessions)
    Registered { device_id: String },

    /// Deregister device (graceful disconnect)
    /// Sent by cocoon before shutdown to notify server of intentional disconnect
    Deregister {
        device_id: String,
        /// Reason for deregistration (shutdown, removal, update, etc.)
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// Deregistration confirmed
    Deregistered { device_id: String },

    /// Registration with setup token confirmed
    /// Includes user_id that now owns this cocoon
    RegisteredWithOwner {
        device_id: String,
        owner_id: String,
        name: Option<String>,
    },

    /// Create a pairing code
    CreatePairingCode,

    /// Pairing code generated
    PairingCode { code: String },

    /// Use a pairing code to connect
    UsePairingCode { code: String },

    /// Successfully paired with peer
    Paired { peer_id: String },

    /// Pairing failed
    PairingFailed { reason: String },

    /// Sync data payload (forwarded as-is)
    SyncData { payload: JsonValue },

    /// Peer came online
    PeerConnected { peer_id: String },

    /// Peer went offline
    PeerDisconnected { peer_id: String },

    // ========== Token-Based Ownership ==========
    /// Claim ownership of a cocoon by proving secret knowledge
    /// Multiple users can claim the same cocoon as co-owners
    ClaimCocoon {
        device_id: String,
        secret: String,
        access_token: String, // JWT or API token from auth system
    },

    /// Claim successful - user is now an owner
    ClaimSuccessful { device_id: String },

    /// Connect to cocoon using access token
    /// Only owners (users who claimed with secret) can connect
    ConnectToCocoon {
        device_id: String,
        access_token: String,
    },

    /// Connection successful - paired with cocoon
    Connected { device_id: String },

    /// List all cocoons owned by this token
    ListMyCocoons { access_token: String },

    /// List of owned cocoons
    MyCocoons { cocoons: Vec<CocoonInfo> },

    /// Remove cocoon ownership (user wants to delete/unlink this cocoon)
    RemoveCocoon {
        device_id: String,
        access_token: String,
    },

    /// Cocoon removed successfully
    CocoonRemoved { device_id: String },

    /// Access denied (not an owner)
    AccessDenied { reason: String },

    // ========== Service Registration ==========
    /// Register local services (HTTP endpoints) with signaling server
    ServiceRegister { services: Vec<ServiceInfo> },

    /// Service registration confirmed
    ServiceRegistered {
        device_id: String,
        services: Vec<ServiceInfo>,
    },

    // ========== HTTP Proxy ==========
    /// Proxy HTTP request to a service on target device
    ProxyRequest {
        request_id: String,
        target_device_id: String,
        service_name: String,
        method: String,
        path: String,
        headers: HashMap<String, String>,
        body: Option<String>,
    },

    /// Proxy response from target device
    ProxyResponse {
        request_id: String,
        status_code: u16,
        headers: HashMap<String, String>,
        body: Option<String>,
    },

    // ========== Query Aggregation ==========
    /// Aggregate query across all user's devices
    AggregateQuery {
        query_id: String,
        query_type: QueryType,
        params: JsonValue,
    },

    /// Partial query result from a device
    AggregateQueryPart {
        query_id: String,
        from_device: String,
        data: JsonValue,
        is_final: bool,
    },

    // ========== Device Capabilities ==========
    /// Update device capabilities (auto-discovered from plugins)
    CapabilitiesUpdate { capabilities: Vec<Capability> },

    /// Request capability from another device (cocoon-to-cocoon)
    CapabilityRequest {
        request_id: String,
        capability: Capability,
        payload: JsonValue,
        prefer_device: Option<String>,
    },

    /// Response to capability request
    CapabilityResponse {
        request_id: String,
        from_device: String,
        payload: JsonValue,
        error: Option<String>,
    },

    /// Error message
    Error { message: String },

    // ========== Hive Orchestration ==========
    /// Register as Hive orchestrator (special client that spawns cocoons)
    /// Authentication: hive_id is signed with HMAC-SHA256 using shared HIVE_SECRET
    /// Signaling server verifies signature to ensure Hive knows the secret
    RegisterHive {
        /// Hive instance identifier
        hive_id: String,
        /// Hive version
        version: String,
        /// Available cocoon kinds this hive can spawn
        cocoon_kinds: Vec<CocoonKind>,
        /// HMAC-SHA256 signature of hive_id using HIVE_SECRET as key (hex-encoded)
        /// Proves Hive knows the secret without transmitting it
        hive_id_signature: String,
    },

    /// Hive registration confirmed
    HiveRegistered { hive_id: String },

    /// Request Hive to spawn a new cocoon
    /// Sent by: Platform API, CLI, or any authorized client
    /// Received by: Hive
    SpawnCocoon {
        /// Unique request ID for tracking
        request_id: String,
        /// Setup token (JWT) for the cocoon - contains user_id
        setup_token: String,
        /// Optional display name for the cocoon
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Cocoon kind to spawn (must match a kind from HiveInfo.cocoon_kinds)
        kind: String,
    },

    /// Cocoon spawn result
    /// Sent by: Hive
    SpawnCocoonResult {
        /// Request ID from SpawnCocoon
        request_id: String,
        /// Whether spawn was successful
        success: bool,
        /// Device ID of spawned cocoon (if successful)
        #[serde(skip_serializing_if = "Option::is_none")]
        device_id: Option<String>,
        /// Container ID (if successful)
        #[serde(skip_serializing_if = "Option::is_none")]
        container_id: Option<String>,
        /// Error message (if failed)
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },

    /// Request Hive to terminate a cocoon
    TerminateCocoon {
        /// Unique request ID for tracking
        request_id: String,
        /// Container name or ID to terminate
        container_id: String,
    },

    /// Cocoon termination result
    TerminateCocoonResult {
        request_id: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },

    /// List all connected Hive orchestrators
    ListHives { access_token: String },

    /// List of connected hives
    HivesList { hives: Vec<HiveInfo> },

    // ========== SSL Certificate Management ==========
    /// Request Hive to issue an SSL certificate for a domain
    /// Sent by: CLI, Platform API, or any authorized client
    /// Received by: Hive
    RequestCertificate {
        /// Unique request ID for tracking
        request_id: String,
        /// Domain(s) to issue certificate for
        domains: Vec<String>,
        /// ACME account email (required by Let's Encrypt)
        email: String,
        /// Use staging environment for testing (default: false)
        #[serde(default)]
        staging: bool,
        /// Challenge type preference: "http01", "tls-alpn01", or "auto"
        #[serde(default)]
        challenge_type: Option<String>,
    },

    /// Certificate issuance result
    /// Sent by: Hive
    CertificateIssued {
        /// Request ID from RequestCertificate
        request_id: String,
        /// Whether issuance was successful
        success: bool,
        /// Primary domain (if successful)
        #[serde(skip_serializing_if = "Option::is_none")]
        domain: Option<String>,
        /// Certificate expiry date ISO 8601 (if successful)
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<String>,
        /// Error message (if failed)
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },

    /// Request certificate status for domains
    GetCertificateStatus {
        request_id: String,
        /// Domains to check (empty = all)
        domains: Vec<String>,
    },

    /// Certificate status response
    CertificateStatus {
        request_id: String,
        certificates: Vec<CertificateInfo>,
    },

    // ========== Browser Debug ==========
    /// Browser extension registers a tab with debug token
    /// Sent by: Chrome extension when detecting X-ADI-Debug-Token header
    BrowserDebugTabAvailable {
        /// Debug token from X-ADI-Debug-Token header
        token: String,
        /// Unique browser instance ID (generated on install)
        browser_id: String,
        /// Current page URL
        url: String,
        /// Page title
        title: String,
        /// Favicon URL (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        favicon: Option<String>,
    },

    /// Browser tab closed or navigated away
    BrowserDebugTabClosed { token: String },

    /// Browser tab updated (SPA navigation)
    BrowserDebugTabUpdated {
        token: String,
        url: String,
        title: String,
    },

    /// Network event streamed from browser extension
    BrowserDebugNetworkEvent {
        token: String,
        event: NetworkEventType,
        data: NetworkEventData,
    },

    /// Console event streamed from browser extension
    BrowserDebugConsoleEvent { token: String, entry: ConsoleEntry },

    /// List all debug tabs available to this user
    /// Sent by: MCP plugin
    BrowserDebugListTabs { access_token: String },

    /// Response with list of debug tabs
    BrowserDebugTabs { tabs: Vec<BrowserDebugTab> },

    /// Get network requests from a tab
    /// Sent by: MCP plugin, routed to extension
    BrowserDebugGetNetwork {
        request_id: String,
        token: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filters: Option<NetworkFilters>,
    },

    /// Network data response from extension
    BrowserDebugNetworkData {
        request_id: String,
        requests: Vec<NetworkRequest>,
    },

    /// Get console logs from a tab
    /// Sent by: MCP plugin, routed to extension
    BrowserDebugGetConsole {
        request_id: String,
        token: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filters: Option<ConsoleFilters>,
    },

    /// Console data response from extension
    BrowserDebugConsoleData {
        request_id: String,
        entries: Vec<ConsoleEntry>,
    },

    // ========== WebRTC Session Management ==========
    /// Request to start a WebRTC session with a cocoon
    /// Sent by: Browser/Client to initiate WebRTC connection
    WebRtcStartSession {
        /// Unique session ID for this WebRTC connection
        session_id: String,
        /// Target cocoon device_id
        device_id: String,
        /// JWT access token for authorization
        access_token: String,
    },

    /// WebRTC session started confirmation
    /// Sent by: Signaling server to client after forwarding to cocoon
    WebRtcSessionStarted {
        session_id: String,
        device_id: String,
    },

    /// WebRTC SDP offer
    /// Sent by: Client (browser) to cocoon via signaling server
    WebRtcOffer {
        session_id: String,
        /// SDP offer (Session Description Protocol)
        sdp: String,
    },

    /// WebRTC SDP answer
    /// Sent by: Cocoon to client via signaling server
    WebRtcAnswer {
        session_id: String,
        /// SDP answer
        sdp: String,
    },

    /// WebRTC ICE candidate
    /// Sent by: Both client and cocoon during ICE negotiation
    WebRtcIceCandidate {
        session_id: String,
        /// ICE candidate in JSON format
        candidate: String,
        /// SDP mid (media stream identification)
        #[serde(skip_serializing_if = "Option::is_none")]
        sdp_mid: Option<String>,
        /// SDP m-line index
        #[serde(skip_serializing_if = "Option::is_none")]
        sdp_mline_index: Option<u32>,
    },

    /// WebRTC session ended
    /// Sent by: Either party when closing the connection
    WebRtcSessionEnded {
        session_id: String,
        /// Reason for ending (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// WebRTC session error
    /// Sent by: Signaling server or cocoon when an error occurs
    WebRtcError {
        session_id: String,
        code: String,
        message: String,
    },

    /// WebRTC data channel message
    /// For sending/receiving data through the established WebRTC connection
    /// This goes through signaling server as fallback or for initial setup
    WebRtcData {
        session_id: String,
        /// Channel label (e.g., "terminal", "file-transfer")
        channel: String,
        /// Data payload (JSON or base64 for binary)
        data: String,
        /// Whether data is base64 encoded binary
        #[serde(default)]
        binary: bool,
    },
}

/// Information about a connected Hive orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveInfo {
    pub hive_id: String,
    pub version: String,
    pub status: String, // "online"
    pub connected_at: String,
    /// Available cocoon kinds this hive can spawn
    pub cocoon_kinds: Vec<CocoonKind>,
}

/// Available cocoon image/kind that a Hive can spawn
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CocoonKind {
    /// Unique identifier (e.g., "linux", "linux-cuda", "macos", "native")
    pub id: String,

    /// Runner type: "docker" (default), "script", "podman"
    #[serde(default = "default_runner_type")]
    pub runner_type: String,

    /// Runner configuration (JSON object)
    /// - For docker: { "image": "registry/cocoon:tag" }
    /// - For script: { "command": "cocoon-worker", "args": ["--kind", "{kind}"] }
    /// - For podman: { "image": "registry/cocoon:tag" }
    #[serde(default = "default_runner_config")]
    pub runner_config: serde_json::Value,

    /// DEPRECATED: Docker image to use (kept for backward compatibility)
    /// Use runner_config instead
    #[serde(default)]
    pub image: String,
}

fn default_runner_type() -> String {
    "docker".to_string()
}

fn default_runner_config() -> serde_json::Value {
    serde_json::json!({})
}

/// WebRTC session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRtcSessionInfo {
    /// Unique session ID
    pub session_id: String,
    /// Client device/user ID
    pub client_id: String,
    /// Target cocoon device ID
    pub cocoon_id: String,
    /// Session state: "pending", "connecting", "connected", "disconnected"
    pub state: String,
    /// When the session was created
    pub created_at: String,
    /// ICE connection state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ice_state: Option<String>,
}

/// SSL Certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    /// Primary domain
    pub domain: String,
    /// All domains covered by this certificate (SANs)
    pub domains: Vec<String>,
    /// Certificate expiry date (ISO 8601)
    pub expires_at: String,
    /// Days until expiry
    pub days_until_expiry: i64,
    /// Whether certificate needs renewal (< 30 days)
    pub needs_renewal: bool,
    /// Certificate issuer (e.g., "Let's Encrypt")
    pub issuer: String,
}

/// Information about an owned cocoon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoonInfo {
    pub device_id: String,
    pub status: String,     // "online" or "offline"
    pub claimed_at: String, // ISO 8601 datetime when claimed
    #[serde(default)]
    pub services: Vec<ServiceInfo>,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Service information for HTTP proxying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub service_type: ServiceType,
    pub local_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_endpoint: Option<String>,
}

/// Service type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    Http,
    Grpc,
    Custom,
}

/// Device capability descriptor
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// Protocol/capability name (e.g., "tasks", "embeddings", "llm.chat")
    pub protocol: String,
    /// Semantic version (e.g., "1.0.0", "2.3.1")
    pub version: String,
}

/// Query types for aggregation across devices
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    ListTasks,
    GetTaskStats,
    SearchTasks,
    SearchKnowledgebase,
    Custom { query_name: String },
}

// ========== Browser Debug Types ==========

/// Network event type for streaming
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkEventType {
    Request,
    Response,
    Finished,
    Failed,
}

/// Network event data (varies by event type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEventData {
    pub request_id: String,
    pub timestamp: i64,
    // Request fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    // Response fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    // Finished fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body_truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    // Failed fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Console log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleEntry {
    pub timestamp: i64,
    pub level: ConsoleLevel,
    pub message: String,
    #[serde(default)]
    pub args: Vec<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
}

/// Console log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleLevel {
    Log,
    Debug,
    Info,
    Warn,
    Error,
}

/// Browser debug tab info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserDebugTab {
    pub token: String,
    pub browser_id: String,
    pub url: String,
    pub title: String,
    pub cocoon_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cocoon_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
}

/// Network request filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_min: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_max: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Console log filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsoleFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<Vec<ConsoleLevel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Complete network request (aggregated from events)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub request_id: String,
    pub timestamp: i64,
    // Request
    pub method: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    // Response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body_truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    // Timing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    // Error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ========== Silk Terminal Protocol ==========

/// Silk command request - sent from web to cocoon via SyncData
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SilkRequest {
    /// Create a new Silk session (persistent shell for env preservation)
    CreateSession {
        /// Initial working directory
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        /// Initial environment variables
        #[serde(default)]
        env: HashMap<String, String>,
        /// Shell to use (default: user's shell or /bin/sh)
        #[serde(skip_serializing_if = "Option::is_none")]
        shell: Option<String>,
    },

    /// Execute a command in the Silk session
    Execute {
        /// Session ID
        session_id: Uuid,
        /// Command to execute
        command: String,
        /// Unique ID for this command execution (for matching output)
        command_id: Uuid,
    },

    /// Send input to an interactive command (running in PTY mode)
    Input {
        session_id: Uuid,
        command_id: Uuid,
        data: String,
    },

    /// Resize interactive terminal
    Resize {
        session_id: Uuid,
        command_id: Uuid,
        cols: u16,
        rows: u16,
    },

    /// Send signal to running command (e.g., Ctrl+C)
    Signal {
        session_id: Uuid,
        command_id: Uuid,
        signal: SilkSignal,
    },

    /// Close session
    CloseSession { session_id: Uuid },
}

/// Signals that can be sent to running commands
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SilkSignal {
    Interrupt, // SIGINT (Ctrl+C)
    Terminate, // SIGTERM
    Kill,      // SIGKILL
}

/// Silk response - sent from cocoon to web via SyncData
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SilkResponse {
    /// Session created successfully
    SessionCreated {
        session_id: Uuid,
        /// Current working directory
        cwd: String,
        /// Shell being used
        shell: String,
    },

    /// Command started executing
    CommandStarted {
        session_id: Uuid,
        command_id: Uuid,
        /// Whether command was detected as interactive
        interactive: bool,
    },

    /// Output chunk from command (non-interactive mode)
    Output {
        session_id: Uuid,
        command_id: Uuid,
        /// Output stream (stdout/stderr)
        stream: SilkStream,
        /// Raw output data (may contain ANSI codes)
        data: String,
        /// Pre-parsed HTML spans (optional, cocoon can provide)
        #[serde(skip_serializing_if = "Option::is_none")]
        html: Option<Vec<SilkHtmlSpan>>,
    },

    /// Command requires interactive mode - switch to PTY
    InteractiveRequired {
        session_id: Uuid,
        command_id: Uuid,
        /// Reason why interactive mode is needed
        reason: String,
        /// PTY session ID to connect xterm to
        pty_session_id: Uuid,
    },

    /// Interactive PTY output (when in interactive mode)
    PtyOutput {
        session_id: Uuid,
        command_id: Uuid,
        pty_session_id: Uuid,
        data: String,
    },

    /// Command completed
    CommandCompleted {
        session_id: Uuid,
        command_id: Uuid,
        exit_code: i32,
        /// Updated working directory (in case cd was run)
        cwd: String,
    },

    /// Session closed
    SessionClosed { session_id: Uuid },

    /// Error occurred
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<Uuid>,
        #[serde(skip_serializing_if = "Option::is_none")]
        command_id: Option<Uuid>,
        code: String,
        message: String,
    },
}

/// Output stream identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SilkStream {
    Stdout,
    Stderr,
}

/// Pre-parsed HTML span for styled output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilkHtmlSpan {
    /// Text content
    pub text: String,
    /// CSS classes to apply
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    /// Inline styles (color, background, etc.)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub styles: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signaling_message_serialization() {
        let msg = SignalingMessage::Register {
            secret: "test-secret-with-at-least-32-chars-for-validation".to_string(),
            device_id: None,
            version: "0.2.1".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            SignalingMessage::Register {
                secret,
                device_id,
                version,
            } => {
                assert_eq!(secret, "test-secret-with-at-least-32-chars-for-validation");
                assert_eq!(device_id, None);
                assert_eq!(version, "0.2.1");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_cocoon_kind_serialization() {
        let kind = CocoonKind {
            id: "ubuntu".to_string(),
            image: "git.the-ihor.com/adi/cocoon:ubuntu".to_string(),
        };

        let json = serde_json::to_string(&kind).unwrap();
        let deserialized: CocoonKind = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "ubuntu");
        assert_eq!(deserialized.image, "git.the-ihor.com/adi/cocoon:ubuntu");
    }

    #[test]
    fn test_certificate_info_serialization() {
        let cert = CertificateInfo {
            domain: "example.com".to_string(),
            domains: vec!["example.com".to_string(), "www.example.com".to_string()],
            expires_at: "2026-04-18T00:00:00Z".to_string(),
            days_until_expiry: 90,
            needs_renewal: false,
            issuer: "Let's Encrypt".to_string(),
        };

        let json = serde_json::to_string(&cert).unwrap();
        let deserialized: CertificateInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.domain, "example.com");
        assert_eq!(deserialized.days_until_expiry, 90);
        assert!(!deserialized.needs_renewal);
    }
}
