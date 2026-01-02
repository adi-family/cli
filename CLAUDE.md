adi-cli, rust, monorepo, workspace, submodules, meta-repo

## Overview
- Meta-repository aggregating ADI family components via git submodules
- Build all components: `cargo build --workspace`
- License: BSL-1.0

## Resources
- Icons: https://phosphoricons.com

## Code Guidelines
- NEVER use emojis in code - use Phosphor icons from https://phosphoricons.com instead
- All icons must be Phosphor unicode glyphs rendered with ICON_FONT

## Submodules
- `crates/adi-cli` - Component installer/manager
- `crates/lib-embed` - Shared embedding library
- `crates/lib-cli-common` - Common CLI utilities
- `crates/lib-migrations` - Database migration framework
- `crates/adi-indexer-core` - Code indexer core library
- `crates/adi-indexer-cli` - Code indexer CLI
- `crates/adi-indexer-http` - Code indexer HTTP server
- `crates/adi-tasks-core` - Task management core library
- `crates/adi-tasks-cli` - Task management CLI
- `crates/adi-tasks-http` - Task management HTTP server
- `crates/adi-knowledgebase-core` - Knowledgebase core library (graph DB + embeddings)
- `crates/adi-knowledgebase-cli` - Knowledgebase CLI
- `crates/adi-knowledgebase-http` - Knowledgebase HTTP server
- `crates/adi-agent-loop-core` - Agent loop core library (autonomous LLM agents)
- `crates/adi-agent-loop-cli` - Agent loop CLI
- `crates/adi-agent-loop-http` - Agent loop HTTP server
- `crates/adi-executor` - Docker-based task execution service
- `crates/cocoon` - Containerized worker with signaling server connectivity for remote command execution
- `crates/cocoon-manager` - REST API for on-demand cocoon container orchestration
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

## Apps
- `apps/infra-service-web` - Web UI for ADI (Next.js + Tailwind CSS)
- `apps/infra-service-auth` - Auth service deployment (docker-compose + adi-auth submodule)

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

### Quick Start (Docker)
```bash
cp .env.local.example .env.local  # Create config (one time)
./scripts/dev.sh up               # Start all services
```

### Services
| Service | URL | Description |
|---------|-----|-------------|
| Web UI | http://localhost:8013 | Next.js frontend |
| Auth API | http://localhost:8090 | Authentication (email + TOTP) |
| Platform API | http://localhost:8091 | Tasks, projects, integrations |
| Signaling | ws://localhost:8011/ws | WebSocket relay for sync |
| FlowMap API | http://localhost:8092 | Code flow visualization |
| Analytics Ingestion | http://localhost:8094 | Event ingestion (receives from services) |
| Analytics API | http://localhost:8093 | Metrics, dashboards, aggregates |
| Cocoon Manager | http://localhost:8020 | Cocoon orchestration API (optional) |
| Registry | http://localhost:8019 | Plugin registry (local) |
| Cocoon | (internal) | Worker container (optional) |

### Dev Script (`./scripts/dev.sh`)
Works in terminal, tmux, screen, and CI/CD pipelines.

**Lifecycle commands:**
```bash
./scripts/dev.sh up       # Start all services (creates .env.local if missing)
./scripts/dev.sh down     # Stop all services
./scripts/dev.sh restart  # Restart all services
./scripts/dev.sh status   # Show service status with ports
```

**Development commands:**
```bash
./scripts/dev.sh logs           # Follow all logs (Ctrl+C to stop)
./scripts/dev.sh logs auth      # Follow specific service logs
./scripts/dev.sh shell auth     # Open shell in container (bash or sh)
./scripts/dev.sh build          # Rebuild Docker images
./scripts/dev.sh rebuild        # Force rebuild + restart (no cache)
./scripts/dev.sh clean          # Stop + remove volumes (fresh start)
```

**Utilities:**
```bash
./scripts/dev.sh mail     # Start Mailpit for email testing
./scripts/dev.sh native   # Show native run instructions
./scripts/dev.sh help     # Show all commands + environment info
```

### tmux/screen Usage
The script auto-detects tmux/screen and handles TTY properly:
- Colors work in all multiplexer terminals
- `logs` falls back to last 100 lines if no TTY
- `shell` requires TTY (normal pane has it)
- `native` shows tip about creating panes
- `help` shows current environment status

```bash
# Force colors if needed
FORCE_COLOR=1 ./scripts/dev.sh status
```

### Native Development (No Docker)
For faster iteration on specific services:
```bash
# Terminal/pane 1: Signaling server
cd crates/tarminal-signaling-server && cargo run

# Terminal/pane 2: Auth service
cd crates/adi-auth && cargo run -p adi-auth-http

# Terminal/pane 3: Web UI
cd apps/infra-service-web && npm run dev

# Terminal/pane 4: Cocoon (optional)
cd crates/cocoon && SIGNALING_SERVER_URL=ws://localhost:8080/ws cargo run
```

### Email Testing
Use Mailpit for local email testing:
```bash
./scripts/dev.sh mail     # Start Mailpit container
# SMTP: localhost:1025
# Web UI: http://localhost:8025

# Add to .env.local:
SMTP_HOST=host.docker.internal
SMTP_PORT=1025
```

### Configuration (.env.local)
Key variables:
- `JWT_SECRET` - Auth token signing (min 32 chars)
- `HMAC_SALT` - Device ID derivation for cocoon
- `SMTP_*` - Email settings (optional for local dev)
- `RUST_LOG` - Log level (info, debug, trace)

## Production Deployment

### Deploy Script (`./scripts/deploy.sh`)
Manages Coolify deployments for production services.

**Requirements:**
- `COOLIFY_API_KEY` environment variable (get from Coolify → Keys & Tokens → API tokens)
- `COOLIFY_URL` (default: http://in.the-ihor.com)

**Commands:**
```bash
./scripts/deploy.sh status              # Check all services status
./scripts/deploy.sh deploy web          # Deploy single service
./scripts/deploy.sh deploy all          # Deploy all services
./scripts/deploy.sh deploy auth -f      # Force rebuild (no cache)
./scripts/deploy.sh watch platform      # Watch deployment progress
./scripts/deploy.sh logs signaling      # View deployment logs
./scripts/deploy.sh list web 10         # List last 10 deployments
```

**Services:**
| Name | Description |
|------|-------------|
| auth | Auth API (adi-auth) |
| platform | Platform API (adi-platform-api) |
| signaling | Signaling Server (tarminal-signaling-server) |
| web | Web UI (infra-service-web) |

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
- adi-indexer-core: `../adi-indexer-core`
- adi-indexer-cli: `../adi-indexer-cli`
- adi-indexer-http: `../adi-indexer-http`
- adi-tasks-core: `../adi-tasks-core`
- adi-tasks-cli: `../adi-tasks-cli`
- adi-tasks-http: `../adi-tasks-http`
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