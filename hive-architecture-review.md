# Hive Architecture Review: Decision Points

**System:** Hive Cocoon Container Orchestration  
**Scope:** crates/hive (core, http, plugin, ssl, plugins)  
**Date:** 2026-01-31

## Summary

- 8 boundary decisions requiring evaluation
- 5 abstraction questions about plugin architecture
- 4 integration points needing ownership clarity
- 3 state management concerns across components

---

## 1. Boundary Decisions

### 1.1: Should SignalingClient and HTTP Proxy be in core or separate modules?

**Evidence:**
- `core/src/signaling_client.rs` - 454 lines, handles WebSocket lifecycle + SSL certificate requests
- `core/src/http_proxy.rs` - 260+ lines, high-performance reverse proxy with connection pooling
- Both have infrastructure concerns (networking, TLS) mixed with business logic

**Current State:**
SignalingClient and HTTP proxy are part of hive-core library, but they're essentially standalone subsystems with minimal coupling to orchestration logic.

**Options to Consider:**
1. Extract to separate crates (`hive-signaling`, `hive-proxy`) - cleaner boundaries, independent versioning
2. Keep in core but separate modules - current state, simpler build
3. Move to `http` crate - consolidate all network I/O in one place

**Questions for Reviewer:**
- Is the signaling protocol stable enough to version independently?
- Do other ADI components need to reuse the proxy functionality?
- Would splitting increase complexity of managing Hive releases?

**Trade-offs:**
- **Extract**: Better separation of concerns, but increases dependency graph complexity (3 crates instead of 1)
- **Keep**: Simpler build/release, but core has multiple unrelated responsibilities
- **Move to http**: Logical grouping, but http crate becomes "networking utilities" instead of "HTTP server binary"

---

### 1.2: Should ServiceManager and SourceManager be merged or kept separate?

**Evidence:**
- `core/src/service_manager/mod.rs` - 1183 lines, manages service lifecycle for single hive.yaml
- `core/src/source_manager.rs` - 390+ lines, manages multiple hive.yaml sources
- Both manage services, but at different granularities (single vs multi-source)
- ServiceManager is embedded inside SourceManager (composition pattern)

**Current State:**
ServiceManager operates on a single HiveConfig, SourceManager orchestrates multiple ServiceManagers across different sources. Clear separation of concerns.

**Coupling Evidence:**
```rust
// SourceManager owns ServiceManagers for each source
pub struct ManagedSource {
    info: SourceInfo,
    manager: ServiceManager,  // Composition
    backend: Option<Arc<SqliteBackend>>,
}
```

**Options to Consider:**
1. **Keep separate** (current) - single-source vs multi-source abstraction
2. **Merge into unified ServiceOrchestrator** - eliminate layer, handle multi-source directly
3. **Extract common interface** - define ServiceOrchestration trait, allow different implementations

**Questions for Reviewer:**
- Is single-source mode (ServiceManager alone) a valid use case?
- Does the two-layer abstraction add clarity or just indirection?
- Will you need alternative SourceManager implementations (e.g., Kubernetes-backed)?

**Trade-offs:**
- **Keep separate**: Clear responsibilities, but duplicates concepts (service state in both layers)
- **Merge**: Simpler mental model, but loses reusability of ServiceManager for single-source scenarios
- **Extract trait**: Most flexible, but adds abstraction cost without clear alternate implementations

---

### 1.3: Should Plugin System and Plugin Manager be reconciled?

**Evidence:**
- `core/src/plugin_system.rs` - Old system with PluginRegistry, PluginInfo, auto-install logic
- `core/src/plugins.rs` - New system with PluginManager, bundled plugins via feature flags
- Both exist simultaneously, unclear which is canonical

**Current Duplication:**
```rust
// Old system (plugin_system.rs)
pub fn plugin_registry() -> &'static PluginRegistry { ... }

// New system (plugins.rs)  
pub fn plugin_manager() -> &'static PluginManager { ... }
```

**Usage Patterns:**
- ServiceManager uses `plugin_registry()` for auto-install checks
- ServiceManager uses `plugin_manager()` to get runner plugins for hook execution
- Both are global singletons but serve overlapping purposes

