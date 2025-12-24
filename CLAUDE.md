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
- `crates/adi-indexer-mcp` - Code indexer MCP server
- `crates/adi-tasks-core` - Task management core library
- `crates/adi-tasks-cli` - Task management CLI
- `crates/adi-tasks-http` - Task management HTTP server
- `crates/adi-tasks-mcp` - Task management MCP server
- `crates/adi-knowledgebase-core` - Knowledgebase core library (graph DB + embeddings)
- `crates/adi-knowledgebase-cli` - Knowledgebase CLI
- `crates/adi-knowledgebase-http` - Knowledgebase HTTP server
- `crates/adi-knowledgebase-mcp` - Knowledgebase MCP server
- `crates/adi-agent-loop-core` - Agent loop core library (autonomous LLM agents)
- `crates/adi-agent-loop-cli` - Agent loop CLI
- `crates/adi-agent-loop-http` - Agent loop HTTP server
- `crates/adi-agent-loop-mcp` - Agent loop MCP server
- `crates/adi-executor` - Docker-based task execution service
- `crates/lib-misc-color` - Unified color type (RGB/RGBA/Hex)
- `crates/lib-animation` - UI animation utilities
- `crates/lib-syntax-highlight` - Syntax highlighting tokenizer
- `crates/lib-terminal-theme` - Terminal color themes
- `crates/lib-json-tree` - JSON tree view state management
- `crates/lib-terminal-grid` - VTE terminal emulation + PTY
- `crates/lib-iced-ui` - Reusable iced UI components
- `crates/lib-client-github` - GitHub API client library
- `crates/lib-client-openrouter` - OpenRouter API client library
- `crates/debug-metal-shader` - Metal shader debug app

## Apps
- `apps/tarminal-native-macos` - Native macOS app for Tarminal (SwiftUI)

## Setup
```bash
git clone --recursive <repo>
# or after clone:
git submodule update --init --recursive
```

## Building
```bash
cargo build --workspace           # Build all
cargo build -p adi-indexer-cli    # Build specific package
cargo build -p tarminal           # Build tarminal
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
- adi-indexer-mcp: `../adi-indexer-mcp`
- adi-tasks-core: `../adi-tasks-core`
- adi-tasks-cli: `../adi-tasks-cli`
- adi-tasks-http: `../adi-tasks-http`
- adi-tasks-mcp: `../adi-tasks-mcp`
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
- tarminal-native-macos: `../tarminal-native-macos`
- each crate in the crates dir must be a submodule
- each app in the apps dir must be a submodule