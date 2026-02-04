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

// ========== ADI Service Protocol ==========
//
// Generic protocol for any ADI service to receive requests via WebRTC.
// Services register with cocoon and receive routed messages.
// Supports request/response and streaming patterns.
//
// ## MCP-Inspired Architecture
//
// This protocol follows patterns from MCP (Model Context Protocol):
// - Capability negotiation at connection time
// - Self-describing services with JSON Schema
// - Dynamic discovery of services and methods
// - Native streaming support
// - Subscriptions for real-time updates
//
// ## Connection Lifecycle
//
// 1. WebRTC data channel "adi" established
// 2. Client sends AdiInitialize::Request with capabilities
// 3. Cocoon responds with AdiInitialize::Response (capabilities + services)
// 4. Client sends AdiInitialize::Ready
// 5. Normal request/response flow begins
// 6. Async notifications may arrive at any time
// 7. Subscriptions enable push-based updates

/// ADI Protocol version (semver)
/// Major version changes indicate breaking protocol changes
/// Minor version changes add new optional features
pub const ADI_PROTOCOL_VERSION: &str = "1.0.0";

// ========== Capability Negotiation ==========

/// Protocol capabilities exchanged during initialization
/// Both client and cocoon declare what they support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiCapabilities {
    /// Protocol version (semver, e.g., "1.0.0")
    pub protocol_version: String,
    /// Client/cocoon endpoint info
    pub info: AdiEndpointInfo,
    /// Supported features
    pub features: AdiFeatures,
}

impl Default for AdiCapabilities {
    fn default() -> Self {
        Self {
            protocol_version: ADI_PROTOCOL_VERSION.to_string(),
            info: AdiEndpointInfo::default(),
            features: AdiFeatures::default(),
        }
    }
}

/// Information about an endpoint (client or cocoon)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiEndpointInfo {
    /// Endpoint name (e.g., "web-client", "cocoon-worker")
    pub name: String,
    /// Endpoint version
    pub version: String,
    /// Unique identifier (device_id for cocoon, session_id for client)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Default for AdiEndpointInfo {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            version: "0.0.0".to_string(),
            id: None,
        }
    }
}

/// Feature flags for capability negotiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiFeatures {
    /// Whether streaming responses are supported
    #[serde(default = "default_true")]
    pub streaming: bool,
    /// Whether notifications are supported
    #[serde(default = "default_true")]
    pub notifications: bool,
    /// Whether subscriptions are supported
    #[serde(default)]
    pub subscriptions: bool,
    /// Maximum message size in bytes (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_message_size: Option<usize>,
    /// Supported content types (default: ["json"])
    #[serde(default = "default_content_types")]
    pub content_types: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_content_types() -> Vec<String> {
    vec!["json".to_string()]
}

impl Default for AdiFeatures {
    fn default() -> Self {
        Self {
            streaming: true,
            notifications: true,
            subscriptions: false,
            max_message_size: None,
            content_types: default_content_types(),
        }
    }
}

// ========== Connection Initialization ==========

/// Initialization messages for establishing ADI protocol connection
/// Exchanged after WebRTC data channel is established
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdiInitialize {
    /// Client → Cocoon: Initialize connection with client capabilities
    Request {
        request_id: Uuid,
        capabilities: AdiCapabilities,
    },

    /// Cocoon → Client: Initialization response with capabilities and available services
    Response {
        request_id: Uuid,
        /// Negotiated capabilities (intersection of client + cocoon)
        capabilities: AdiCapabilities,
        /// Available services and their methods
        services: Vec<AdiServiceInfo>,
    },

    /// Client → Cocoon: Confirmation that client is ready
    /// After this, normal request/response flow begins
    Ready { request_id: Uuid },

    /// Error during initialization
    Error {
        request_id: Uuid,
        code: String,
        message: String,
    },
}

// ========== Notifications (Async Events) ==========

