cli, rust, monorepo, workspace, submodules, meta-repo

## Overview
- Meta-repository aggregating ADI family components via git submodules
- Build all components: `cargo build --workspace`
- License: BSL-1.0

## Code Guidelines
- For translations and internationalization, prefer Fluent (https://projectfluent.org/)
- Translation plugins follow naming pattern: `[plugin-id].[language-code]` (e.g., `adi.tasks.en-US`)
- `lib-i18n-core`: Core library with Fluent integration, service discovery, and global `t!()` macro
- For colors and theming, use the unified theme system in `packages/theme/` (see `packages/theme/CLAUDE.md`)
- For UI/UX design philosophy, styling patterns, and component usage, see `ADI-STYLING.md`
- For environment variables in CLI crate, use `clienv` module (`crates/cli/src/clienv.rs`):
  - All env var names defined in `EnvVar` enum — never use raw `std::env::var("...")` strings
  - Public getter functions expose typed access: `clienv::theme()`, `clienv::config_dir()`, etc.
  - New env vars: add variant to `EnvVar`, add `as_str()` mapping, add public getter function
- **Always use `lib-console-output`** for all terminal output — never use raw `println!`/`eprintln!`
  - Use `out_info!`, `out_success!`, `out_warn!`, `out_error!`, `out_debug!` macros for messages
  - Use `Section` for headers, `Columns`/`Table` for tabular data, `List` for bullet lists, `KeyValue` for label-value pairs
  - Use `theme::*` functions for styling (`theme::success`, `theme::error`, `theme::brand_bold`, etc.)
  - Exception: raw `println!` is acceptable only for machine-readable/scripting output (paths, raw JSON passthrough)

## Plugin ABI Architecture

The project uses a **unified v3 plugin ABI**:

### Unified v3 ABI
- **lib-plugin-abi-v3** (`crates/lib/lib-plugin-abi-v3`) - Unified plugin interface
- Used by: All plugins (CLI, HTTP, MCP, orchestration, language analyzers)
- Features: Native Rust async traits, type-safe contexts, zero FFI overhead
- Service traits: `CliCommands`, `HttpRoutes`, `McpTools`, `Runner`, `HealthCheck`, `EnvProvider`, `ProxyMiddleware`, `ObservabilitySink`, `RolloutStrategy`, `LanguageAnalyzer`

## Multi-Crate Component Architecture

### User-Facing Components (with plugin)
Components that users interact with via `adi` CLI need a plugin:

| Subdirectory | Purpose | Crate Type |
|--------------|---------|------------|
| `core/` | Business logic, types, traits | Library (`lib`) |
| `http/` | REST API server (axum-based) | Binary |
| `plugin/` | adi CLI plugin (`adi {component} ...`) | Dynamic library (`cdylib`) |
| `cli/` | Standalone CLI (optional) | Binary |

**Components:**
- `agent-loop` (core, http, plugin) - `adi agent run`
- `tasks` (core, cli, http, plugin) - `adi tasks list`
- `indexer` (core, cli, http, plugin) - `adi index`
- `knowledgebase` (core, cli, http, plugin) - `adi kb`
- `llm-proxy` (core, http, plugin) - `adi llm-proxy`
- `hive` (core, http, plugin) - `adi hive`
- `audio` (core, plugin) - `adi audio`
- `tools` (core, plugin) - `adi tools`
- `browser-debug` (core, plugin, mcp) - `adi browser-debug`
- `workflow` (plugin) - `adi workflow`
- `linter` (plugin) - `adi lint`
- `flags` (core, plugin) - `adi flags`

### Backend Services (no plugin)
Services that run on servers and are called via HTTP don't need plugins:

| Subdirectory | Purpose | Crate Type |
|--------------|---------|------------|
| `core/` | Business logic, types, storage | Library (`lib`) |
| `http/` | REST API server (axum-based) | Binary |
| `cli/` | Migrations + server management | Binary |

**Components:**
- `platform` (core, http, cli) - Unified Platform API
- `auth` (core, http) - Authentication service (email + TOTP)
- `signaling-server` - WebSocket signaling
- `balance` - Balance/transaction tracking
- `credentials` - Secure credentials storage
- `logging` - Centralized logging

**Naming convention:**
- Core: `{component}-core` (e.g., `platform-core`)
- HTTP: `{component}-http` (e.g., `platform-http`)
- CLI: `{component}-cli` (e.g., `platform-cli`)
- Plugin: `{component}-plugin` (only for user-facing components)

**Dependencies flow:** `cli` → `core` ← `http` (both cli and http depend on core)

## Submodules

### Core CLI
- `crates/cli` - Component installer/manager

### User-Facing Components (nested structure: core/http/plugin)
- `crates/agent-loop` - Autonomous LLM agents
- `crates/tasks` - Task management
- `crates/indexer` - Code indexer
- `crates/knowledgebase` - Graph DB + embeddings
- `crates/llm-proxy` - LLM API proxy (BYOK/Platform modes)
- `crates/hive` - Cocoon container orchestration (see `crates/hive/CLAUDE.md` for detailed hive docs)
- `crates/audio` - Audio processing
- `crates/tools` - CLI tools collection
- `crates/browser-debug` - Browser debugging + MCP
- `crates/workflow` - Workflow automation
- `crates/linter` - Code linting
- `crates/flags` - File flag tracking (review freshness)
- `crates/lang` - Language analyzers (rust, python, typescript, etc.)

### Backend Services
- `crates/platform` - Unified Platform API (core/http/cli)
- `crates/auth` - Authentication service (email + TOTP)
- `crates/signaling-server` - WebSocket signaling server
- `crates/analytics` - Analytics API (metrics, dashboards)
- `crates/analytics-ingestion` - Analytics event ingestion
- `crates/plugin-registry` - Plugin registry HTTP server
- `crates/executor` - Docker-based task execution
- `crates/cocoon` - Containerized worker for remote execution

### Standalone Services (separate workspaces)
- `crates/balance` - Balance/transaction tracking
- `crates/credentials` - Secure credentials storage (ChaCha20-Poly1305)
- `crates/logging` - Centralized logging (ingestion + query)

### Libraries
- `crates/lib/lib-embed` - Shared embedding library
- `crates/lib/lib-cli-common` - Common CLI utilities
- `crates/lib/lib-daemon-core` - Daemon management (PID, Unix sockets, IPC)
- `crates/lib/lib-migrations` - Database migration framework
- `crates/lib/lib-migrations-core` - Migration core types
- `crates/lib/lib-migrations-sql` - SQL migration utilities
- `crates/lib/lib-misc-color` - Unified color type (RGB/RGBA/Hex)
- `crates/lib/lib-animation` - UI animation utilities
- `crates/lib/lib-syntax-highlight` - Syntax highlighting tokenizer
- `crates/lib/lib-terminal-theme` - Terminal color themes
- `crates/lib/lib-json-tree` - JSON tree view state management
- `crates/lib/lib-terminal-grid` - VTE terminal emulation + PTY
- `crates/lib/lib-iced-ui` - Reusable iced UI components
- `crates/lib/lib-client` - API client utilities
- `crates/lib/lib-console-output` - Console output component library (see below)
- `crates/lib/lib-http-common` - Common HTTP utilities
- `crates/lib/lib-signaling-protocol` - WebSocket signaling protocol
- `crates/lib/lib-tarminal-sync` - Terminal CRDT sync protocol
- `crates/lib/lib-analytics-core` - Analytics client library
- `crates/lib/lib-logging-core` - Logging client with distributed tracing
- `crates/lib/lib-plugin-abi-v3` - Unified plugin ABI
- `crates/lib/lib-plugin-host` - Plugin host runtime
- `crates/lib/lib-plugin-manifest` - Plugin manifest parsing
- `crates/lib/lib-plugin-registry` - Plugin registry client
- `crates/lib/lib-plugin-verify` - Plugin verification
- `crates/lib/lib-indexer-lang-abi` - Language analyzer ABI
- `crates/lib/lib-typespec-api` - TypeSpec API utilities
- `crates/lib/lib-i18n-core` - Internationalization (Fluent)
- `crates/lib/lib-mcp-core` - Model Context Protocol core
- `crates/lib/lib-webrtc-manager` - WebRTC management
- `crates/lib/lib-hive-daemon-client` - Hive daemon client
- `crates/lib/lib-task-store` - Task persistence

### Plugins
- `crates/embed-plugin` - Embedding plugin
- `crates/llm-extract-plugin` - LLM extraction plugin
- `crates/llm-uzu-plugin` - Local LLM inference (Apple Silicon)

### Tools
- `crates/tool-generate-heartbit` - Heartbit generation
- `crates/webrtc-test-peer` - WebRTC testing utility

## FlowMap (Standalone)
- `crates/lib-flowmap-core` - Core flow graph types
- `crates/lib-flowmap-parser` - TypeScript/JavaScript flow extraction
- `apps/flowmap-api` - HTTP API server for flow visualization

### Build FlowMap
```bash
cargo build --release
# Binary at: ./target/release/flowmap-api
```

### Run FlowMap
```bash
# Start the API server (default port 8092)
./target/release/flowmap-api

# Or with custom port
PORT=8080 ./target/release/flowmap-api
```

### FlowMap API
| Endpoint | Description |
|----------|-------------|
| `GET /api/parse?path=/dir` | Parse a directory, returns flow summaries |
| `GET /api/flows?path=/dir` | List flows for a parsed directory |
| `GET /api/flows/{id}?path=/dir` | Get full flow graph by ID |
| `GET /api/flows/{id}/issues?path=/dir` | Get issues (unhandled errors) |

### FlowMap Frontend
- Located at: `/flowmap` in the web UI (apps/infra-service-web)
- Set `NEXT_PUBLIC_FLOWMAP_API_URL` to point to the API server

## Analytics System

### Architecture
```
Services → lib-analytics-core → HTTP POST → analytics-ingestion → TimescaleDB
                                                                           ↓
                                            analytics ← (reads) ←─────────┘
```

- **lib-analytics-core**: HTTP client library that sends events to ingestion service
- **analytics-ingestion**: Receives events via HTTP and writes to TimescaleDB
- **analytics**: REST API for querying metrics and dashboards
- **TimescaleDB**: Time-series database (PostgreSQL extension) for storing events
- **Continuous Aggregates**: Auto-updating materialized views for fast analytics queries

### Event Tracking
All services automatically track events via `AnalyticsClient`:
- API requests (latency, status codes, endpoints)
- Authentication events (login attempts, token refreshes)
- Task lifecycle (created, started, completed, failed)
- Integration health (connections, errors, usage)
- Cocoon activity (registrations, connections, session duration)
- System errors (application errors with context)

### Analytics API Endpoints
| Endpoint | Description |
|----------|-------------|
| `GET /api/analytics/overview` | Dashboard summary (DAU/WAU/MAU, tasks, cocoons) |
| `GET /api/analytics/users/daily` | Daily active users over time |
| `GET /api/analytics/users/weekly` | Weekly active users over time |
| `GET /api/analytics/tasks/daily` | Task stats by day (created, completed, failed) |
| `GET /api/analytics/tasks/overview` | Task success rate and avg duration |
| `GET /api/analytics/api/latency` | API endpoint latency (p50, p95, p99) |
| `GET /api/analytics/api/slowest` | Top 10 slowest endpoints (24h) |

### Key Metrics Tracked
**User Activity:**
- Daily/Weekly/Monthly Active Users
- Total unique users
- Authentication success rates

**Task Performance:**
- Task creation, completion, failure rates
- Average task duration
- P95 task duration
- Success rate percentage

**API Performance:**
- Request latency (p50, p95, p99)
- Requests per endpoint
- Error rates (4xx, 5xx)

**Integration Health:**
- Connections/disconnections
- Usage patterns
- Error frequency

**Cocoon Activity:**
- Active cocoons
- Session durations
- Registration trends

### Integration Example
```rust
use lib_analytics_core::{AnalyticsClient, AnalyticsEvent};

// Initialize in main (points to ingestion service)
let analytics_url = std::env::var("ANALYTICS_URL")
    .unwrap_or_else(|_| "http://localhost:8094".to_string());
let analytics_client = AnalyticsClient::new(analytics_url);

// Track events (batched automatically, sent via HTTP)
analytics_client.track(AnalyticsEvent::TaskCreated {
    task_id: task.id,
    user_id: user.id,
    project_id: Some(project_id),
    cocoon_id: task.cocoon_id,
    command: task.command.clone(),
});
```

### Configuration
Services that track events need:
- `ANALYTICS_URL` - URL of analytics ingestion service (e.g., `http://localhost:8094`)

Analytics ingestion service needs:
- `DATABASE_URL` - PostgreSQL connection string
- `PORT` - Listen port (default: 8094)

## Logging System

### Architecture
```
Request → Service A [trace: abc, span: 001] → Service B [trace: abc, span: 002]
              ↓                                    ↓
         LoggingClient                       LoggingClient
              ↓                                    ↓
                    logging (TimescaleDB)
```

- **lib-logging-core**: Client library with distributed tracing (trace ID + span ID) and correlation IDs
- **logging**: Receives logs via HTTP and stores to TimescaleDB, provides query API
- **TimescaleDB**: Time-series database for log storage with 30-day retention

### Distributed Tracing
All logs include hierarchical trace context:
- **Trace ID**: UUID v7 for entire request chain (propagated across services)
- **Span ID**: UUID v7 for current operation
- **Parent Span ID**: Links spans in parent-child hierarchy

Headers for propagation:
- `X-Trace-ID`: Trace identifier (root of request chain)
- `X-Span-ID`: Current span identifier
- `X-Parent-Span-ID`: Parent span (passed to downstream services)

### Correlation IDs
Business-level correlation IDs allow querying all logs related to a specific entity:
- **Cocoon ID**: All logs for a specific cocoon device (`X-Cocoon-ID` header)
- **User ID**: All logs for a specific user (`X-User-ID` header)
- **Session ID**: All logs for a WebSocket/WebRTC session (`X-Session-ID` header)
- **Hive ID**: All logs for hive orchestration (`X-Hive-ID` header)

### Log Levels (Extended 7)
| Level | Value | Description |
|-------|-------|-------------|
| TRACE | 0 | Most detailed, for development |
| DEBUG | 1 | Debug information |
| INFO | 2 | General information |
| NOTICE | 3 | Notable events |
| WARN | 4 | Warning conditions |
| ERROR | 5 | Error conditions |
| FATAL | 6 | Critical errors |

### Logging API Endpoints
| Endpoint | Description |
|----------|-------------|
| `POST /logs/batch` | Ingest batch of logs |
| `GET /logs` | Query logs with filters |
| `GET /logs/trace/{trace_id}` | Get all logs for a trace |
| `GET /logs/span/{span_id}` | Get logs for a specific span |
| `GET /logs/cocoon/{cocoon_id}` | Get all logs for a cocoon |
| `GET /logs/user/{user_id}` | Get all logs for a user |
| `GET /logs/session/{session_id}` | Get all logs for a session |
| `GET /logs/stats` | Get logging statistics (24h) |
| `GET /health` | Health check |

### Query Parameters
| Parameter | Description |
|-----------|-------------|
| `service` | Filter by service name |
| `level` | Minimum log level (trace, debug, info, notice, warn, error, fatal) |
| `trace_id` | Filter by trace ID |
| `cocoon_id` | Filter by cocoon device ID |
| `user_id` | Filter by user ID |
| `session_id` | Filter by session ID |
| `hive_id` | Filter by hive ID |
| `search` | Search in message text |
| `from` | Start time (ISO 8601) |
| `to` | End time (ISO 8601) |
| `limit` | Max results (default: 100, max: 1000) |
| `offset` | Pagination offset |

### Query Examples
```bash
# All logs for a specific cocoon
curl "http://localhost:8040/logs/cocoon/49ab3b2a32fdb98f..."

# All logs for a specific user
curl "http://localhost:8040/logs/user/09f210bf-f65e-41df..."

# Filter by cocoon and level
curl "http://localhost:8040/logs?cocoon_id=abc123&level=error"

# Search within a session
curl "http://localhost:8040/logs/session/webrtc-123?search=WebRTC"
```

### Integration Example
```rust
use lib_logging_core::{LoggingClient, TraceContext};

// Initialize in main (non-blocking, fire-and-forget)
let client = lib_logging_core::from_env("my-service");

// Create trace context with correlation IDs
let ctx = TraceContext::new()
    .with_cocoon("device-id-here")
    .with_user("user-uuid-here")
    .with_session("ws-session-id");

// Log with context (non-blocking, returns immediately)
client.info("User logged in", &ctx)
    .with_field("email", "user@example.com")
    .send();

// Create child span (preserves correlation IDs)
let child_ctx = ctx.child();
client.debug("Calling auth service", &child_ctx).send();
```

### Non-Blocking Guarantees
- All `log()` calls return immediately (never block)
- Uses unbounded channel (send never blocks)
- Console output via `tracing` always works (backup)
- If logging service is down, logs still appear in console
- Background task handles async HTTP delivery

### Axum Middleware
```rust
use lib_logging_core::{trace_layer, TraceContextExt};

// Add middleware to router
let app = Router::new()
    .route("/api/users", get(list_users))
    .layer(trace_layer());

// Extract context in handler
async fn list_users(req: Request) -> impl IntoResponse {
    let ctx = req.trace_context();
    client.info("Listing users", &ctx).send();
    // ...
}
```

### Configuration
Services need:
- `LOGGING_URL` - URL of logging service (e.g., `http://localhost:8040`)

Logging service needs:
- `DATABASE_URL` - TimescaleDB connection string
- `PORT` - Listen port (default: 8040)

## Console Output Library (`lib-console-output`)

Unified console output component library for all CLI plugins. Use this for **all** user-facing terminal output.

### Theme (`theme` module) — Dynamic ADI Theme System
- Brand/accent color is driven by active theme from `packages/theme/` (ANSI 256-color)
- Status colors (success, error, warning) are universal across all themes
- `theme::init(id)` — set theme programmatically; auto-resolves from `ADI_THEME` env var if not called
- `theme::active()` — get the active `Theme` struct (colors, fonts, name)
- `theme::brand(val)` / `theme::brand_bold(val)` — accent color from active theme
- `theme::info(val)` — accent color (brand-aligned info messages)
- `theme::debug(val)` — cyan (distinct from brand)
- `theme::success(val)` — green
- `theme::warning(val)` — yellow
- `theme::error(val)` — red bold
- `theme::muted(val)` — dim (trace, hints, borders, disabled)
- `theme::bold(val)` — bold (prompts, headers)
- `theme::icons::*` — icon constants: `SUCCESS` (✓), `ERROR` (✕), `WARNING` (⚠), `INFO` (ℹ), `DEBUG` (›), `TRACE` (·), `BRAND` (◆)
- `theme::generated::*` — re-exports: `THEMES`, `DEFAULT_THEME`, `find_theme()`, `Theme`, `ThemeMode`, `ThemeFonts`
- Full theme docs: `packages/theme/CLAUDE.md`

### Block Components (`blocks` module)
All implement `Renderable` trait: `line_count()`, `print() -> LiveHandle`, `render() -> String`, `Display`.

| Component | Use case | Live variant |
|-----------|----------|--------------|
| `Table` | Bordered table with auto-width columns, rounded corners | `LiveTable` — push/set/remove rows, auto re-renders |
| `Columns` | Borderless aligned columns with optional header | — |
| `Card` | Bordered panel with optional title | — |
| `KeyValue` | Aligned label-value pairs | `LiveKeyValue` — set key/value, auto re-renders |
| `Section` | Header with separator line (`── Title ──`) | — |
| `List` | Bullet (`•`) or numbered list | — |

- `LiveHandle` — returned by `.print()`, provides `.clear()` and `.refresh(new)` for in-place terminal updates
- Width calculation uses `console::measure_text_width` — safe with ANSI-styled cell values
- Borders styled via `theme::muted`, titles via `theme::brand_bold`, bullets via `theme::brand`

### Usage Example
```rust
use lib_console_output::{theme, blocks::{Table, Section, Card, KeyValue, Renderable}};

// Section header
Section::new("Services").width(50).print();

// Bordered table with styled cells
let handle = Table::new()
    .header(["Service", "Status", "Port"])
    .row(["web", &theme::success("running").to_string(), "8080"])
    .row(["api", &theme::error("stopped").to_string(), "3000"])
    .print();

// Clear and replace later
handle.clear();

// Live table (auto-refreshes on mutation)
let mut live = blocks::LiveTable::new().header(["Service", "Status"]);
live.push_row(["web", "starting"]);
live.set_row(0, ["web", "running"]); // clears + re-renders
live.done();
```

### Other Modules
- `progress` — Spinner, ProgressBar, StepProgress, MultiProgress (all themed magenta)
- `input` — Select, MultiSelect, Confirm, Input, Password (interactive, JSON stream, fallback modes)
- Macros: `out_info!`, `out_error!`, `out_success!`, `out_warn!`, `out_debug!`, `out_trace!`
- Dual-mode: text (human) + JSON stream (`SILK_MODE=true`) for WebRTC/cloud

## File Flags (`adi.flags`)

- `adi flags init` — create `.adi/flags.toml` (defines states + check mode)
- `adi flags set <state> <files...>` — mark files as clean for a state
- `adi flags status [state]` — show dirty files (modified since last flag)
- `adi flags list [state]` — list tracked files
- `adi flags clear <state> [files...]` — remove flags
- `adi flags states` — list configured states
- Config at `.adi/flags.toml`, index at `.adi/cache/flags/<state>`

## Cocoon
- Cocoon is a containerized worker environment that connects to the signaling server
- Provides isolated execution environment for running commands remotely
- Replaces file-based execution with real-time WebSocket communication
- Used by executor to run tasks in Docker containers with live command streaming

## Uzu LLM Plugin (Apple Silicon only)
- `crates/llm-uzu-plugin` - Local LLM inference plugin for Apple Silicon
- `crates/lib-client-uzu` - Uzu inference engine client library (dependency)
- **Distribution**: Pre-built binaries via plugin registry
- **Installation**: `adi plugin install adi.llm.uzu`
- **Performance**: ~35 tokens/sec on M2 (Llama-3.2-1B)
- **Requirements**: macOS with Apple Silicon (M1/M2/M3+)
- **Build Requirements** (for developers only):
  - Metal Toolchain: `xcodebuild -downloadComponent MetalToolchain`
  - Excluded from default workspace builds
- **Usage**: `adi llm-uzu load <model> && adi llm-uzu generate <model> <prompt>`
- **Alternative to**: Ollama for maximum performance on Apple Silicon
- **Privacy**: 100% local inference, no API calls

## Packages
- `packages/theme` - Unified ADI theme system (see `packages/theme/CLAUDE.md` for full docs)

## Apps
- `apps/infra-service-web` - Web UI for ADI (Next.js + Tailwind CSS)
- `apps/flowmap-api` - FlowMap HTTP API server for code flow visualization
- `apps/web-app` - Web application
- `apps/chrome-extension-debugger` - Chrome extension for debugging

## Production Release Images
All production services are built using **cross-compilation** for 10-20x faster builds than Docker.

### Architecture
**Fast Build Pipeline:**
1. Cross-compile Rust binaries natively on macOS to Linux (x86_64-unknown-linux-musl)
2. Copy pre-built binaries into minimal Alpine containers (~5MB vs 1GB+)
3. Push to registry

**Services:**
- `analytics` - Analytics API (metrics, dashboards)
- `analytics-ingestion` - Analytics event ingestion service
- `auth` - Authentication service (email + TOTP)
- `platform` - Unified Platform API
- `signaling-server` - WebSocket signaling server
- `plugin-registry` - Plugin registry server
- `flowmap-api` - Code flow visualization API
- `hive` - Hive: Cocoon orchestration API

Each release directory contains:
- `Dockerfile` - Minimal Alpine image (copies pre-built binary)
- `docker-compose.yml` - Production deployment with Traefik labels
- `.env.example` - Environment variable template
- `README.md` - Service documentation

### Setup (one-time)
```bash
# 1. Install musl target for static Linux binaries
rustup target add x86_64-unknown-linux-musl

# 2. Install musl-cross toolchain (macOS) - REQUIRED
brew install filosottile/musl-cross/musl-cross
```

**Why musl-cross is required:**
- Provides `x86_64-linux-musl-gcc` linker
- Needed for Rust crates with C dependencies (like ring, boring-ssl, etc.)
- Without it, builds will fail with "tool not found" errors

**Note:** The `.cargo/config.toml` file configures the linker automatically.

### Build Release Images
```bash
# Build Linux binaries (native speed, persistent Cargo cache)
adi workflow build-linux                        # Interactive: select services
adi workflow build-linux --services auth    # Build specific service

# Build Docker images + push (optional)
adi workflow release                            # Interactive: select services
adi workflow release --push                     # Build + push to registry
adi workflow release --services auth --tag v1.0.0  # Build with custom tag
```

### Performance Benefits
- ⚡ **10-20x faster**: Native build vs Docker emulation
- 💾 **Persistent cache**: Cargo cache survives across builds
- 📦 **Smaller images**: 5MB Alpine vs 1GB+ multi-stage
- 🔄 **Parallel builds**: Build all services concurrently

### Deploy to Production
All services use Traefik for routing at `https://adi.the-ihor.com/api/*`:
```bash
cd release/adi.the-ihor.com/auth
cp .env.example .env  # Configure environment
docker-compose up -d  # Deploy with Traefik
```

## ADI Workflows

Interactive workflows are defined in `.adi/workflows/` directory. Each workflow has a `.toml` config and corresponding `.sh` script.

### Available Workflows

| Workflow | Description | Command |
|----------|-------------|---------|
| `build-linux` | Cross-compile services for Linux | `adi workflow build-linux` |
| `build-plugin` | Build and install plugins locally (no registry) | `adi workflow build-plugin` |
| `release` | Build + Docker image + push to registry | `adi workflow release` |
| `release-cli` | Build and release CLI binary | `adi workflow release-cli` |
| `deploy` | Deploy services to Coolify | `adi workflow deploy` |
| `release-plugin` | Build and publish a single plugin | `adi workflow release-plugin` |
| `release-plugins` | Build and publish multiple plugins | `adi workflow release-plugins` |
| `commit-submodule` | Commit changes in submodule and parent | `adi workflow commit-submodule` |
| `lint-plugin` | Lint a plugin before release | `adi workflow lint-plugin` |
| `seal` | Commit and push all changes including submodules | `adi workflow seal` |
| `cocoon-images` | Build cocoon Docker image variants | `adi workflow cocoon-images` |
| `autodoc` | Generate API documentation with LLM enrichment | `adi workflow autodoc` |
| `sync-theme` | Sync theme JSON to CSS + Rust outputs | `adi workflow sync-theme` |
| `convert-sounds` | Convert audio files | `adi workflow convert-sounds` |

### Workflow Structure

Each workflow consists of:
- `<name>.toml` - Workflow configuration (inputs, steps, conditions)
- `<name>.sh` - Bash script with actual implementation

### Example: Release Workflow
```bash
# Interactive mode - select services
adi workflow release

# Release specific service
adi workflow release llm-proxy

# Release and push to registry
adi workflow release llm-proxy --push

# Release all services with custom tag
adi workflow release all --push --tag v1.0.0
```

### Example: Deploy Workflow
```bash
# Interactive mode
adi workflow deploy

# Or with arguments
adi workflow deploy --action deploy --service auth
```

### Example: Release Plugin
```bash
# Interactive - select plugin and target
adi workflow release-plugin

# Direct - release specific plugin
adi workflow release-plugin --plugin agent-loop --registry production
```

### Example: Build Plugin (Local Development)
Build and install plugins locally without publishing to registry:
```bash
# Interactive mode
adi workflow build-plugin

# Build and install directly
.adi/workflows/build-plugin.sh adi.hive --install

# Build, force-replace existing, skip lint (fastest)
.adi/workflows/build-plugin.sh adi.hive --install --force --skip-lint

# Build only (output to dist/plugins/)
.adi/workflows/build-plugin.sh adi.cocoon
```

**Options:**
- `--install` - Install to `~/.local/share/adi/plugins/`
- `--force` - Replace existing installation
- `--skip-lint` - Skip linting for faster builds

**Common plugins:** `adi.hive`, `adi.cocoon`, `adi.agent-loop`, `adi.tasks`, `adi.workflow`

### Example: Autodoc (API Documentation)
Generate API documentation for Rust crates with LLM enrichment and translations:
```bash
# Interactive mode
adi workflow autodoc

# Generate English docs
.adi/workflows/autodoc.sh lib-embed --lang en

# Generate Ukrainian docs with LLM enrichment
.adi/workflows/autodoc.sh lib-embed --lang uk --enrich

# Overwrite existing documentation
.adi/workflows/autodoc.sh lib-embed --lang en --enrich --force
```

**Options:**
- `--lang <code>` - Language (en, uk, ru, zh, ja, ko, es, de, fr)
- `--enrich` - Use LLM (claude CLI) to enrich docs with examples and descriptions
- `--force` - Overwrite existing documentation

**Output:** `.adi/docs/<crate-name>/<lang>/api.md`

**Requirements:**
- `cargo-public-api` (auto-installed if missing)
- `claude` CLI (for `--enrich` option)

## Setup
```bash
git clone --recursive <repo>
# or after clone:
git submodule update --init --recursive
```

## Building
```bash
cargo build --workspace           # Build all
cargo build -p cli                # Build adi CLI
cargo build -p indexer-cli        # Build specific package
```

## Local Development

### Prerequisites
- **Hive local orchestrator** - manages services, routing, and reverse proxy via `adi hive`
- Add to `/etc/hosts`: `127.0.0.1 adi.local`

### Quick Start
```bash
cp .env.local.example .env.local  # Create config (one time)
adi hive up                       # Start all services (uses .adi/hive.yaml)
adi hive status                   # Check service status
adi hive restart web              # Restart specific service
adi hive down                     # Stop all services
```

### Web UI Environment (apps/infra-service-web/.env.local)
```bash
NEXT_PUBLIC_SIGNALING_URL=ws://adi.local/api/signaling/ws
NEXT_PUBLIC_PLATFORM_API_URL=http://adi.local/api/platform
NEXT_PUBLIC_PROXY_API_URL=http://adi.local/api/llm-proxy
AUTH_API_URL=http://adi.local/api/auth
```

### Local URLs (via hive proxy at http://adi.local)
| Path | Service | Port | Description |
|------|---------|------|-------------|
| `/` | Web UI | 8013 | Next.js frontend |
| `/api/auth/*` | Auth API | 8012 | Authentication (email + TOTP) |
| `/api/platform/*` | Platform API | 8015 | Tasks, projects, integrations |
| `/api/flowmap/*` | FlowMap API | 8017 | Code flow visualization |
| `/api/analytics/*` | Analytics API | 8023 | Metrics, dashboards, aggregates |
| `/api/analytics-ingestion/*` | Analytics Ingestion | 8022 | Event ingestion |
| `/api/llm-proxy/*` | LLM Proxy | 8029 | LLM API proxy (BYOK/Platform) |
| `/api/balance/*` | Balance API | 8030 | Balance and transaction tracking |
| `/api/credentials/*` | Credentials API | 8032 | Secure credentials storage |
| `/api/signaling/*` | Signaling | 8011 | WebSocket relay for sync |
| `/api/logging/*` | Logging | 8040 | Centralized logging service |
| `/api/registry/*` | Registry | 8019 | Plugin registry (optional) |
| `/api/hive/*` | Hive | 8020 | Cocoon orchestration (optional) |

### Direct Service Ports
| Service | URL | Description |
|---------|-----|-------------|
| PostgreSQL | localhost:8027 | Auth, Platform, LLM Proxy databases |
| TimescaleDB | localhost:8028 | Analytics database |
| Coturn TURN | turn:localhost:3478 | WebRTC NAT traversal (user: adi, pass: adi) |
| Web UI | http://localhost:8013 | Next.js frontend |
| Auth API | http://localhost:8012 | Authentication (email + TOTP) |
| Platform API | http://localhost:8015 | Tasks, projects, integrations |
| FlowMap API | http://localhost:8017 | Code flow visualization |
| Signaling | ws://localhost:8011/ws | WebSocket relay for sync |
| Analytics Ingestion | http://localhost:8022 | Event ingestion |
| Analytics API | http://localhost:8023 | Metrics, dashboards, aggregates |
| LLM Proxy | http://localhost:8029 | LLM API proxy (BYOK/Platform) |
| Balance API | http://localhost:8030 | Balance and transaction tracking |
| Credentials API | http://localhost:8032 | Secure credentials storage |
| Logging | http://localhost:8040 | Centralized logging service |
| Hive | http://localhost:8020 | Cocoon orchestration (optional) |
| Registry | http://localhost:8019 | Plugin registry (optional) |

### Native Development (No Docker)
For faster iteration on specific services:
```bash
# Terminal/pane 1: Signaling server
cd crates/signaling-server && cargo run

# Terminal/pane 2: Auth service
cd crates/auth && DATABASE_URL=postgres://postgres:postgres@localhost/adi_auth cargo run -p auth-http

# Terminal/pane 3: Web UI
cd apps/infra-service-web && npm run dev

# Terminal/pane 4: Cocoon (optional)
cd crates/cocoon && SIGNALING_SERVER_URL=ws://localhost:8080/ws cargo run
```

### Configuration (.env.local)
Key variables:
- `DATABASE_URL` - PostgreSQL connection for auth (e.g., postgres://postgres:postgres@localhost/adi_auth)
- `JWT_SECRET` - Auth token signing (min 32 chars)
- `HMAC_SALT` - Device ID derivation for cocoon
- `SMTP_*` - Email settings (optional for local dev)
- `RUST_LOG` - Log level (info, debug, trace)
- `ADI_THEME` - Theme override for CLI (e.g., `indigo`, `scarlet`, `emerald`)

## CLI Usage
The `adi` CLI provides direct plugin commands for convenience:
```bash
adi tasks list                    # Direct task management
adi agent-loop run                # Direct agent loop access
adi run adi.tasks list            # Alternative plugin run syntax
```

## Updating Submodules
```bash
git submodule update --remote     # Pull latest from all submodules
```

## Component Repos
Each submodule is an independent repo that can be developed standalone.
- Each crate in `crates/` must be a submodule
- Each app in `apps/` must be a submodule
- Nested components (like `indexer/core`) are contained in a single submodule repo