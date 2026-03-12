//! Sync protocol messages
//!
//! Core message types for the Tarminal synchronization protocol.
//! All messages are JSON-serializable for cross-platform compatibility.

use crate::{DeviceId, SyncMetadata};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

/// Messages exchanged between peers during synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    /// Initial handshake with device info
    Hello {
        device_id: DeviceId,
        display_name: String,
        app_version: String,
        protocol_version: u32,
    },

    /// Request full state sync
    RequestFullSync,

    /// Full state response
    FullState { state: AppState },

    /// Incremental workspace update
    WorkspaceUpdate { workspace: SyncableWorkspace },

    /// Incremental session update
    SessionUpdate { session: SyncableSession },

    /// Incremental command block update
    CommandBlockUpdate { block: SyncableCommandBlock },

    /// Delete notification (tombstone)
    Delete {
        entity_type: EntityType,
        entity_id: Uuid,
        deleted_by: DeviceId,
        deleted_at: chrono::DateTime<chrono::Utc>,
    },

    /// Acknowledgment
    Ack { message_id: Uuid },

    /// Ping for keepalive
    Ping,

    /// Pong response
    Pong,
}

/// Entity types for delete operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Workspace,
    Session,
    CommandBlock,
}

/// Complete application state for full sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub workspaces: Vec<SyncableWorkspace>,
    pub sessions: Vec<SyncableSession>,
    pub command_blocks: Vec<SyncableCommandBlock>,
}

/// Syncable workspace entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncableWorkspace {
    pub id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub session_ids: Vec<Uuid>,
    pub active_session_id: Option<Uuid>,
    pub sync_metadata: SyncMetadata,
}

/// Syncable session entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncableSession {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub title: String,
    pub command_block_ids: Vec<Uuid>,
    pub current_directory: String,
    pub session_type: SessionType,
    pub sync_metadata: SyncMetadata,
}

/// Session type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    /// Block-based terminal (command + output blocks)
    BlockBased,
    /// Full PTY terminal (interactive shell)
    Interactive,
}