/// Notifications are async events sent without a request
/// No response is expected (fire-and-forget)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdiNotification {
    /// Service list changed (services added/removed/updated)
    ServicesChanged {
        /// Service IDs that were added
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        added: Vec<String>,
        /// Service IDs that were removed
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        removed: Vec<String>,
        /// Service IDs that were updated (version/methods changed)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        updated: Vec<String>,
    },

    /// Methods changed for a specific service
    MethodsChanged {
        service: String,
        /// Updated method list
        methods: Vec<AdiMethodInfo>,
    },

    /// Service-specific event (defined by plugin)
    ServiceEvent {
        service: String,
        /// Event name (e.g., "task_created", "index_updated")
        event: String,
        /// Event payload (service-defined)
        data: JsonValue,
    },

    /// Progress update for long-running operation
    Progress {
        /// Original request ID
        request_id: Uuid,
        /// Progress value (0.0 to 1.0)
        progress: f32,
        /// Optional human-readable message
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        /// Optional extra data
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<JsonValue>,
    },

    /// Connection health/keepalive
    Ping { timestamp: i64 },

    /// Response to ping
    Pong {
        timestamp: i64,
        /// Server timestamp for latency calculation
        server_timestamp: i64,
    },
}

// ========== Subscriptions ==========

/// Subscription management for real-time updates
/// Similar to WebSocket subscriptions or GraphQL subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdiSubscription {
    /// Client → Cocoon: Subscribe to service events
    Subscribe {
        request_id: Uuid,
        /// Service to subscribe to
        service: String,
        /// Event name (e.g., "task_status", "log_stream", "*" for all)
        event: String,
        /// Optional filter for events
        #[serde(skip_serializing_if = "Option::is_none")]
        filter: Option<JsonValue>,
    },

    /// Cocoon → Client: Subscription confirmed
    Subscribed {
        request_id: Uuid,
        /// Unique subscription ID (for unsubscribe)
        subscription_id: Uuid,
        /// Service subscribed to
        service: String,
        /// Event subscribed to
        event: String,
    },

    /// Client → Cocoon: Unsubscribe from events
    Unsubscribe { subscription_id: Uuid },

    /// Cocoon → Client: Unsubscription confirmed
    Unsubscribed { subscription_id: Uuid },

    /// Cocoon → Client: Subscription event data
    /// Sent when subscribed event occurs
    Event {
        subscription_id: Uuid,
        service: String,
        event: String,
        data: JsonValue,
        /// Sequence number for ordering
        seq: u64,
    },

    /// Subscription error
    Error {
        request_id: Uuid,
        code: String,
        message: String,
    },
}

// ========== Service Capabilities ==========

/// Service-level capabilities (what a specific service supports)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiServiceCapabilities {
    /// Whether service supports subscriptions
    #[serde(default)]
    pub subscriptions: bool,
    /// Whether service emits notifications
    #[serde(default)]
    pub notifications: bool,
    /// Whether service methods can stream responses
    #[serde(default = "default_true")]
    pub streaming: bool,
}

impl Default for AdiServiceCapabilities {
    fn default() -> Self {
        Self {
            subscriptions: false,
            notifications: false,
            streaming: true,
        }
    }
}

// ========== Request/Response ==========

/// ADI service request - sent via "adi" WebRTC data channel
/// Generic request that gets routed to registered service handlers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdiRequest {
    /// Unique request ID for correlation
    pub request_id: Uuid,
    /// Target service identifier (e.g., "tasks", "indexer", "kb", "agent")
    pub service: String,
    /// Method to invoke (e.g., "list", "create", "search")
    pub method: String,
    /// Method parameters as JSON
    #[serde(default)]
    pub params: JsonValue,
}