**Options to Consider:**
1. **Unify into single PluginManager** - merge auto-install logic into new system
2. **Clarify roles** - plugin_system = discovery/install, plugin_manager = execution
3. **Deprecate old system** - complete migration to new plugin ABI architecture

**Questions for Reviewer:**
- Is the auto-install feature (plugin_registry) still needed with bundled plugins?
- Should external plugin loading be prioritized or is bundled-only sufficient?
- Which system represents the long-term architecture vision?

---

### 1.4: Should Daemon, RemoteControl, and HTTP server be in same crate?

**Evidence:**
- `core/src/daemon.rs` - HiveDaemon with Unix socket IPC (843+ lines)
- `core/src/signaling_control.rs` - RemoteControlHandler for WebSocket control (200+ lines)
- `http/src/main.rs` - HTTP server binary
- All three are entry points with different interfaces (CLI, WebSocket, HTTP)

**Current State:**
Daemon and RemoteControl are in core library, HTTP server is separate binary. This allows:
- Core can be embedded in different contexts
- HTTP server is just one consumer of core functionality

**Boundary Question:**
Should control plane (daemon, remote control) be separated from orchestration logic (ServiceManager, etc.)?

**Options to Consider:**
1. **Extract control plane** to `hive-control` crate - daemon + signaling_control + IPC
2. **Keep in core** (current) - simpler dependency graph
3. **Create hive-runtime** - merge core + control plane, rename current "core" to "hive-engine"

**Questions for Reviewer:**
- Do you envision embedding Hive orchestration logic in other applications?
- Is the daemon mode the primary interface or just one option?
- Should WebSocket control be usable without daemon mode?

---

### 1.5: Should Exposure System be part of core or a plugin?

**Evidence:**
- `core/src/exposure.rs` - ExposureManager for cross-source service dependencies
- `core/src/hive_config/types.rs` - ExposeConfig, UsesConfig as first-class YAML concepts
- Feature is optional (only needed for multi-source setups) but deeply integrated

**Current State:**
Exposure system is built into core as a fundamental feature, managed by SourceManager. It handles:
- Service registration with bcrypt-hashed secrets
- Variable interpolation and port resolution
- Health-based dependency readiness

**Options to Consider:**
1. **Keep in core** (current) - exposure is a core orchestration feature
2. **Extract as plugin** - optional feature, loaded on-demand
3. **Make it a SourceManager-level concern** - only relevant for multi-source mode

**Questions for Reviewer:**
- What percentage of Hive deployments use multi-source configurations?
- Is exposure/uses a "nice-to-have" or fundamental to Hive's value proposition?
- Could this feature be implemented entirely via environment variable plugins?

---

## 2. Coupling & Abstraction Issues

### 2.1: Tight Coupling Between ServiceManager and ProxyState

**Evidence:**
```rust
// ServiceManager requires ProxyState even if no proxy is used
pub struct ServiceManager {
    proxy_state: Arc<ServiceProxyState>,
    rollout_manager: Arc<RolloutManager>,
    // ...
}

// RolloutManager also requires ProxyState
pub struct RolloutManager {
    proxy_state: Arc<ServiceProxyState>,
    // ...
}
```

**Why This Needs Decision:**
- ServiceManager always constructs ProxyState from HiveConfig, even if `proxy: null`
- RolloutManager depends on ProxyState for blue-green traffic switching
- This creates implicit dependency: rollouts require proxy infrastructure

**Options to Consider:**
1. **Make ProxyState optional** - `Option<Arc<ServiceProxyState>>`, check at runtime
2. **Create ProxyManager abstraction** - trait for traffic routing, allow null implementation
3. **Accept coupling** - document that proxy is always initialized (low overhead)

**Questions for Reviewer:**
- Is it valid to use blue-green rollouts without HTTP proxy?
- Should ServiceManager work in "orchestration-only" mode (no proxy at all)?
- Is the ProxyState initialization cost acceptable even when unused?

---

### 2.2: Plugin ABI Abstraction - Too Generic or Just Right?