/// Syncable command block entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncableCommandBlock {
    pub id: Uuid,
    pub session_id: Uuid,
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_metadata: SyncMetadata,
}

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
    AccessDenied {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        auth_kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        auth_domain: Option<String>,
    },

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
    /// Unique identifier (e.g., "linux", "linux-cuda", "macos")
    pub id: String,
    /// Docker image to use
    pub image: String,
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
    fn test_sync_message_serialization() {
        let msg = SyncMessage::Hello {
            device_id: Uuid::new_v4(),
            display_name: "Test Device".to_string(),
            app_version: "1.0".to_string(),
            protocol_version: 1,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: SyncMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            SyncMessage::Hello { display_name, .. } => {
                assert_eq!(display_name, "Test Device");
            }
            _ => panic!("Wrong message type"),
        }
    }

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
    fn test_register_with_setup_token_serialization() {
        let msg = SignalingMessage::RegisterWithSetupToken {
            secret: "test-secret-with-at-least-32-chars-for-validation".to_string(),
            setup_token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test".to_string(),
            name: Some("production-api".to_string()),
            version: "0.2.1".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("register_with_setup_token"));
        assert!(json.contains("setup_token"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::RegisterWithSetupToken {
                secret,
                setup_token,
                name,
                version,
            } => {
                assert_eq!(secret, "test-secret-with-at-least-32-chars-for-validation");
                assert!(setup_token.starts_with("eyJ"));
                assert_eq!(name, Some("production-api".to_string()));
                assert_eq!(version, "0.2.1");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_deregister_serialization() {
        let msg = SignalingMessage::Deregister {
            device_id: "device-123".to_string(),
            reason: Some("shutdown".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("deregister"));
        assert!(json.contains("device_id"));
        assert!(json.contains("shutdown"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::Deregister { device_id, reason } => {
                assert_eq!(device_id, "device-123");
                assert_eq!(reason, Some("shutdown".to_string()));
            }
            _ => panic!("Wrong message type"),
        }

        // Test without reason
        let msg_no_reason = SignalingMessage::Deregister {
            device_id: "device-456".to_string(),
            reason: None,
        };
        let json_no_reason = serde_json::to_string(&msg_no_reason).unwrap();
        assert!(!json_no_reason.contains("reason")); // Optional field should be skipped
    }

    #[test]
    fn test_deregistered_serialization() {
        let msg = SignalingMessage::Deregistered {
            device_id: "device-123".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("deregistered"));
        assert!(json.contains("device_id"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::Deregistered { device_id } => {
                assert_eq!(device_id, "device-123");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_remove_cocoon_serialization() {
        let msg = SignalingMessage::RemoveCocoon {
            device_id: "device-123".to_string(),
            access_token: "token-abc".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("remove_cocoon"));
        assert!(json.contains("device_id"));
        assert!(json.contains("access_token"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::RemoveCocoon {
                device_id,
                access_token,
            } => {
                assert_eq!(device_id, "device-123");
                assert_eq!(access_token, "token-abc");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_parse_remove_cocoon_empty_token() {
        let json = r#"{"type":"remove_cocoon","device_id":"dc3de77a7d6dc68d465dfa4f85cc22335f8cc5dc5b8406cf2b1eae64e51561af","access_token":""}"#;
        let result: Result<SignalingMessage, _> = serde_json::from_str(json);
        println!("Result: {:?}", result);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_cocoon_removed_serialization() {
        let msg = SignalingMessage::CocoonRemoved {
            device_id: "device-123".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("cocoon_removed"));
        assert!(json.contains("device_id"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::CocoonRemoved { device_id } => {
                assert_eq!(device_id, "device-123");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_capability_serialization() {
        let cap = Capability {
            protocol: "tasks".to_string(),
            version: "1.0.0".to_string(),
        };

        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.protocol, "tasks");
        assert_eq!(deserialized.version, "1.0.0");
    }

    #[test]
    fn test_service_info_serialization() {
        let service = ServiceInfo {
            name: "tasks-api".to_string(),
            service_type: ServiceType::Http,
            local_port: 8080,
            health_endpoint: Some("/health".to_string()),
        };

        let json = serde_json::to_string(&service).unwrap();
        let deserialized: ServiceInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "tasks-api");
        assert_eq!(deserialized.service_type, ServiceType::Http);
        assert_eq!(deserialized.local_port, 8080);
        assert_eq!(deserialized.health_endpoint, Some("/health".to_string()));
    }

    #[test]
    fn test_capability_request_serialization() {
        let mut payload = serde_json::Map::new();
        payload.insert("text".to_string(), JsonValue::String("hello".to_string()));

        let msg = SignalingMessage::CapabilityRequest {
            request_id: "req-123".to_string(),
            capability: Capability {
                protocol: "embeddings".to_string(),
                version: "1.0.0".to_string(),
            },
            payload: JsonValue::Object(payload),
            prefer_device: Some("device-456".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("capability_request"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::CapabilityRequest {
                request_id,
                capability,
                payload,
                prefer_device,
            } => {
                assert_eq!(request_id, "req-123");
                assert_eq!(capability.protocol, "embeddings");
                assert_eq!(capability.version, "1.0.0");
                assert_eq!(payload.get("text").unwrap().as_str().unwrap(), "hello");
                assert_eq!(prefer_device, Some("device-456".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_cocoon_info_with_capabilities() {
        let info = CocoonInfo {
            device_id: "dev-123".to_string(),
            status: "online".to_string(),
            claimed_at: "2024-01-01T00:00:00Z".to_string(),
            services: vec![ServiceInfo {
                name: "api".to_string(),
                service_type: ServiceType::Http,
                local_port: 3000,
                health_endpoint: None,
            }],
            capabilities: vec![
                Capability {
                    protocol: "tasks".to_string(),
                    version: "1.0.0".to_string(),
                },
                Capability {
                    protocol: "knowledgebase".to_string(),
                    version: "2.3.1".to_string(),
                },
            ],
            location: Some("us-west".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: CocoonInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.device_id, "dev-123");
        assert_eq!(deserialized.services.len(), 1);
        assert_eq!(deserialized.capabilities.len(), 2);
        assert_eq!(deserialized.location, Some("us-west".to_string()));
    }

    #[test]
    fn test_silk_request_create_session() {
        let mut env = HashMap::new();
        env.insert("FOO".to_string(), "bar".to_string());

        let req = SilkRequest::CreateSession {
            cwd: Some("/home/user".to_string()),
            env,
            shell: Some("/bin/zsh".to_string()),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("create_session"));

        let deserialized: SilkRequest = serde_json::from_str(&json).unwrap();
        match deserialized {
            SilkRequest::CreateSession { cwd, env, shell } => {
                assert_eq!(cwd, Some("/home/user".to_string()));
                assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
                assert_eq!(shell, Some("/bin/zsh".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_silk_request_execute() {
        let session_id = Uuid::new_v4();
        let command_id = Uuid::new_v4();

        let req = SilkRequest::Execute {
            session_id,
            command: "ls -la".to_string(),
            command_id,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("execute"));
        assert!(json.contains("ls -la"));

        let deserialized: SilkRequest = serde_json::from_str(&json).unwrap();
        match deserialized {
            SilkRequest::Execute {
                session_id: sid,
                command,
                command_id: cid,
            } => {
                assert_eq!(sid, session_id);
                assert_eq!(command, "ls -la");
                assert_eq!(cid, command_id);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_silk_response_output() {
        let session_id = Uuid::new_v4();
        let command_id = Uuid::new_v4();

        let mut styles = HashMap::new();
        styles.insert("color".to_string(), "#00ff00".to_string());

        let resp = SilkResponse::Output {
            session_id,
            command_id,
            stream: SilkStream::Stdout,
            data: "hello world".to_string(),
            html: Some(vec![SilkHtmlSpan {
                text: "hello".to_string(),
                classes: vec!["bold".to_string()],
                styles,
            }]),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("output"));
        assert!(json.contains("stdout"));

        let deserialized: SilkResponse = serde_json::from_str(&json).unwrap();
        match deserialized {
            SilkResponse::Output {
                stream, data, html, ..
            } => {
                assert_eq!(stream, SilkStream::Stdout);
                assert_eq!(data, "hello world");
                assert!(html.is_some());
                let spans = html.unwrap();
                assert_eq!(spans[0].text, "hello");
                assert!(spans[0].classes.contains(&"bold".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_silk_response_interactive_required() {
        let session_id = Uuid::new_v4();
        let command_id = Uuid::new_v4();
        let pty_session_id = Uuid::new_v4();

        let resp = SilkResponse::InteractiveRequired {
            session_id,
            command_id,
            reason: "Command requires TTY".to_string(),
            pty_session_id,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("interactive_required"));
        assert!(json.contains("pty_session_id"));

        let deserialized: SilkResponse = serde_json::from_str(&json).unwrap();
        match deserialized {
            SilkResponse::InteractiveRequired {
                reason,
                pty_session_id: pid,
                ..
            } => {
                assert_eq!(reason, "Command requires TTY");
                assert_eq!(pid, pty_session_id);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_register_hive_serialization() {
        let msg = SignalingMessage::RegisterHive {
            hive_id: "hive-001".to_string(),
            version: "0.1.0".to_string(),
            cocoon_kinds: vec![
                CocoonKind {
                    id: "linux".to_string(),
                    image: "registry.the-ihor.com/cocoon:latest".to_string(),
                },
                CocoonKind {
                    id: "linux-cuda".to_string(),
                    image: "registry.the-ihor.com/cocoon:cuda".to_string(),
                },
            ],
            // Example HMAC-SHA256 signature (hex-encoded)
            hive_id_signature: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
                .to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("register_hive"));
        assert!(json.contains("cocoon_kinds"));
        assert!(json.contains("hive_id_signature"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::RegisterHive {
                hive_id,
                version,
                cocoon_kinds,
                hive_id_signature,
            } => {
                assert_eq!(hive_id, "hive-001");
                assert_eq!(version, "0.1.0");
                assert_eq!(cocoon_kinds.len(), 2);
                assert_eq!(cocoon_kinds[0].id, "linux");
                assert_eq!(hive_id_signature.len(), 64); // SHA256 hex = 64 chars
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_spawn_cocoon_serialization() {
        let msg = SignalingMessage::SpawnCocoon {
            request_id: "req-123".to_string(),
            setup_token: "eyJhbGciOiJIUzI1NiJ9.test.sig".to_string(),
            name: Some("my-cocoon".to_string()),
            kind: "linux-cuda".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("spawn_cocoon"));
        assert!(json.contains("setup_token"));
        assert!(json.contains("linux-cuda"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::SpawnCocoon {
                request_id,
                setup_token,
                name,
                kind,
            } => {
                assert_eq!(request_id, "req-123");
                assert!(setup_token.starts_with("eyJ"));
                assert_eq!(name, Some("my-cocoon".to_string()));
                assert_eq!(kind, "linux-cuda");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_spawn_cocoon_result_success() {
        let msg = SignalingMessage::SpawnCocoonResult {
            request_id: "req-123".to_string(),
            success: true,
            device_id: Some("device-abc".to_string()),
            container_id: Some("container-xyz".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("spawn_cocoon_result"));
        assert!(!json.contains("error")); // None should be skipped

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::SpawnCocoonResult {
                request_id,
                success,
                device_id,
                container_id,
                error,
            } => {
                assert_eq!(request_id, "req-123");
                assert!(success);
                assert_eq!(device_id, Some("device-abc".to_string()));
                assert_eq!(container_id, Some("container-xyz".to_string()));
                assert!(error.is_none());
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_spawn_cocoon_result_failure() {
        let msg = SignalingMessage::SpawnCocoonResult {
            request_id: "req-456".to_string(),
            success: false,
            device_id: None,
            container_id: None,
            error: Some("Docker unavailable".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Docker unavailable"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::SpawnCocoonResult { success, error, .. } => {
                assert!(!success);
                assert_eq!(error, Some("Docker unavailable".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_terminate_cocoon_serialization() {
        let msg = SignalingMessage::TerminateCocoon {
            request_id: "req-789".to_string(),
            container_id: "cocoon-abc123".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("terminate_cocoon"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::TerminateCocoon {
                request_id,
                container_id,
            } => {
                assert_eq!(request_id, "req-789");
                assert_eq!(container_id, "cocoon-abc123");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_list_hives_serialization() {
        let msg = SignalingMessage::ListHives {
            access_token: "token-abc".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("list_hives"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::ListHives { access_token } => {
                assert_eq!(access_token, "token-abc");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_hives_list_serialization() {
        let msg = SignalingMessage::HivesList {
            hives: vec![
                HiveInfo {
                    hive_id: "hive-001".to_string(),
                    version: "0.1.0".to_string(),
                    status: "online".to_string(),
                    connected_at: "2024-01-01T00:00:00Z".to_string(),
                    cocoon_kinds: vec![CocoonKind {
                        id: "linux".to_string(),
                        image: "registry.the-ihor.com/cocoon:latest".to_string(),
                    }],
                },
                HiveInfo {
                    hive_id: "hive-002".to_string(),
                    version: "0.1.1".to_string(),
                    status: "online".to_string(),
                    connected_at: "2024-01-02T00:00:00Z".to_string(),
                    cocoon_kinds: vec![
                        CocoonKind {
                            id: "linux".to_string(),
                            image: "registry.the-ihor.com/cocoon:latest".to_string(),
                        },
                        CocoonKind {
                            id: "linux-cuda".to_string(),
                            image: "registry.the-ihor.com/cocoon:cuda".to_string(),
                        },
                    ],
                },
            ],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("hives_list"));
        assert!(json.contains("hive-001"));
        assert!(json.contains("hive-002"));
        assert!(json.contains("cocoon_kinds"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::HivesList { hives } => {
                assert_eq!(hives.len(), 2);
                assert_eq!(hives[0].hive_id, "hive-001");
                assert_eq!(hives[0].cocoon_kinds.len(), 1);
                assert_eq!(hives[1].version, "0.1.1");
                assert_eq!(hives[1].cocoon_kinds.len(), 2);
            }
            _ => panic!("Wrong message type"),
        }
    }

    // ========== Browser Debug Tests ==========

    #[test]
    fn test_browser_debug_tab_available() {
        let msg = SignalingMessage::BrowserDebugTabAvailable {
            token: "eyJ0eXBlIjoiZGVidWciLCJjIjoiY29jb29uLTEyMyJ9".to_string(),
            browser_id: "browser-abc-123".to_string(),
            url: "https://example.com/app".to_string(),
            title: "My App".to_string(),
            favicon: Some("https://example.com/favicon.ico".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("browser_debug_tab_available"));
        assert!(json.contains("browser_id"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::BrowserDebugTabAvailable {
                token,
                browser_id,
                url,
                title,
                favicon,
            } => {
                assert!(token.starts_with("eyJ"));
                assert_eq!(browser_id, "browser-abc-123");
                assert_eq!(url, "https://example.com/app");
                assert_eq!(title, "My App");
                assert!(favicon.is_some());
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_browser_debug_network_event() {
        let msg = SignalingMessage::BrowserDebugNetworkEvent {
            token: "test-token".to_string(),
            event: NetworkEventType::Request,
            data: NetworkEventData {
                request_id: "req-123".to_string(),
                timestamp: 1234567890,
                method: Some("POST".to_string()),
                url: Some("https://api.example.com/data".to_string()),
                request_headers: Some(HashMap::from([(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )])),
                request_body: Some(r#"{"key":"value"}"#.to_string()),
                status: None,
                status_text: None,
                response_headers: None,
                mime_type: None,
                response_body: None,
                response_body_truncated: None,
                duration_ms: None,
                error: None,
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("browser_debug_network_event"));
        assert!(json.contains("request"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::BrowserDebugNetworkEvent { event, data, .. } => {
                assert_eq!(event, NetworkEventType::Request);
                assert_eq!(data.method, Some("POST".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_browser_debug_console_event() {
        let msg = SignalingMessage::BrowserDebugConsoleEvent {
            token: "test-token".to_string(),
            entry: ConsoleEntry {
                timestamp: 1234567890,
                level: ConsoleLevel::Error,
                message: "Uncaught TypeError: undefined is not a function".to_string(),
                args: vec![JsonValue::String("error details".to_string())],
                source: Some("app.js".to_string()),
                line: Some(42),
                column: Some(10),
                stack_trace: Some("Error: ...\n  at foo (app.js:42:10)".to_string()),
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("browser_debug_console_event"));
        assert!(json.contains("error"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::BrowserDebugConsoleEvent { entry, .. } => {
                assert_eq!(entry.level, ConsoleLevel::Error);
                assert_eq!(entry.line, Some(42));
                assert!(entry.stack_trace.is_some());
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_browser_debug_list_tabs() {
        let msg = SignalingMessage::BrowserDebugListTabs {
            access_token: "jwt-token".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("browser_debug_list_tabs"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::BrowserDebugListTabs { access_token } => {
                assert_eq!(access_token, "jwt-token");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_browser_debug_tabs_response() {
        let msg = SignalingMessage::BrowserDebugTabs {
            tabs: vec![
                BrowserDebugTab {
                    token: "token-1".to_string(),
                    browser_id: "browser-1".to_string(),
                    url: "https://app.example.com".to_string(),
                    title: "App Page".to_string(),
                    cocoon_id: "cocoon-123".to_string(),
                    cocoon_name: Some("dev-server".to_string()),
                    favicon: None,
                },
                BrowserDebugTab {
                    token: "token-2".to_string(),
                    browser_id: "browser-1".to_string(),
                    url: "https://app.example.com/other".to_string(),
                    title: "Other Page".to_string(),
                    cocoon_id: "cocoon-123".to_string(),
                    cocoon_name: Some("dev-server".to_string()),
                    favicon: Some("https://app.example.com/favicon.ico".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("browser_debug_tabs"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::BrowserDebugTabs { tabs } => {
                assert_eq!(tabs.len(), 2);
                assert_eq!(tabs[0].cocoon_id, "cocoon-123");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_browser_debug_get_network_with_filters() {
        let msg = SignalingMessage::BrowserDebugGetNetwork {
            request_id: "req-123".to_string(),
            token: "debug-token".to_string(),
            filters: Some(NetworkFilters {
                url_pattern: Some("api".to_string()),
                method: Some(vec!["POST".to_string(), "PUT".to_string()]),
                status_min: Some(400),
                status_max: Some(599),
                since: Some(1234567890),
                limit: Some(100),
            }),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("browser_debug_get_network"));
        assert!(json.contains("filters"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::BrowserDebugGetNetwork {
                request_id,
                filters,
                ..
            } => {
                assert_eq!(request_id, "req-123");
                let f = filters.unwrap();
                assert_eq!(f.url_pattern, Some("api".to_string()));
                assert_eq!(f.status_min, Some(400));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_browser_debug_network_data() {
        let msg = SignalingMessage::BrowserDebugNetworkData {
            request_id: "req-123".to_string(),
            requests: vec![NetworkRequest {
                request_id: "net-1".to_string(),
                timestamp: 1234567890,
                method: "GET".to_string(),
                url: "https://api.example.com/users".to_string(),
                request_headers: None,
                request_body: None,
                status: Some(200),
                status_text: Some("OK".to_string()),
                response_headers: Some(HashMap::from([(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )])),
                response_body: Some(r#"[{"id":1}]"#.to_string()),
                response_body_truncated: Some(false),
                mime_type: Some("application/json".to_string()),
                duration_ms: Some(150),
                error: None,
            }],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("browser_debug_network_data"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::BrowserDebugNetworkData {
                request_id,
                requests,
            } => {
                assert_eq!(request_id, "req-123");
                assert_eq!(requests.len(), 1);
                assert_eq!(requests[0].status, Some(200));
                assert_eq!(requests[0].duration_ms, Some(150));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_browser_debug_get_console_with_filters() {
        let msg = SignalingMessage::BrowserDebugGetConsole {
            request_id: "req-456".to_string(),
            token: "debug-token".to_string(),
            filters: Some(ConsoleFilters {
                level: Some(vec![ConsoleLevel::Error, ConsoleLevel::Warn]),
                message_pattern: Some("TypeError".to_string()),
                since: None,
                limit: Some(50),
            }),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("browser_debug_get_console"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::BrowserDebugGetConsole { filters, .. } => {
                let f = filters.unwrap();
                assert!(f.level.unwrap().contains(&ConsoleLevel::Error));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_browser_debug_console_data() {
        let msg = SignalingMessage::BrowserDebugConsoleData {
            request_id: "req-456".to_string(),
            entries: vec![
                ConsoleEntry {
                    timestamp: 1234567890,
                    level: ConsoleLevel::Log,
                    message: "App initialized".to_string(),
                    args: vec![],
                    source: None,
                    line: None,
                    column: None,
                    stack_trace: None,
                },
                ConsoleEntry {
                    timestamp: 1234567891,
                    level: ConsoleLevel::Error,
                    message: "Failed to load data".to_string(),
                    args: vec![JsonValue::String("Network error".to_string())],
                    source: Some("data-loader.js".to_string()),
                    line: Some(100),
                    column: Some(5),
                    stack_trace: None,
                },
            ],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("browser_debug_console_data"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::BrowserDebugConsoleData {
                request_id,
                entries,
            } => {
                assert_eq!(request_id, "req-456");
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].level, ConsoleLevel::Log);
                assert_eq!(entries[1].level, ConsoleLevel::Error);
            }
            _ => panic!("Wrong message type"),
        }
    }

    // ========== SSL Certificate Tests ==========

    #[test]
    fn test_request_certificate_serialization() {
        let msg = SignalingMessage::RequestCertificate {
            request_id: "req-cert-123".to_string(),
            domains: vec!["example.com".to_string(), "www.example.com".to_string()],
            email: "admin@example.com".to_string(),
            staging: true,
            challenge_type: Some("http01".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("request_certificate"));
        assert!(json.contains("example.com"));
        assert!(json.contains("staging"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::RequestCertificate {
                request_id,
                domains,
                email,
                staging,
                challenge_type,
            } => {
                assert_eq!(request_id, "req-cert-123");
                assert_eq!(domains.len(), 2);
                assert_eq!(email, "admin@example.com");
                assert!(staging);
                assert_eq!(challenge_type, Some("http01".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_certificate_issued_success() {
        let msg = SignalingMessage::CertificateIssued {
            request_id: "req-cert-123".to_string(),
            success: true,
            domain: Some("example.com".to_string()),
            expires_at: Some("2026-04-18T00:00:00Z".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("certificate_issued"));
        assert!(!json.contains("error")); // None should be skipped

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::CertificateIssued {
                request_id,
                success,
                domain,
                expires_at,
                error,
            } => {
                assert_eq!(request_id, "req-cert-123");
                assert!(success);
                assert_eq!(domain, Some("example.com".to_string()));
                assert!(expires_at.is_some());
                assert!(error.is_none());
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_certificate_issued_failure() {
        let msg = SignalingMessage::CertificateIssued {
            request_id: "req-cert-456".to_string(),
            success: false,
            domain: None,
            expires_at: None,
            error: Some("DNS validation failed".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("DNS validation failed"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::CertificateIssued { success, error, .. } => {
                assert!(!success);
                assert_eq!(error, Some("DNS validation failed".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_get_certificate_status() {
        let msg = SignalingMessage::GetCertificateStatus {
            request_id: "req-status-123".to_string(),
            domains: vec!["example.com".to_string()],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("get_certificate_status"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::GetCertificateStatus {
                request_id,
                domains,
            } => {
                assert_eq!(request_id, "req-status-123");
                assert_eq!(domains, vec!["example.com".to_string()]);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_certificate_status_response() {
        let msg = SignalingMessage::CertificateStatus {
            request_id: "req-status-123".to_string(),
            certificates: vec![CertificateInfo {
                domain: "example.com".to_string(),
                domains: vec!["example.com".to_string(), "www.example.com".to_string()],
                expires_at: "2026-04-18T00:00:00Z".to_string(),
                days_until_expiry: 90,
                needs_renewal: false,
                issuer: "Let's Encrypt".to_string(),
            }],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("certificate_status"));
        assert!(json.contains("Let's Encrypt"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::CertificateStatus {
                request_id,
                certificates,
            } => {
                assert_eq!(request_id, "req-status-123");
                assert_eq!(certificates.len(), 1);
                assert_eq!(certificates[0].domain, "example.com");
                assert_eq!(certificates[0].days_until_expiry, 90);
                assert!(!certificates[0].needs_renewal);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_certificate_info_serialization() {
        let info = CertificateInfo {
            domain: "api.example.com".to_string(),
            domains: vec!["api.example.com".to_string()],
            expires_at: "2026-01-01T00:00:00Z".to_string(),
            days_until_expiry: 15,
            needs_renewal: true,
            issuer: "Let's Encrypt".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: CertificateInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.domain, "api.example.com");
        assert_eq!(deserialized.days_until_expiry, 15);
        assert!(deserialized.needs_renewal);
    }

    // ========== WebRTC Session Tests ==========

    #[test]
    fn test_webrtc_start_session() {
        let msg = SignalingMessage::WebRtcStartSession {
            session_id: "rtc-session-123".to_string(),
            device_id: "cocoon-abc".to_string(),
            access_token: "jwt-token".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("web_rtc_start_session"));
        assert!(json.contains("rtc-session-123"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcStartSession {
                session_id,
                device_id,
                access_token,
            } => {
                assert_eq!(session_id, "rtc-session-123");
                assert_eq!(device_id, "cocoon-abc");
                assert_eq!(access_token, "jwt-token");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_session_started() {
        let msg = SignalingMessage::WebRtcSessionStarted {
            session_id: "rtc-session-123".to_string(),
            device_id: "cocoon-abc".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("web_rtc_session_started"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcSessionStarted {
                session_id,
                device_id,
            } => {
                assert_eq!(session_id, "rtc-session-123");
                assert_eq!(device_id, "cocoon-abc");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_offer() {
        let sdp_offer = "v=0\r\no=- 123456 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n...";
        let msg = SignalingMessage::WebRtcOffer {
            session_id: "rtc-session-123".to_string(),
            sdp: sdp_offer.to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("web_rtc_offer"));
        assert!(json.contains("sdp"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcOffer { session_id, sdp } => {
                assert_eq!(session_id, "rtc-session-123");
                assert!(sdp.contains("v=0"));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_answer() {
        let sdp_answer = "v=0\r\no=- 654321 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n...";
        let msg = SignalingMessage::WebRtcAnswer {
            session_id: "rtc-session-123".to_string(),
            sdp: sdp_answer.to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("web_rtc_answer"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcAnswer { session_id, sdp } => {
                assert_eq!(session_id, "rtc-session-123");
                assert!(sdp.contains("v=0"));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_ice_candidate() {
        let msg = SignalingMessage::WebRtcIceCandidate {
            session_id: "rtc-session-123".to_string(),
            candidate: "candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host".to_string(),
            sdp_mid: Some("0".to_string()),
            sdp_mline_index: Some(0),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("web_rtc_ice_candidate"));
        assert!(json.contains("candidate"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcIceCandidate {
                session_id,
                candidate,
                sdp_mid,
                sdp_mline_index,
            } => {
                assert_eq!(session_id, "rtc-session-123");
                assert!(candidate.contains("UDP"));
                assert_eq!(sdp_mid, Some("0".to_string()));
                assert_eq!(sdp_mline_index, Some(0));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_ice_candidate_minimal() {
        let msg = SignalingMessage::WebRtcIceCandidate {
            session_id: "rtc-session-123".to_string(),
            candidate: "candidate:1 1 UDP 2130706431 192.168.1.1 54321 typ host".to_string(),
            sdp_mid: None,
            sdp_mline_index: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("sdp_mid")); // None should be skipped
        assert!(!json.contains("sdp_mline_index")); // None should be skipped

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcIceCandidate {
                sdp_mid,
                sdp_mline_index,
                ..
            } => {
                assert!(sdp_mid.is_none());
                assert!(sdp_mline_index.is_none());
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_session_ended() {
        let msg = SignalingMessage::WebRtcSessionEnded {
            session_id: "rtc-session-123".to_string(),
            reason: Some("user_disconnected".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("web_rtc_session_ended"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcSessionEnded { session_id, reason } => {
                assert_eq!(session_id, "rtc-session-123");
                assert_eq!(reason, Some("user_disconnected".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_error() {
        let msg = SignalingMessage::WebRtcError {
            session_id: "rtc-session-123".to_string(),
            code: "ice_failed".to_string(),
            message: "ICE connection failed after timeout".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("web_rtc_error"));
        assert!(json.contains("ice_failed"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcError {
                session_id,
                code,
                message,
            } => {
                assert_eq!(session_id, "rtc-session-123");
                assert_eq!(code, "ice_failed");
                assert!(message.contains("ICE connection failed"));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_data() {
        let msg = SignalingMessage::WebRtcData {
            session_id: "rtc-session-123".to_string(),
            channel: "terminal".to_string(),
            data: r#"{"type":"input","data":"ls -la\n"}"#.to_string(),
            binary: false,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("web_rtc_data"));
        assert!(json.contains("terminal"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcData {
                session_id,
                channel,
                data,
                binary,
            } => {
                assert_eq!(session_id, "rtc-session-123");
                assert_eq!(channel, "terminal");
                assert!(data.contains("ls -la"));
                assert!(!binary);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_data_binary() {
        let msg = SignalingMessage::WebRtcData {
            session_id: "rtc-session-123".to_string(),
            channel: "file-transfer".to_string(),
            data: "SGVsbG8gV29ybGQh".to_string(), // base64 "Hello World!"
            binary: true,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"binary\":true"));

        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            SignalingMessage::WebRtcData { binary, .. } => {
                assert!(binary);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_webrtc_session_info() {
        let info = WebRtcSessionInfo {
            session_id: "rtc-session-123".to_string(),
            client_id: "user-456".to_string(),
            cocoon_id: "cocoon-abc".to_string(),
            state: "connected".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            ice_state: Some("connected".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: WebRtcSessionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.session_id, "rtc-session-123");
        assert_eq!(deserialized.state, "connected");
        assert_eq!(deserialized.ice_state, Some("connected".to_string()));
    }
}
