adi-cli, rust, monorepo, workspace, submodules, meta-repo

## Overview
- Meta-repository aggregating ADI family components via git submodules
- Build all components: `cargo build --workspace`
- License: BSL-1.0

## Code Guidelines
- For translations and internationalization, prefer Fluent (https://projectfluent.org/)
- Translation plugin system: See `docs/i18n-translation-plugin-system.md` for architecture and implementation guide
- Translation plugins follow naming pattern: `[plugin-id].[language-code]` (e.g., `adi.tasks.en-US`, `adi.tasks.zh-CN`)

## Internationalization (i18n)

### Status
- ✅ Phase 1: Core infrastructure (`lib-i18n-core`) - Complete
- ✅ Phase 2: English translation plugin (`adi-cli-lang-en`) - Complete
- ✅ Phase 3: Integration into `adi-cli` - Complete (96+ messages converted)
- ⏳ Phase 4: Testing & documentation - Pending

### Implementation Progress
See `NEXT_STEPS_I18N.md` for detailed status and remaining work.

### Architecture
- **lib-i18n-core**: Core library with Fluent integration, service discovery, and global `t!()` macro
- **Translation plugins**: Dynamic plugins that provide `.ftl` message files
  - `adi-cli-lang-en`: English translations (~100 messages across 7 domains)
  - Future: `adi-cli-lang-zh-CN`, `adi-cli-lang-uk-UA`, etc.
- **Service-based discovery**: Plugins register translation services, discovered at runtime

### Message Domains
1. Self-update (11 messages) - Update checking and installation
2. Shell completions (7 messages) - Completion initialization
3. Plugin management (25 messages) - Install, update, uninstall
4. Search (5 messages) - Registry search
5. Services (3 messages) - Service listing
6. Run commands (8 messages) - Plugin execution
7. External commands (9 messages) - Dynamic command dispatch
8. Common (9 messages) - Shared UI elements

## Multi-Crate Component Architecture
Several components follow a standard multi-crate structure within a single directory:

| Subdirectory | Purpose | Crate Type |
|--------------|---------|------------|
| `core/` | Business logic, types, traits | Library (`lib`) |
| `http/` | REST API server (axum-based) | Binary |
| `plugin/` | adi CLI plugin | Dynamic library (`cdylib`) |
| `cli/` | Standalone CLI (optional) | Binary |

**Components using this pattern:**
- `adi-agent-loop` (core, http, plugin) - Autonomous LLM agents with tool use
- `adi-tasks` (core, cli, http, plugin) - Task management
- `adi-indexer` (core, cli, http, plugin) - Code indexing
- `adi-knowledgebase` (core, cli, http) - Graph DB + embeddings
- `adi-api-proxy` (core, http, plugin) - LLM API proxy with BYOK/Platform modes

**Naming convention:**
- Core: `adi-{component}-core` (e.g., `adi-agent-loop-core`)
- HTTP: `adi-{component}-http` (e.g., `adi-agent-loop-http`)
- Plugin: `adi-{component}-plugin` (e.g., `adi-agent-loop-plugin`)

**Dependencies flow:** `plugin` → `core` ← `http` (both plugin and http depend on core)

## Submodules
- `crates/adi-cli` - Component installer/manager
- `crates/lib-embed` - Shared embedding library
- `crates/lib-cli-common` - Common CLI utilities
- `crates/lib-migrations` - Database migration framework
- `crates/adi-indexer/core` - Code indexer core library
- `crates/adi-indexer/cli` - Code indexer CLI
- `crates/adi-indexer/http` - Code indexer HTTP server
- `crates/adi-tasks/core` - Task management core library
- `crates/adi-tasks/cli` - Task management CLI
- `crates/adi-tasks/http` - Task management HTTP server
- `crates/adi-knowledgebase/core` - Knowledgebase core library (graph DB + embeddings)
- `crates/adi-knowledgebase/cli` - Knowledgebase CLI
- `crates/adi-knowledgebase/http` - Knowledgebase HTTP server
- `crates/adi-agent-loop/core` - Agent loop core library (autonomous LLM agents)
- `crates/adi-agent-loop/http` - Agent loop HTTP server
- `crates/adi-agent-loop/plugin` - Agent loop plugin (includes CLI functionality)
- `crates/adi-api-proxy/core` - API Proxy core library (BYOK/Platform modes, encryption)
- `crates/adi-api-proxy/http` - API Proxy HTTP server (OpenAI-compatible proxy)
- `crates/adi-api-proxy/plugin` - API Proxy CLI plugin
- `crates/adi-executor` - Docker-based task execution service
- `crates/cocoon` - Containerized worker with signaling server connectivity for remote command execution
- `crates/hive` - Hive: WebSocket-based cocoon orchestration (spawns/manages containers via signaling server)
- `crates/lib-misc-color` - Unified color type (RGB/RGBA/Hex)
- `crates/lib-animation` - UI animation utilities
- `crates/lib-syntax-highlight` - Syntax highlighting tokenizer
- `crates/lib-terminal-theme` - Terminal color themes
- `crates/lib-json-tree` - JSON tree view state management
- `crates/lib-terminal-grid` - VTE terminal emulation + PTY
- `crates/lib-iced-ui` - Reusable iced UI components
- `crates/lib-client-github` - GitHub API client library
- `crates/lib-client-openrouter` - OpenRouter API client library
- `crates/lib-tarminal-sync` - Client-agnostic sync protocol for Tarminal
- `crates/tarminal-signaling-server` - WebSocket signaling server for device pairing
- `crates/adi-platform-api` - Unified Platform API (tasks, integrations, orchestration)
- `crates/lib-analytics-core` - Analytics event tracking and persistence library
- `crates/adi-analytics-api` - Analytics API (metrics, dashboards, aggregates)
- `crates/adi-balance-api` - Balance and transaction tracking service
- `crates/debug-metal-shader` - Metal shader debug app

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
Services → lib-analytics-core → HTTP POST → adi-analytics-ingestion → TimescaleDB
                                                                             ↓
                                              adi-analytics-api ← (reads) ←─┘
```

- **lib-analytics-core**: HTTP client library that sends events to ingestion service
- **adi-analytics-ingestion**: Receives events via HTTP and writes to TimescaleDB
- **adi-analytics-api**: REST API for querying metrics and dashboards
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

### Database Migrations
Analytics migrations are managed by `lib-analytics-core` binary:
```bash
# Run analytics migrations
cd crates/lib-analytics-core
cargo run --bin analytics-migrate --features migrate all

# Check status
cargo run --bin analytics-migrate --features migrate status
```

### Database Schema
Events are stored in `analytics_events` table (TimescaleDB hypertable):
- Automatic time-series partitioning by day
- Compression after 7 days (~90% space savings)
- 90-day retention policy for raw events
- Continuous aggregates kept indefinitely

### Continuous Aggregates
Auto-updating materialized views for fast queries:
- `analytics_daily_active_users` - DAU/WAU/MAU
- `analytics_task_stats_daily` - Task metrics by day
- `analytics_api_latency_hourly` - API performance by hour
- `analytics_integration_health_daily` - Integration status
- `analytics_auth_events_daily` - Authentication metrics
- `analytics_cocoon_activity_daily` - Cocoon usage
- `analytics_errors_hourly` - Error tracking

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

## Cocoon
- Cocoon is a containerized worker environment that connects to the signaling server
- Provides isolated execution environment for running commands remotely
- Replaces file-based execution with real-time WebSocket communication
- Used by adi-executor to run tasks in Docker containers with live command streaming

## Uzu LLM Plugin (Apple Silicon only)
- `crates/adi-llm-uzu-plugin` - Local LLM inference plugin for Apple Silicon
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

## Apps
- `apps/infra-service-web` - Web UI for ADI (Next.js + Tailwind CSS)
- `apps/flowmap-api` - FlowMap HTTP API server for code flow visualization

## Production Release Images
All production services are built using **cross-compilation** for 10-20x faster builds than Docker.

### Architecture
**Fast Build Pipeline:**
1. Cross-compile Rust binaries natively on macOS to Linux (x86_64-unknown-linux-musl)
2. Copy pre-built binaries into minimal Alpine containers (~5MB vs 1GB+)
3. Push to registry

**Services:**
- `adi-analytics-api` - Analytics API (metrics, dashboards)
- `adi-analytics-ingestion` - Analytics event ingestion service
- `adi-auth` - Authentication service (email + TOTP)
- `adi-platform-api` - Unified Platform API
- `tarminal-signaling-server` - WebSocket signaling server
- `adi-plugin-registry` - Plugin registry HTTP server
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
adi workflow build-linux --services adi-auth    # Build specific service

# Build Docker images + push (optional)
adi workflow release                            # Interactive: select services
adi workflow release --push                     # Build + push to registry
adi workflow release --services adi-auth --tag v1.0.0  # Build with custom tag
```

### Performance Benefits
- ⚡ **10-20x faster**: Native build vs Docker emulation
- 💾 **Persistent cache**: Cargo cache survives across builds
- 📦 **Smaller images**: 5MB Alpine vs 1GB+ multi-stage
- 🔄 **Parallel builds**: Build all services concurrently

### Deploy to Production
All services use Traefik for routing at `https://adi.the-ihor.com/api/*`:
```bash
cd release/adi.the-ihor.com/adi-auth
cp .env.example .env  # Configure environment
docker-compose up -d  # Deploy with Traefik
```

## ADI Workflows

Interactive workflows are defined in `.adi/workflows/` directory. Each workflow has a `.toml` config and corresponding `.sh` script.

### Available Workflows

| Workflow | Description | Command |
|----------|-------------|---------|
| `build-linux` | Cross-compile services for Linux | `adi workflow build-linux` |
| `release` | Build + Docker image + push to registry | `adi workflow release` |
| `deploy` | Deploy services to Coolify | `adi workflow deploy` |
| `dev` | Local development environment | `adi workflow dev` |
| `release-plugin` | Build and publish a single plugin | `adi workflow release-plugin` |
| `release-plugins` | Build and publish multiple plugins | `adi workflow release-plugins` |
| `commit-submodule` | Commit changes in submodule and parent | `adi workflow commit-submodule` |
| `lint-plugin` | Lint a plugin before release | `adi workflow lint-plugin` |
| `seal` | Commit and push all changes including submodules | `adi workflow seal` |
| `cocoon-images` | Build cocoon Docker image variants | `adi workflow cocoon-images` |

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

## Setup
```bash
git clone --recursive <repo>
# or after clone:
git submodule update --init --recursive
```

## Building
```bash
cargo build --workspace           # Build all
cargo build -p adi-cli            # Build adi CLI
cargo build -p adi-indexer-cli    # Build specific package
```

## Local Development

### Prerequisites
1. **Nginx reverse proxy** - Located at `~/projects/docker-compose.yaml`
   ```bash
   cd ~/projects && docker-compose up -d nginx
   ```
2. **Add to /etc/hosts**: `127.0.0.1 adi.local`
3. **Nginx config**: `~/projects/.config/nginx/sites-enabled/adi.local`
   - Reload after changes: `docker exec nakit_yok_nginx nginx -s reload`

### Quick Start
```bash
cp .env.local.example .env.local  # Create config (one time)
adi workflow dev                  # Interactive: start services
.adi/workflows/dev.sh up          # Non-interactive: start default services
.adi/workflows/dev.sh status      # Check service status
.adi/workflows/dev.sh restart web # Restart specific service
```

### Web UI Environment (apps/infra-service-web/.env.local)
```bash
NEXT_PUBLIC_SIGNALING_URL=ws://adi.local/api/signaling/ws
NEXT_PUBLIC_PLATFORM_API_URL=http://adi.local/api/platform
NEXT_PUBLIC_PROXY_API_URL=http://adi.local/api/llm-proxy
AUTH_API_URL=http://adi.local/api/auth
```

### Local URLs (via nginx at http://adi.local)
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
| `/api/signaling/*` | Signaling | 8011 | WebSocket relay for sync |
| `/api/registry/*` | Registry | 8019 | Plugin registry (optional) |
| `/api/hive/*` | Hive | 8020 | Cocoon orchestration (optional) |

### Direct Service Ports
| Service | URL | Description |
|---------|-----|-------------|
| PostgreSQL | localhost:8027 | Auth, Platform, LLM Proxy databases |
| TimescaleDB | localhost:8028 | Analytics database |
| Web UI | http://localhost:8013 | Next.js frontend |
| Auth API | http://localhost:8012 | Authentication (email + TOTP) |
| Platform API | http://localhost:8015 | Tasks, projects, integrations |
| FlowMap API | http://localhost:8017 | Code flow visualization |
| Signaling | ws://localhost:8011/ws | WebSocket relay for sync |
| Analytics Ingestion | http://localhost:8022 | Event ingestion |
| Analytics API | http://localhost:8023 | Metrics, dashboards, aggregates |
| LLM Proxy | http://localhost:8029 | LLM API proxy (BYOK/Platform) |
| Balance API | http://localhost:8030 | Balance and transaction tracking |
| Hive | http://localhost:8020 | Cocoon orchestration (optional) |
| Registry | http://localhost:8019 | Plugin registry (optional) |

### Native Development (No Docker)
For faster iteration on specific services:
```bash
# Terminal/pane 1: Signaling server
cd crates/tarminal-signaling-server && cargo run

# Terminal/pane 2: Auth service
cd crates/adi-auth && DATABASE_URL=postgres://postgres:postgres@localhost/adi_auth cargo run -p adi-auth-http

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

## Deployment Repos
Some crates have separate deployment wrapper repos that contain them as submodules:
- `apps/infra-service-auth` wraps `crates/adi-auth`

After pushing changes to a crate, also update its deployment repo:
```bash
cd apps/infra-service-auth
git submodule update --remote adi-auth
git add adi-auth && git commit -m "🔗 Update adi-auth: <description>" && git push
```

## Component Repos
Each submodule is an independent repo that can be developed standalone:
- adi-cli: `../adi-cli`
- lib-embed: `../lib-embed`
- lib-cli-common: `../lib-cli-common`
- lib-migrations: `../lib-migrations`
- adi-indexer: `../adi-indexer` (contains core, cli, http, plugin)
- adi-tasks: `../adi-tasks` (contains core, cli, http, plugin)
- lib-misc-color: `../lib-misc-color`
- lib-animation: `../lib-animation`
- lib-syntax-highlight: `../lib-syntax-highlight`
- lib-terminal-theme: `../lib-terminal-theme`
- lib-json-tree: `../lib-json-tree`
- lib-terminal-grid: `../lib-terminal-grid`
- lib-iced-ui: `../lib-iced-ui`
- lib-client-github: `../lib-client-github`
- lib-client-openrouter: `../lib-client-openrouter`
- debug-metal-shader: `../debug-metal-shader`
- adi-executor: `../adi-executor`
- adi-web-ui: `../adi-web-ui`
- each crate in the crates dir must be a submodule
- each app in the apps dir must be a submodule