/// ADI service response - sent back via "adi" WebRTC data channel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdiResponse {
    /// Successful response with data
    Success {
        request_id: Uuid,
        service: String,
        method: String,
        data: JsonValue,
    },

    /// Streaming response chunk (for long-running operations)
    /// Services push data through a pipe, each chunk becomes a Stream message
    Stream {
        request_id: Uuid,
        service: String,
        method: String,
        /// Chunk data
        data: JsonValue,
        /// Sequence number for ordering
        seq: u32,
        /// false = more chunks coming, true = final chunk
        done: bool,
    },

    /// Error response
    Error {
        request_id: Uuid,
        service: String,
        method: String,
        /// Error code (e.g., "not_found", "invalid_params", "internal")
        code: String,
        /// Human-readable error message
        message: String,
    },

    /// Service not found/not registered
    ServiceNotFound { request_id: Uuid, service: String },

    /// Method not supported by service
    MethodNotFound {
        request_id: Uuid,
        service: String,
        method: String,
        /// Available methods for discovery
        available_methods: Vec<String>,
    },
}

/// Service discovery messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdiDiscovery {
    /// Request list of available services
    ListServices { request_id: Uuid },

    /// Response with available services and their methods
    ServicesList {
        request_id: Uuid,
        services: Vec<AdiServiceInfo>,
    },
}

/// Information about a registered ADI service
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdiServiceInfo {
    /// Service identifier (e.g., "tasks", "indexer", "kb")
    pub id: String,
    /// Human-readable name (e.g., "Task Management")
    pub name: String,
    /// Service version (semver)
    #[serde(default)]
    pub version: String,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Available methods
    #[serde(default)]
    pub methods: Vec<AdiMethodInfo>,
    /// Service-level capabilities
    #[serde(default)]
    pub capabilities: AdiServiceCapabilities,
}

