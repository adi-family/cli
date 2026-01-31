# Architecture Review: Hive Component

## Summary
- **8 boundary decisions** requiring evaluation (1 ✅ completed, 7 pending)
- **5 abstraction questions** to consider
- **4 integration points** needing ownership clarity
- **2 growth indicators** suggesting potential refactoring

**Scope:** 50,214 lines of code across core (14,372 lines), http (215 lines), plugin (1,468 lines), ssl module, and 35+ plugin crates.

---

## Boundary Decisions Needed

### 1. ~~Should `lib-tarminal-sync` be a shared dependency or internalized?~~ ✅ COMPLETED

**Decision:** Extracted signaling protocol types to `lib-signaling-protocol`

**Implementation:**
- ✅ Created new crate: `crates/lib/lib-signaling-protocol`
- ✅ Contains: `SignalingMessage` enum and all supporting types (CocoonKind, CertificateInfo, HiveInfo, WebRtcSessionInfo, ServiceInfo, browser debug types, Silk terminal protocol)
- ✅ `lib-tarminal-sync` now focuses purely on terminal CRDT synchronization (SyncMessage, VersionVector, GridDelta)
- ✅ All 9 consumers migrated to `lib-signaling-protocol`:
  1. hive/core
  2. hive/http
  3. hive/plugins/orchestrator
  4. cocoon
  5. tarminal-signaling-server
  6. adi-platform-api
  7. adi-browser-debug/core
  8. webrtc-test-peer
  9. lib-tarminal-sync (dependency removed)
- ✅ Zero coupling between the two protocols

**Rationale:**
- Hive, cocoon, signaling-server, and platform-api don't participate in terminal emulation - they only use WebSocket message protocol
- Clear separation of concerns: signaling protocol vs terminal sync protocol
- Allows independent evolution of each protocol
- Reduces coupling between unrelated domains
- Cleaner architecture with explicit boundaries

**Files Changed (25 total):**
- Created: `crates/lib/lib-signaling-protocol/` (new crate with lib.rs, Cargo.toml, CLAUDE.md)
- Updated: 9 Cargo.toml files (all consumers)
- Updated: 13 source files (import statements)
- Updated: `Cargo.toml` - added lib-signaling-protocol to workspace members
- Updated: `CLAUDE.md` - documented new crate
- Updated: `hive-architecture-review.md` - marked decision as completed

**Status:** ✅ **COMPLETED** - All consumers migrated, clean separation achieved, no backward compatibility cruft

**Impact:** Completed successfully - 9 crates migrated, zero breaking changes, all builds passing

---

### 2. Should SSL functionality be bundled in core or remain a separate crate?

**Evidence:**
- `/Users/mgorunuch/projects/adi-family/cli/crates/hive/ssl/` exists as separate crate (adi-hive-ssl)
- ssl/Cargo.toml shows it depends on hive-core (line 15: `adi-hive-core = { path = "../core" }`)
- http/src/main.rs (lines 58-123) creates SSL manager but delegates all logic to adi-hive-ssl
- core/src/ssl.rs is only 90 lines - just defines `SslManagerHandle` trait boundary
- adi-hive-ssl is ~2GB of dependencies (ACME client, rustls, x509 parsing)

**Current State:**
SSL is cleanly separated with a handle/trait boundary. Core defines the interface (`SslManagerHandle`), ssl crate implements certificate issuance, http binary wires them together.

**Options to Consider:**
1. **Keep current separation** (recommended) - Clean boundary, optional feature, independent versioning
2. **Merge into core as feature** - Simplify dependency graph at cost of bloating core
3. **Extract to standalone service** - Dedicated cert-manager process (over-engineering?)