**Evidence:**
- All plugin traits use `serde_json::Value` for configuration
- No compile-time validation of plugin configs
- Type safety deferred to runtime parsing in each plugin

**Example:**
```rust
#[async_trait]
pub trait RunnerPlugin {
    async fn init(&mut self, defaults: &serde_json::Value) -> Result<()>;
    async fn start(
        &self,
        service_name: &str,
        config: &serde_json::Value,  // <-- Untyped
        env: HashMap<String, String>,
        ctx: &RuntimeContext,
    ) -> Result<ProcessHandle>;
}
```

**Why This Needs Decision:**
- **Flexibility**: Plugins can define arbitrary config schemas
- **Type Safety**: No compile-time guarantees, runtime errors possible
- **Discoverability**: Hard to know what config a plugin expects without reading code/docs

**Options to Consider:**
1. **Keep generic** (current) - maximum plugin flexibility, minimal ABI changes
2. **Add schema validation** - plugins expose JSON Schema, validate before init
3. **Use typed configs** - define config structs in lib-plugin-abi-orchestration, versioned breaking changes

**Questions for Reviewer:**
- Is the plugin ecosystem stable enough for schema contracts?
- Would compile-time validation reduce runtime failures significantly?
- Does the flexibility outweigh the type-safety cost?

---

### 2.3: ObservabilityEvent Flooding Risk

**Evidence:**
- `core/src/observability.rs` - EventCollector with broadcast channels
- All logs/metrics/health events go through same channel
- No backpressure mechanism if consumers are slow

**Potential Issue:**
```rust
// EventCollector broadcasts to all subscribers
pub fn emit(&self, event: ObservabilityEvent) {
    let _ = self.sender.send(event);  // Ignores send errors (full channel)
}
```

**Options to Consider:**
1. **Add backpressure** - bounded channels, block on full (may slow down services)
2. **Priority queues** - critical events (health) prioritized over logs
3. **Separate channels** - different channels for logs, metrics, health (current approach is unified)
4. **Accept data loss** - document that slow consumers may miss events

**Questions for Reviewer:**
- Is it acceptable to drop observability events under load?
- Should critical events (service crashes) be guaranteed delivery?
- What is the expected event volume per second per service?

---

### 2.4: RuntimeContext vs HiveConfig RuntimeContext - Name Collision

**Evidence:**
- `lib-plugin-abi-orchestration/src/types.rs` - RuntimeContext for plugin execution
- `hive-core/src/hive_config/interpolation.rs` - RuntimeContext for port interpolation
- Same name, different purposes, imported together

**Example Collision:**
```rust
use crate::hive_config::RuntimeContext;  // Interpolation context
use lib_plugin_abi_orchestration::RuntimeContext;  // Plugin context
// Ambiguity in code, requires explicit paths
```

**Options to Consider:**
1. **Rename one** - e.g., `HiveRuntimeContext` vs `PluginRuntimeContext`
2. **Merge contexts** - unify into single RuntimeContext (may expose internal state to plugins)
3. **Use module-qualified names** - always write `hive_config::RuntimeContext`

**Questions for Reviewer:**
- Are these contexts representing the same concept at different layers?
- Should plugins have access to the full Hive runtime state?
- Is this collision causing real confusion or just aesthetic?

---

### 2.5: Daemon Client IPC - Unix Sockets vs WebSocket Duplication

**Evidence:**
- `core/src/daemon.rs` - DaemonClient using Unix sockets for local control
- `core/src/signaling_control.rs` - RemoteControlHandler using WebSocket for remote control
- Both implement similar request/response patterns but different transports

**Duplication:**
```rust
// DaemonRequest (Unix socket)
pub enum DaemonRequest {
    GetStatus,
    StartService { fqn: String },
    StopService { fqn: String },
    // ...
}

// HiveRequest (WebSocket)
pub enum HiveRequest {
    GetStatus,
    StartService { fqn: String },
    StopService { fqn: String },
    // ... (nearly identical)
}
```