/// Information about a service method
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdiMethodInfo {
    /// Method name (e.g., "list", "create", "search")
    pub name: String,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Whether this method supports streaming responses
    #[serde(default)]
    pub streaming: bool,
    /// JSON Schema for input parameters (for validation and documentation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params_schema: Option<JsonValue>,
    /// JSON Schema for response data (for documentation)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_schema: Option<JsonValue>,
    /// Whether this method is deprecated
    #[serde(default)]
    pub deprecated: bool,
    /// Deprecation message (if deprecated)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecation_message: Option<String>,
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
            runner_type: "docker".to_string(),
            runner_config: serde_json::json!({"image": "git.the-ihor.com/adi/cocoon:ubuntu"}),
            image: "git.the-ihor.com/adi/cocoon:ubuntu".to_string(),
        };

        let json = serde_json::to_string(&kind).unwrap();
        let deserialized: CocoonKind = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "ubuntu");
        assert_eq!(deserialized.runner_type, "docker");
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

    #[test]
    fn test_adi_request_serialization() {
        let req = AdiRequest {
            request_id: Uuid::nil(),
            service: "tasks".to_string(),
            method: "list".to_string(),
            params: serde_json::json!({"status": "todo"}),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"service\":\"tasks\""));
        assert!(json.contains("\"method\":\"list\""));

        let deserialized: AdiRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.service, "tasks");
        assert_eq!(deserialized.method, "list");
        assert_eq!(deserialized.params["status"], "todo");
    }

    #[test]
    fn test_adi_response_success_serialization() {
        let resp = AdiResponse::Success {
            request_id: Uuid::nil(),
            service: "tasks".to_string(),
            method: "list".to_string(),
            data: serde_json::json!([{"id": 1, "title": "Test"}]),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"success\""));

        let deserialized: AdiResponse = serde_json::from_str(&json).unwrap();
        match deserialized {
            AdiResponse::Success {
                service,
                method,
                data,
                ..
            } => {
                assert_eq!(service, "tasks");
                assert_eq!(method, "list");
                assert!(data.is_array());
            }
            _ => panic!("Wrong response type"),
        }
    }

    #[test]
    fn test_adi_response_stream_serialization() {
        let resp = AdiResponse::Stream {
            request_id: Uuid::nil(),
            service: "agent".to_string(),
            method: "run".to_string(),
            data: serde_json::json!({"chunk": "Hello"}),
            seq: 1,
            done: false,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"type\":\"stream\""));
        assert!(json.contains("\"seq\":1"));
        assert!(json.contains("\"done\":false"));
    }

    #[test]
    fn test_adi_discovery_serialization() {
        let discovery = AdiDiscovery::ServicesList {
            request_id: Uuid::nil(),
            services: vec![AdiServiceInfo {
                id: "tasks".to_string(),
                name: "Task Management".to_string(),
                version: "0.1.0".to_string(),
                description: Some("Manage tasks and todos".to_string()),
                methods: vec![AdiMethodInfo {
                    name: "list".to_string(),
                    description: "List all tasks".to_string(),
                    streaming: false,
                    params_schema: None,
                    result_schema: None,
                    deprecated: false,
                    deprecation_message: None,
                }],
                capabilities: AdiServiceCapabilities::default(),
            }],
        };

        let json = serde_json::to_string(&discovery).unwrap();
        assert!(json.contains("\"type\":\"services_list\""));
        assert!(json.contains("\"id\":\"tasks\""));

        let deserialized: AdiDiscovery = serde_json::from_str(&json).unwrap();
        match deserialized {
            AdiDiscovery::ServicesList { services, .. } => {
                assert_eq!(services.len(), 1);
                assert_eq!(services[0].id, "tasks");
                assert_eq!(services[0].methods.len(), 1);
            }
            _ => panic!("Wrong discovery type"),
        }
    }

    #[test]
    fn test_adi_capabilities_default() {
        let caps = AdiCapabilities::default();
        assert_eq!(caps.protocol_version, ADI_PROTOCOL_VERSION);
        assert!(caps.features.streaming);
        assert!(caps.features.notifications);
        assert!(!caps.features.subscriptions);
    }

    #[test]
    fn test_adi_initialize_serialization() {
        let init = AdiInitialize::Request {
            request_id: Uuid::nil(),
            capabilities: AdiCapabilities {
                protocol_version: "1.0.0".to_string(),
                info: AdiEndpointInfo {
                    name: "web-client".to_string(),
                    version: "1.0.0".to_string(),
                    id: Some("session-123".to_string()),
                },
                features: AdiFeatures::default(),
            },
        };

        let json = serde_json::to_string(&init).unwrap();
        assert!(json.contains("\"type\":\"request\""));
        assert!(json.contains("\"protocol_version\":\"1.0.0\""));
        assert!(json.contains("\"name\":\"web-client\""));

        let deserialized: AdiInitialize = serde_json::from_str(&json).unwrap();
        match deserialized {
            AdiInitialize::Request { capabilities, .. } => {
                assert_eq!(capabilities.info.name, "web-client");
                assert!(capabilities.features.streaming);
            }
            _ => panic!("Wrong init type"),
        }
    }

    #[test]
    fn test_adi_notification_serialization() {
        let notif = AdiNotification::ServiceEvent {
            service: "tasks".to_string(),
            event: "task_created".to_string(),
            data: serde_json::json!({"task_id": "123", "title": "New task"}),
        };

        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"type\":\"service_event\""));
        assert!(json.contains("\"service\":\"tasks\""));
        assert!(json.contains("\"event\":\"task_created\""));

        let deserialized: AdiNotification = serde_json::from_str(&json).unwrap();
        match deserialized {
            AdiNotification::ServiceEvent {
                service,
                event,
                data,
            } => {
                assert_eq!(service, "tasks");
                assert_eq!(event, "task_created");
                assert_eq!(data["task_id"], "123");
            }
            _ => panic!("Wrong notification type"),
        }
    }

    #[test]
    fn test_adi_notification_progress() {
        let notif = AdiNotification::Progress {
            request_id: Uuid::nil(),
            progress: 0.5,
            message: Some("Indexing files...".to_string()),
            data: None,
        };

        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"type\":\"progress\""));
        assert!(json.contains("\"progress\":0.5"));

        let deserialized: AdiNotification = serde_json::from_str(&json).unwrap();
        match deserialized {
            AdiNotification::Progress {
                progress, message, ..
            } => {
                assert!((progress - 0.5).abs() < 0.001);
                assert_eq!(message, Some("Indexing files...".to_string()));
            }
            _ => panic!("Wrong notification type"),
        }
    }

    #[test]
    fn test_adi_subscription_serialization() {
        let sub = AdiSubscription::Subscribe {
            request_id: Uuid::nil(),
            service: "tasks".to_string(),
            event: "status_changed".to_string(),
            filter: Some(serde_json::json!({"project_id": "proj-1"})),
        };

        let json = serde_json::to_string(&sub).unwrap();
        assert!(json.contains("\"type\":\"subscribe\""));
        assert!(json.contains("\"service\":\"tasks\""));

        let deserialized: AdiSubscription = serde_json::from_str(&json).unwrap();
        match deserialized {
            AdiSubscription::Subscribe {
                service,
                event,
                filter,
                ..
            } => {
                assert_eq!(service, "tasks");
                assert_eq!(event, "status_changed");
                assert!(filter.is_some());
            }
            _ => panic!("Wrong subscription type"),
        }
    }

    #[test]
    fn test_adi_subscription_event() {
        let event = AdiSubscription::Event {
            subscription_id: Uuid::nil(),
            service: "tasks".to_string(),
            event: "status_changed".to_string(),
            data: serde_json::json!({"task_id": "123", "status": "completed"}),
            seq: 42,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"event\""));
        assert!(json.contains("\"seq\":42"));

        let deserialized: AdiSubscription = serde_json::from_str(&json).unwrap();
        match deserialized {
            AdiSubscription::Event { seq, data, .. } => {
                assert_eq!(seq, 42);
                assert_eq!(data["status"], "completed");
            }
            _ => panic!("Wrong subscription type"),
        }
    }

    #[test]
    fn test_adi_service_capabilities() {
        let caps = AdiServiceCapabilities {
            subscriptions: true,
            notifications: true,
            streaming: true,
        };

        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("\"subscriptions\":true"));
        assert!(json.contains("\"notifications\":true"));
        assert!(json.contains("\"streaming\":true"));

        // Test default
        let default_caps = AdiServiceCapabilities::default();
        assert!(!default_caps.subscriptions);
        assert!(!default_caps.notifications);
        assert!(default_caps.streaming); // streaming defaults to true
    }

    #[test]
    fn test_adi_method_info_with_schemas() {
        let method = AdiMethodInfo {
            name: "create".to_string(),
            description: "Create a new task".to_string(),
            streaming: false,
            params_schema: Some(serde_json::json!({
                "type": "object",
                "required": ["title"],
                "properties": {
                    "title": { "type": "string" },
                    "description": { "type": "string" }
                }
            })),
            result_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "created_at": { "type": "string", "format": "date-time" }
                }
            })),
            deprecated: false,
            deprecation_message: None,
        };

        let json = serde_json::to_string(&method).unwrap();
        assert!(json.contains("\"params_schema\""));
        assert!(json.contains("\"result_schema\""));
        assert!(!json.contains("\"deprecated\":true")); // false is default, may be omitted

        let deserialized: AdiMethodInfo = serde_json::from_str(&json).unwrap();
        assert!(deserialized.params_schema.is_some());
        assert!(deserialized.result_schema.is_some());
    }

    #[test]
    fn test_adi_method_deprecated() {
        let method = AdiMethodInfo {
            name: "old_list".to_string(),
            description: "List tasks (deprecated)".to_string(),
            streaming: false,
            params_schema: None,
            result_schema: None,
            deprecated: true,
            deprecation_message: Some("Use 'list' method instead".to_string()),
        };

        let json = serde_json::to_string(&method).unwrap();
        assert!(json.contains("\"deprecated\":true"));
        assert!(json.contains("\"deprecation_message\""));

        let deserialized: AdiMethodInfo = serde_json::from_str(&json).unwrap();
        assert!(deserialized.deprecated);
        assert_eq!(
            deserialized.deprecation_message,
            Some("Use 'list' method instead".to_string())
        );
    }
}