**Questions for Reviewer:**
- Will hive support multiple SSL providers (Let's Encrypt, ZeroSSL, custom CA)?
- Should SSL be optional at compile time or runtime?
- Do other components need to share certificate management logic?

**Trade-offs:**
- Current design: Clear separation of concerns, but adds 1 extra crate
- Feature flag in core: Simpler build, but couples core to ACME/TLS implementation details

**Impact:** Low - Current design is working well, mostly a documentation/organization decision

---

### 3. Should plugin ABI live in hive or be a shared library?

**Evidence:**
- `/Users/mgorunuch/projects/adi-family/cli/crates/hive/plugins/abi/` exists inside hive directory
- plugins/abi/Cargo.toml: workspace dependencies, no hive-specific logic
- Used by 35+ plugin crates, all within hive/plugins/
- Defines universal traits: `RunnerPlugin`, `HealthPlugin`, `EnvPlugin`, `ProxyPlugin`, `ObsPlugin`
- No dependencies on hive-core itself

**Current State:**
Plugin ABI is physically located within the hive component tree but is logically independent. It defines contracts that both bundled and external plugins implement.

**Options to Consider:**
1. **Move to crates/lib/hive-plugin-abi** - Make it visibly shared infrastructure
2. **Keep in hive/plugins/abi** - Treat as internal to hive ecosystem
3. **Split ABI per category** - hive-runner-abi, hive-proxy-abi, etc. (over-modularization?)

**Questions for Reviewer:**
- Will non-hive components ever implement these plugin interfaces?
- Should plugin ABI have independent versioning from hive releases?
- Is the ABI stable, or will it evolve frequently with hive features?

**Impact:** Low-Medium - Refactoring is mechanical, but affects plugin discovery/loading

---

### 4. ~~Should daemon functionality be core's responsibility or a separate crate?~~ ✅ COMPLETED

**Decision:** Extracted generic daemon infrastructure to `lib-daemon-core`, kept hive-specific logic in hive-core

**Implementation:**
- ✅ Created new crate: `crates/lib/lib-daemon-core`
- ✅ Provides reusable daemon infrastructure:
  - PID file management with stale file detection
  - Unix socket server/client utilities
  - IPC protocol framework (request/response traits)
  - Graceful shutdown coordinator with signal handling
  - Auto-cleanup (RAII for PID files and sockets)
- ✅ Refactored `hive-core` to use `lib-daemon-core`:
  - Replaced manual PID file handling with `PidFile` struct
  - Replaced manual socket handling with `UnixSocketServer`/`UnixSocketClient`
  - Replaced `mpsc` shutdown with `ShutdownCoordinator`
  - Kept hive-specific logic: `DaemonRequest`/`DaemonResponse`, source management, proxy, observability
- ✅ All tests passing, builds successfully

**Architecture:**
```
lib-daemon-core (generic infrastructure)
    ↓
adi-hive-core (hive-specific daemon logic)
    ↓
adi-hive-http (daemon binary)
```

**Rationale:**
- Generic daemon patterns (PID files, Unix sockets) are reusable across ADI components
- Clear separation: infrastructure vs. business logic
- Other services (future: adi-task-daemon, adi-agent-daemon) can reuse lib-daemon-core
- Reduced code duplication and improved maintainability
- Hive-specific logic (sources, services, proxy, observability) remains in hive-core

**Files Changed (5 total):**
- Created: `crates/lib/lib-daemon-core/` (new crate with 6 modules)
- Updated: `crates/hive/core/src/daemon.rs` (refactored to use lib-daemon-core)
- Updated: `crates/hive/core/Cargo.toml` (added lib-daemon-core dependency)
- Updated: `Cargo.toml` (added lib-daemon-core to workspace)
- Created: `crates/lib/lib-daemon-core/CLAUDE.md` (documentation)

**Status:** ✅ **COMPLETED** - Generic daemon infrastructure extracted, hive refactored successfully

**Impact:** Completed successfully - Clean separation achieved, zero breaking changes, all builds passing

---

### 5. Should core provide both YAML and SQLite config backends?

**Evidence:**
- core/src/hive_config/parser.rs: YAML parsing (442 lines)
- core/src/sqlite_backend.rs: SQLite config backend (1,099 lines)
- Two completely different configuration paradigms:
  - YAML: read-only, version-controlled, declarative
  - SQLite: read-write, dynamic updates, runtime patches
- Both exposed through `SourceManager` but serve different use cases

**Current State:**
Hive supports "infrastructure as code" (YAML) and "runtime configuration" (SQLite) in the same component. This creates dual execution paths for every config operation.

**Options to Consider:**
1. **Split into hive-config-yaml and hive-config-sqlite** - Core depends on both via traits
2. **Keep unified** - Configuration abstraction justifies having both
3. **SQLite as extension layer** - All configs start as YAML, SQLite stores runtime overrides

**Questions for Reviewer:**
- Are YAML and SQLite sources equally important, or is one a legacy/migration path?
- Should users mix YAML and SQLite sources in the same daemon instance?
- Do they share enough logic to justify living together?

**Trade-offs:**
- Unified: Simple mental model, but complex implementation
- Split: Clean separation, but more crates and integration testing burden

**Impact:** High - Affects configuration strategy and user experience

---

### 6. Is the bundled vs. external plugin architecture sustainable?

**Evidence:**
- core/Cargo.toml (lines 82-89): Optional dependencies for 8 bundled plugins
- core/Cargo.toml (lines 106-125): Feature flags to control bundling
- core/src/plugins.rs: PluginManager handles both bundled and external
- 35+ plugin crates in hive/plugins/ directory
- Every new plugin requires:
  1. Implementing plugin ABI traits
  2. Adding to hive-core features (if bundled)
  3. Publishing to plugin registry (if external)

**Current State:**
Hive uses a hybrid approach - common plugins bundled by default, specialized plugins installed externally. This creates complexity in plugin loading, versioning, and discovery.

**Architecture Questions:**
- How do you version bundled vs. external plugins?
- Can external plugins override bundled ones?
- What happens when bundled plugin depends on external plugin?

**Options to Consider:**
1. **All plugins external** - Core provides only plugin loading infrastructure
2. **Keep hybrid with clear rules** - Define which categories must be bundled
3. **Plugins as separate workspace** - Move all plugins out of hive/ directory

**Questions for Reviewer:**
- Which plugins are truly "core" (e.g., docker runner) vs. optional (e.g., OIDC auth)?
- Should plugin bundling be deployment-time decision rather than compile-time?
- Is the plugin registry trustworthy enough to depend on for core functionality?

**Impact:** High - Affects build system, deployment, and plugin ecosystem

---

### 7. Should proxy functionality be in core or hive-http?

**Evidence:**
- core/src/http_proxy.rs: 400+ lines of proxy server logic
- core/src/service_proxy.rs: 547 lines of service routing logic
- core/src/proxy_plugins.rs: 711 lines of middleware (CORS, rate limiting, etc.)
- http/src/main.rs: Only 215 lines - mostly SSL and signaling setup
- Plugin CLI uses proxy through daemon client, never directly

**Current State:**
The HTTP proxy server lives in hive-core (a library crate) but is only ever used by the HTTP binary. This violates the typical library/binary separation.

**Options to Consider:**
1. **Move proxy to adi-hive-http** - Core provides only proxy types/traits
2. **Keep in core** - Proxy is fundamental to hive's service orchestration model
3. **Extract to adi-service-proxy** - Reusable proxy library for all ADI services

**Questions for Reviewer:**
- Will other ADI components need the same proxy functionality?
- Is the proxy tightly coupled to hive's service management, or is it generic?
- Should proxy be usable without hive daemon?

**Impact:** Medium-High - Affects core API and reusability across ADI platform

---

### 8. Should signaling client be in core or orchestrator plugin?

**Evidence:**
- core/src/signaling_client.rs: 453 lines - WebSocket client for cocoon spawning
- core/src/signaling_control.rs: 618 lines - Remote control via signaling server
- hive/plugins/orchestrator/ exists but unclear relationship to core signaling
- http/src/main.rs (lines 204-212): Signaling client is main loop, not library usage
- Plugin CLI never uses signaling directly

**Current State:**
Signaling client is in core but only used by the HTTP binary. This makes hive-core depend on network I/O and WebSocket protocol even when used as a library.

**Options to Consider:**
1. **Move signaling to hive-http** - Core provides only types/messages
2. **Move to orchestrator plugin** - Cocoon management is plugin responsibility
3. **Keep in core** - Signaling is fundamental to hive's identity

**Questions for Reviewer:**
- Is hive's primary purpose cocoon orchestration or service management?
- Should hive-core be usable offline (no signaling server)?
- Will other components implement the same signaling protocol?

**Impact:** High - Affects core's purpose and deployment requirements

---

## Abstraction Fit

### 1. Is `ServiceManager` doing too much?

**Evidence:**
- core/src/service_manager/mod.rs: 1,182 lines
- Responsibilities include:
  - Service lifecycle (start/stop/restart)
  - Health checking
  - Blue-green deployments
  - Environment resolution
  - Docker/process management
  - Observability events
  - Hook execution
- 8 sub-modules: docker_runner, env_plugins, environment, health, process, rollout, runners
- 68 public methods on ServiceManager struct

**Questions for Reviewer:**
- Should deployment strategies (blue-green) be separate from lifecycle management?
- Is environment resolution coupled too tightly to service execution?
- Could sub-modules become separate manager types?

**Symptoms of Over-Responsibility:**
- Difficult to test in isolation
- Many optional dependencies
- Mixed abstraction levels (Docker API calls + business logic)

**Impact:** High - Affects testability and maintainability

---

### 2. Are there missing domain concepts in the configuration model?

**Evidence:**
- core/src/hive_config/types.rs: 670 lines defining config structures
- Flat service config with many optional fields (runner, proxy, healthcheck, hooks, rollout, expose, uses, environment, etc.)
- No clear grouping of related concerns
- Example: `ServiceConfig` has 18+ fields at same nesting level

**Missing Abstractions:**
- **Deployment specification** - rollout + healthcheck + hooks could be grouped
- **Network configuration** - proxy + expose + uses are all networking concerns
- **Runtime environment** - environment + volumes + secrets

**Questions for Reviewer:**
- Is the flat structure intentional for YAML ergonomics?
- Should config validation enforce relationships between fields?
- Would grouped structs make programmatic manipulation clearer?

**Impact:** Medium - Affects configuration complexity and validation

---

### 3. Is the plugin ABI too fine-grained or too coarse?

**Evidence:**
- 9 plugin categories: parse, runner, env, health, rollout, proxy.ssl, proxy.auth, proxy, obs
- 35+ plugin implementations in hive/plugins/
- Each category has separate trait in hive-plugin-abi
- Some plugins (e.g., runner-docker) are large (500+ lines), others (proxy-cors) are tiny (50 lines)

**Questions for Reviewer:**
- Should small middleware plugins (CORS, headers) use a lightweight extension API instead of full plugin ABI?
- Are there missing plugin categories (e.g., service discovery, secrets management)?
- Should rollout strategies be plugins or built into core?

**Trade-offs:**
- Fine-grained: Flexibility, many small plugins, discovery complexity
- Coarse-grained: Simpler ecosystem, less flexibility, bundling required

**Impact:** Medium - Affects plugin development experience

---

### 4. Is observability event system the right abstraction?

**Evidence:**
- core/src/observability.rs: 755 lines
- core/src/observability_plugins.rs: 496 lines
- Events: ServiceStart, ServiceStop, ServiceCrash, ServiceLog, HealthCheck, etc.
- In-memory buffer with broadcast channels
- Plugins consume events (stdout, file, future: Loki, Prometheus)

**Architecture:**
```rust
Service → EventCollector → Broadcast → ObsPlugin(stdout/file)
```

**Questions for Reviewer:**
- Should hive use structured logging (tracing) instead of custom event system?
- Is observability pushing toward full telemetry system (metrics, traces, logs)?
- Do events provide enough context for distributed system debugging?

**Alternative Approaches:**
- Use OpenTelemetry OTLP export instead of custom plugins
- Integrate with lib-logging-core (mentioned in CLAUDE.md)
- Keep simple for embedded use cases

**Impact:** Medium - Affects monitoring and debugging capabilities

---

### 5. Generic naming: "Manager", "Config", "State"

**Evidence:**
- `SourceManager` (core/src/source_manager.rs: 617 lines)
- `ServiceManager` (core/src/service_manager/mod.rs: 1,182 lines)
- `RolloutManager` (core/src/service_manager/rollout.rs)
- `ExposureManager` (core/src/exposure.rs)
- `PluginManager` (core/src/plugins.rs)
- `DockerManager` (core/src/docker_manager.rs)

**Questions for Reviewer:**
- Do these Manager classes represent distinct domain concepts?
- Could some be renamed to reflect actual responsibility (e.g., SourceRegistry, ServiceOrchestrator)?
- Are there missing coordination patterns that would benefit from more specific names?

**Impact:** Low-Medium - Affects code discoverability and onboarding

---

## Cross-Cutting Concerns

### 1. Configuration interpolation scattered across parsing and runtime

**Evidence:**
- hive_config/interpolation.rs: Variable substitution at parse time
- service_manager/environment.rs: Environment resolution at runtime
- Both use similar ${VAR} syntax but different resolution rules
- YAML defaults merged separately from runtime patches (SQLite)

**Questions for Reviewer:**
- Should all interpolation happen at parse time, or are runtime variables necessary?
- Is the two-phase approach (parse + runtime) intentional or accidental complexity?
- Could we unify under a single configuration resolution pipeline?

**Impact:** Medium - Affects configuration predictability

---

### 2. Error handling patterns inconsistent

**Evidence:**
- Some modules use `anyhow::Result` (service_manager, daemon)
- Some use custom error types with `thiserror` (signaling_client, docker_manager)
- Plugin ABI uses `anyhow::Result` in traits
- No consistent error categorization (retryable, permanent, user-facing)

**Questions for Reviewer:**
- Should user-facing errors be distinguished from internal errors?
- Do errors need structured context for observability?
- Should plugins return typed errors for better error handling?

**Impact:** Low-Medium - Affects error recovery and debugging

---

### 3. Async execution patterns mixed

**Evidence:**
- Some code uses tokio::spawn for background tasks
- Daemon uses select! for concurrent operations
- Service manager uses manual future polling in places
- No consistent approach to task cancellation/shutdown

**Questions for Reviewer:**
- Should there be a task orchestration layer for managing background work?
- Is graceful shutdown handled consistently across all components?
- Should long-running operations support cancellation tokens?

**Impact:** Medium - Affects reliability and shutdown behavior

---

### 4. State management distributed across multiple stores

**Evidence:**
- In-memory: ServiceManager runtime state
- SQLite: Configuration backend, exposure registry
- PID files: Daemon process management
- Unix socket: IPC state
- Log buffer: Observability events

**Questions for Reviewer:**
- Is this intentional separation of concerns or accidental complexity?
- Should there be a unified state abstraction?
- How is state recovery handled after crashes?

**Impact:** High - Affects data consistency and recovery

---

## Integration Points

### 1. What owns the Docker socket?

**Evidence:**
- DockerManager in hive-core (core/src/docker_manager.rs)
- ServiceManager uses DockerManager via Arc
- Plugin system allows runner-docker plugin
- Both bundled and external paths to Docker

**Questions for Reviewer:**
- Can multiple hive instances run on same machine?
- Should Docker resources be namespaced by hive instance?
- Who is responsible for Docker image lifecycle (pull, prune)?

**Integration Complexity:** Container naming, network isolation, volume cleanup

---

### 2. Who manages the unified proxy lifecycle?

**Evidence:**
- Daemon starts proxy server (daemon.rs line 35-38)
- ServiceProxyState manages routes from multiple sources
- SSL manager can reload certs dynamically
- No clear shutdown coordination

**Questions for Reviewer:**
- If daemon crashes, does proxy stop gracefully?
- How are proxy routes updated without downtime?
- Should proxy be separate process for better isolation?

**Integration Complexity:** Service discovery, health check integration, SSL reload

---

### 3. How do sources coordinate with exposure manager?

**Evidence:**
- ExposureManager stores exposed services (core/src/exposure.rs)
- SourceManager loads/unloads sources (core/src/source_manager.rs)
- Unclear ordering: does exposure happen before or after service start?
- No clear cleanup when source is disabled

**Questions for Reviewer:**
- Can services from different sources expose to each other?
- What happens to exposed services when source is reloaded?
- Should exposure be bidirectional (service A depends on B, B knows about A)?

**Integration Complexity:** Dependency resolution, circular dependencies, cleanup

---

### 4. Remote control vs. local control consistency

**Evidence:**
- Local: Unix socket (daemon.rs)
- Remote: WebSocket signaling (signaling_control.rs)
- Both implement `HiveRequest/HiveResponse` protocol
- Different authentication models (local=PID check, remote=device registration)

**Questions for Reviewer:**
- Should both control paths have same capabilities?
- How is authorization enforced for sensitive operations (shutdown, config updates)?
- Can remote control and local control conflict?

**Integration Complexity:** Auth model, request routing, state consistency

---

## Growth Indicators

### 1. daemon.rs (1,304 lines) - needs decomposition

**Evidence:**
- Handles Unix socket server, request routing, source orchestration, proxy management
- 3 TODO comments indicating incomplete features
- Largest file in hive-core
- Mixed abstraction levels (low-level socket handling + high-level orchestration)

**Refactoring Options:**
- Extract socket server to daemon/socket.rs
- Extract request handlers to daemon/handlers.rs
- Extract state management to daemon/state.rs

**Impact:** Code organization and testability

---

### 2. service_manager/mod.rs (1,182 lines) - approaching complexity limit

**Evidence:**
- 68 public methods
- Manages Docker, processes, health checks, rollouts, hooks
- Deep nesting in some functions (5+ levels)
- 8 sub-modules but still 1,182 lines in main module

**Refactoring Options:**
- Extract rollout logic to separate orchestrator
- Move health checking to dedicated component
- Split into ServiceLifecycle + ServiceRuntime

**Impact:** Testability and feature development velocity

---

## Dependency Direction Issues

### 1. Core depends on concrete plugin implementations

**Evidence:**
- core/Cargo.toml lines 82-89: Optional dependencies on hive-runner-docker, hive-obs-stdout, etc.
- Feature flag "bundled-plugins" pulls in 8 concrete plugins
- Violates dependency inversion - core should depend only on plugin ABI

**Questions for Reviewer:**
- Should bundled plugins be wired in hive-http instead of core?
- Is core meant to be a library or a batteries-included framework?
- Could plugin bundling be a build-time tool instead of feature flags?

**Impact:** Core API surface and reusability

---

### 2. SSL module depends on core, but core provides SSL abstractions

**Evidence:**
- ssl/Cargo.toml line 15: `adi-hive-core = { path = "../core" }`
- core/src/ssl.rs defines SslManagerHandle interface
- Circular knowledge: core knows about SSL concepts, SSL implements core abstractions

**Current Design:**
```
core → ssl interface (SslManagerHandle)
ssl → core types (Config, etc.) + implements interface
```

**Questions for Reviewer:**
- Should SSL types be extracted to a shared ssl-types crate?
- Is the current circular knowledge acceptable?
- Would inverting the dependency (core depends on ssl traits) be cleaner?

**Impact:** Module independence and testing

---

## Configuration vs. Code Decisions

### 1. Plugin selection: runtime config or compile-time features?

**Evidence:**
- Bundled plugins: Compile-time feature flags in Cargo.toml
- External plugins: Runtime installation via `adi plugin install`
- Hybrid creates two plugin loading paths
- hive.yaml references plugins by ID (runtime), but bundled plugins must be compiled in

**Questions for Reviewer:**
- Should plugin bundling be purely a deployment concern?
- Can we make all plugins external and bundle at Docker build time?
- Is compile-time optimization (bundled) worth the complexity?

**Impact:** Build system and deployment flexibility

---

### 2. Service routing: declarative config or imperative code?

**Evidence:**
- Proxy routes defined in YAML (hive.yaml: services.*.proxy)
- ExposureManager stores runtime mappings
- ServiceProxyState builds routing table at runtime
- No way to programmatically manipulate routes without config changes

**Questions for Reviewer:**
- Should advanced routing (A/B testing, canary) be configurable or require code?
- Is YAML expressive enough for complex routing rules?
- Should there be a route DSL or API for dynamic manipulation?

**Impact:** Feature completeness and user flexibility

---

## Recommended Priority for Review

### ✅ Completed
1. ~~**lib-tarminal-sync separation**~~ (Decision #1) - ✅ Completed - Extracted to lib-signaling-protocol

### High Priority (Architectural Identity)
2. **Daemon vs. Library** (Decision #4) - Affects core's purpose
3. **Signaling in Core** (Decision #8) - Defines hive's primary responsibility
4. **Bundled Plugins** (Decision #6) - Affects ecosystem strategy
5. **State Management** (Cross-Cutting #4) - Impacts reliability

### Medium Priority (Maintainability)
6. **ServiceManager Decomposition** (Growth #2) - Technical debt
7. **YAML vs. SQLite** (Decision #5) - Configuration strategy
8. **Proxy in Core** (Decision #7) - Library reusability
9. **Plugin ABI Location** (Decision #3) - Organization clarity

### Lower Priority (Incremental Improvement)
10. **SSL Separation** (Decision #2) - Already well-designed
11. **Error Handling** (Cross-Cutting #2) - Can evolve incrementally
12. **Naming Patterns** (Abstraction #5) - Low-risk refactoring

---

## Files and Patterns Referenced

**Key Files:**
- `/Users/mgorunuch/projects/adi-family/cli/crates/hive/core/src/lib.rs` - Public API surface (100 lines)
- `/Users/mgorunuch/projects/adi-family/cli/crates/hive/core/src/daemon.rs` - Daemon orchestration (1,304 lines)
- `/Users/mgorunuch/projects/adi-family/cli/crates/hive/core/src/service_manager/mod.rs` - Service lifecycle (1,182 lines)
- `/Users/mgorunuch/projects/adi-family/cli/crates/hive/core/Cargo.toml` - Dependency configuration
- `/Users/mgorunuch/projects/adi-family/cli/crates/hive/http/src/main.rs` - Entry point (215 lines)
- `/Users/mgorunuch/projects/adi-family/cli/crates/hive/plugin/src/lib.rs` - CLI plugin (1,468 lines)

**Dependency Pattern:**
```
plugin → core ← http
         ↓
      ssl, lib-tarminal-sync, plugins/*
```

**External Dependencies:**
- `lib-tarminal-sync` - Used by 9 crates, shared signaling protocol
- 35+ plugin crates in hive/plugins/ - Hybrid bundled/external model
- adi-hive-ssl - Optional SSL/TLS with Let's Encrypt
