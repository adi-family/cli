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
- `crates/debug-metal-shader` - Metal shader debug app

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
| Web UI | http://localhost:3000 | Next.js frontend |
| Auth API | http://localhost:8090 | Authentication (email + TOTP) |
| Signaling | ws://localhost:8080/ws | WebSocket relay for sync |
| Cocoon | (internal) | Worker container |

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