**Options to Consider:**
1. **Unify request/response types** - single enum, transport-agnostic handlers
2. **Keep separate** (current) - allows protocol-specific optimizations
3. **Create abstraction layer** - HiveController trait, different transport implementations

**Questions for Reviewer:**
- Do local (Unix socket) and remote (WebSocket) controls need different capabilities?
- Is the duplication worth the independence, or does it cause sync bugs?
- Should there be a third transport option (gRPC, HTTP REST) in the future?

---

## 3. Integration Points & Ownership

### 3.1: Who Owns Cocoon Lifecycle - Hive or Signaling Server?

**Evidence:**
- Hive receives `SpawnCocoon` messages from signaling server
- Hive creates Docker containers and responds with `SpawnCocoonResult`
- Cocoon connects to signaling server independently (Hive doesn't track connection state)
- Termination can be requested via signaling server, but Hive manages Docker lifecycle

**Current Flow:**
```
Platform API → Signaling Server → Hive → Docker
                     ↓                      ↓
                 Cocoon ← (independent connection) 
```

**Ownership Question:**
- Signaling server knows which cocoons are connected (device_id mapping)
- Hive knows which containers are running (Docker state)
- No single source of truth for "is cocoon ready to use?"

**Options to Consider:**
1. **Hive polls cocoon health** - actively check container + connectivity
2. **Signaling server notifies Hive** - send "CocoonConnected" message back to Hive
3. **Accept eventual consistency** - client retries until cocoon is ready

**Questions for Reviewer:**
- Should Hive be aware of cocoon connectivity beyond container status?
- Who is responsible for cleaning up orphaned containers (running but disconnected)?
- Is the current fire-and-forget model (Hive spawns, forgets) acceptable?

---

### 3.2: SSL Certificate Management - Hive or Dedicated Service?

**Evidence:**
- `ssl/` module in Hive for Let's Encrypt ACME integration
- Certificates can be requested on-demand via WebSocket (`RequestCertificate`)
- Also supports startup-time issuance via environment variables

**Current State:**
SSL is embedded in Hive, tightly coupled to signaling client (receives cert requests via WebSocket).

**Concerns:**
- Certificate issuance is a global infrastructure concern (affects multiple services)
- Embedded in Hive means each Hive instance manages its own certs
- No centralized certificate inventory or renewal coordination

**Options to Consider:**
1. **Keep in Hive** (current) - self-contained, but duplicates effort across Hives
2. **Extract to dedicated cert-manager service** - centralized, but adds dependency
3. **Hybrid approach** - Hive can manage certs locally OR delegate to cert-manager

**Questions for Reviewer:**
- How many Hive instances are typically deployed (single vs fleet)?
- Is per-Hive certificate management acceptable or should it be centralized?
- Should certificate issuance be exposed via HTTP API or only WebSocket?

---

### 3.3: Plugin Discovery - Who Installs Missing Plugins?

**Evidence:**
```rust
// ServiceManager checks if plugin is available
if !is_builtin_plugin(PluginType::Runner, runner_type) {
    let plugin_id = resolve_plugin_id(PluginType::Runner, runner_type);
    plugin_registry().ensure_available(&plugin_id).await?;  // Auto-install
}
```

**Current Behavior:**
- ServiceManager auto-installs missing plugins when starting a service
- This requires network access and write permissions to plugin directory
- No pre-validation (fails at runtime if plugin is unavailable)

**Options to Consider:**
1. **Keep auto-install** (current) - convenient, but unpredictable (network failures)
2. **Validate at config load time** - check plugins before starting services
3. **Require explicit install** - `adi hive plugins install` before `hive start`
4. **Bundle all plugins** - remove external plugin system entirely

**Questions for Reviewer:**
- Should Hive start if required plugins are missing?
- Is auto-install behavior surprising or helpful?
- Should production deployments use bundled plugins only?

---

### 3.4: Observability Event Routing - Push or Pull?

**Evidence:**
- `core/src/observability.rs` - EventCollector broadcasts events
- `core/src/observability_plugins.rs` - ObsPlugins subscribe to events
- Plugins push to destinations (stdout, file, Loki, Prometheus)

**Current Architecture:**
```
ServiceManager → EventCollector → [Broadcast] → ObsPlugins → Destinations
```

**Alternative Pattern (Pull):**
```
Destinations ← Poll ← EventCollector ← ServiceManager
```

**Options to Consider:**
1. **Keep push model** (current) - real-time, low latency, but no backpressure
2. **Add pull API** - HTTP endpoints for scraping events (Prometheus-style)
3. **Hybrid** - push for real-time, pull for historical queries

**Questions for Reviewer:**
- Are observability plugins primarily real-time forwarders or historical stores?
- Should external systems be able to query Hive for metrics/logs?
- Is the push model causing resource issues (memory, CPU)?

---

## 4. State Management & Coordination

### 4.1: ServiceManager In-Memory State vs System State Divergence

**Evidence:**
```rust
// ServiceManager maintains in-memory state
pub struct ServiceRuntime {
    pub state: ServiceState,
    pub process: Option<ProcessHandle>,
    pub healthy: Option<bool>,
    // ...
}
```

**Known Issue:**
```rust
// ServiceManager verifies in-memory state against system state
let in_memory_running = services.get(name)
    .map(|r| r.state == ServiceState::Running)
    .unwrap_or(false);

if in_memory_running {
    // Check actual system state (Docker container, PID file)
    let system_running = self.is_service_running_on_system(name, service_config).await;
    if !system_running {
        warn!("Service {} was marked as running in memory but is not running on system", name);
        // Fix stale state...
    }
}
```

**Why This Happens:**
- Docker containers can exit without notifying Hive
- Processes can be killed (SIGKILL) bypassing cleanup
- Network partitions, restarts, crashes

**Options to Consider:**
1. **Accept eventual consistency** - reconcile on-demand, current approach
2. **Active monitoring** - poll system state periodically, update in-memory cache
3. **Event-driven reconciliation** - Docker event stream, process monitor signals
4. **Single source of truth** - always query system state, no in-memory cache

**Questions for Reviewer:**
- How critical is real-time accuracy of service state?
- Is the reconciliation-on-access pattern causing user confusion?
- Should `adi hive status` show cached state or always query Docker/processes?

**Trade-offs:**
- **Eventual consistency**: Fast reads, occasional staleness
- **Active monitoring**: Accurate, but adds CPU/memory overhead
- **Event-driven**: Real-time, but complex (Docker event stream, signal handling)
- **No cache**: Always accurate, but slow (Docker API call per status check)

---

### 4.2: Blue-Green Deployment State - Who Owns Active/Inactive Tracking?

**Evidence:**
```rust
// RolloutManager tracks blue-green state
pub struct BlueGreenState {
    active_color: BlueGreenColor,
    blue_port: u16,
    green_port: u16,
    new_instance_started_at: Option<Instant>,
}

// ServiceProxyState routes traffic based on ports
pub fn update_service_port(&self, service_name: &str, port: u16) {
    // Updates routing without knowledge of blue/green
}
```

**Coordination Issue:**
- RolloutManager decides when to switch (health checks pass)
- ProxyState performs the actual traffic switch (update port)
- ServiceManager updates in-memory state (active ports)
- Potential for inconsistency if any step fails

**Example Failure Scenario:**
1. RolloutManager decides to switch (green is healthy)
2. ProxyState.update_service_port() succeeds (traffic now goes to green)
3. ServiceManager crashes before updating in-memory ports
4. On restart, ServiceManager thinks blue is still active

**Options to Consider:**
1. **Transactional switch** - write-ahead log, rollback on failure
2. **External state store** - SQLite, Redis for blue-green state
3. **Accept eventual consistency** - reconcile on restart
4. **Make proxy authoritative** - always query ProxyState for active port

**Questions for Reviewer:**
- Has this inconsistency been observed in practice?
- Is blue-green state critical enough to warrant persistent storage?
- Should switches be idempotent and retryable?

---

### 4.3: Global Plugin Manager vs Per-Service Plugin Instances

**Evidence:**
```rust
// Global singleton PluginManager
static PLUGIN_MANAGER: std::sync::OnceLock<PluginManager> = std::sync::OnceLock::new();

pub fn plugin_manager() -> &'static PluginManager {
    PLUGIN_MANAGER.get_or_init(PluginManager::new)
}
```

**Current Design:**
- All plugins are globally registered once at startup
- Services share the same plugin instances via Arc references
- Plugin `init()` is called once with global defaults, not per-service

**Concern:**
- Plugins might need per-service state (e.g., different credentials per service)
- Current architecture shares plugin instances (stateful plugins could conflict)

**Options to Consider:**
1. **Keep global singleton** (current) - plugins must be stateless or use internal maps
2. **Per-service plugin instances** - clone plugins per service, isolated state
3. **Plugin context pattern** - pass service-specific context to every method call

**Questions for Reviewer:**
- Are there use cases for stateful plugins (e.g., per-service database connections)?
- Is the performance cost of per-service plugin instances acceptable?
- Should plugin lifecycle be tied to service lifecycle (init when service starts)?

---

## 5. Growth Indicators & Hotspots

### 5.1: ServiceManager is 1183 lines - Time to Split?

**Current Responsibilities:**
- Service lifecycle (start, stop, restart)
- Dependency resolution (topological sort)
- Health checking (integration with HealthChecker)
- Blue-green deployments (integration with RolloutManager)
- Process spawning (integration with ProcessManager)
- Hook execution (pre/post lifecycle hooks)
- Log streaming (integration with LogBuffer)
- Environment building (integration with EnvironmentResolver)

**Refactoring Options:**
1. **Extract ServiceLifecycle** - start/stop/restart logic only
2. **Extract ServiceCoordinator** - dependency + rollout + health orchestration
3. **Keep monolithic** - ServiceManager is the "orchestrator of orchestrators"

**Questions for Reviewer:**
- Is ServiceManager's complexity intrinsic (orchestration is complex) or accidental?
- Would splitting make testing easier or just scatter logic?
- Are there clear boundaries within ServiceManager that suggest natural splits?

---

### 5.2: Signaling Protocol Version Lock-In

**Evidence:**
- `lib-signaling-protocol` is shared between Hive, Cocoon, Platform API, Signaling Server
- Changes to protocol require coordinated updates across 4+ repositories
- No explicit versioning in protocol messages

**Current Brittleness:**
```rust
pub enum SignalingMessage {
    SpawnCocoon { ... },
    // Adding new fields breaks old clients
}
```

**Options to Consider:**
1. **Add protocol version negotiation** - clients/servers declare supported versions
2. **Use backward-compatible extensions** - optional fields, feature flags
3. **Accept breaking changes** - coordinate updates across all components
4. **Split into versioned sub-protocols** - v1 for cocoon, v2 for hive control

**Questions for Reviewer:**
- How frequently does the signaling protocol change?
- Are all components updated simultaneously or at different cadences?
- Is versioning overhead worth the flexibility?

---

## 6. Design Trade-offs Requiring Expert Judgment

### 6.1: Bundled vs External Plugins - Development vs Production

**Current Hybrid Model:**
- Common plugins bundled via feature flags (docker, stdout, cors, rate-limit, health-http)
- Optional plugins available via `adi plugin install` (podman, loki, prometheus, vault)

**Bundled Advantages:**
- Single binary, no runtime dependencies
- Guaranteed compatibility (same version)
- Faster startup (no dynamic loading)

**External Advantages:**
- Smaller binary size
- Plugin updates without Hive updates
- Third-party plugins possible

**Decision Needed:**
Should Hive move to one model or keep hybrid?

**Questions for Reviewer:**
- What is the expected plugin development velocity (high = favor external)?
- Are third-party plugins a goal or just a theoretical possibility?
- Is binary size a constraint (embedded systems, serverless)?

---

### 6.2: Daemon Mode vs Direct Execution - Primary Use Case?

**Evidence:**
- Hive can run as daemon (`adi hive start --daemon`) with Unix socket control
- Hive can run directly (`hive` binary, foreground process)
- Daemon mode adds complexity (PID files, Unix sockets, graceful shutdown)

**Usage Patterns:**
```bash
# Daemon mode (production)
adi hive start --daemon
adi hive status
adi hive stop

# Direct mode (development)
hive  # Runs in foreground
```

**Decision Needed:**
Should daemon mode be the primary interface or just an option?

**Questions for Reviewer:**
- Do most users run Hive as a systemd service (daemon mode less important)?
- Is the Unix socket control used frequently or just for debugging?
- Should Hive focus on being container-friendly (foreground, signals) vs daemon-friendly?

---

### 6.3: Multi-Source Configuration - Core Feature or Power-User Tool?

**Evidence:**
- SourceManager supports multiple hive.yaml sources (local, git, SQLite)
- Exposure system enables cross-source dependencies
- Adds complexity: source conflicts, FQN resolution, exposure secrets

**Usage Scenarios:**
- **Single source**: Most users have one hive.yaml, manage services locally
- **Multi-source**: Advanced users combine configs from different repos/teams
- **SQLite sources**: Dynamic service creation via API (adi-platform-api integration)

**Decision Needed:**
Should multi-source be prioritized or relegated to advanced documentation?

**Questions for Reviewer:**
- What percentage of Hive users need multi-source configurations?
- Is the exposure system's added complexity justified by usage?
- Should single-source mode be optimized separately (remove overhead)?

---

## Appendix: Architectural Metrics

### Module Size Distribution
| Module | Lines | Responsibility |
|--------|-------|----------------|
| service_manager/mod.rs | 1183 | Service orchestration |
| daemon.rs | 843+ | Daemon IPC |
| signaling_client.rs | 454 | WebSocket client |
| service_proxy.rs | 466+ | HTTP reverse proxy |
| source_manager.rs | 390+ | Multi-source coordination |
| exposure.rs | 200+ | Cross-source dependencies |
| signaling_control.rs | 200+ | Remote control |

### Dependency Depth (Arc<> chains)
```
HiveDaemon
  └── Arc<SourceManager>
        ├── Arc<ExposureManager>
        ├── Arc<ServiceProxyState>
        └── ManagedSource
              └── ServiceManager
                    ├── Arc<ProcessManager>
                    ├── Arc<HealthChecker>
                    ├── Arc<DockerRunner>
                    ├── Arc<RolloutManager>
                    │     └── Arc<ServiceProxyState>  (shared reference)
                    └── Arc<ServiceProxyState>  (shared reference)
```

**Observation:** ServiceProxyState is shared across 3 levels of the hierarchy. Is this intentional data sharing or accidental coupling?

### Plugin Categories
| Category | Built-in | Bundled | External | Total |
|----------|----------|---------|----------|-------|
| Runner   | script   | docker  | podman, compose | 4 |
| Health   | -        | http, tcp | grpc, redis, postgres, mysql, cmd | 7 |
| Proxy    | -        | cors, rate-limit | headers, ip-filter, compress, cache, auth-basic, auth-jwt, auth-oidc, auth-api-key, rewrite | 11 |
| Obs      | -        | stdout, file | loki, prometheus | 4 |
| Env      | -        | dotenv  | vault, aws-secrets, 1password | 4 |
| Rollout  | recreate | -       | blue-green | 2 |

**Observation:** Proxy middleware has the most variety (11 plugins). Is this category more modular by design or just more mature?

---

## Recommended Review Order

1. **Start with boundaries** (Section 1) - foundational decisions affect all other areas
2. **Evaluate coupling** (Section 2) - identify which tight coupling is intentional vs accidental
3. **Clarify ownership** (Section 3) - assign responsibility for integration points
4. **Assess state management** (Section 4) - determine acceptable consistency models
5. **Prioritize refactoring** (Section 5) - decide which growth indicators require action

---

## Questions for Next Steps

1. Should Hive optimize for single-source simplicity or multi-source power?
2. Is the plugin system ready for external contributors or internal-only for now?
3. What is the expected deployment model: single Hive per host, fleet of Hives, or embedded in other services?
4. Are breaking changes acceptable in the next major version (2.0) to simplify architecture?

