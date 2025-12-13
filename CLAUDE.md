adi-cli, rust, monorepo, workspace, submodules, meta-repo

## Overview
- Meta-repository aggregating ADI family components via git submodules
- Build all components: `cargo build --workspace`
- License: BSL-1.0

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
- `crates/lib-color` - Unified color type (RGB/RGBA/Hex)
- `crates/lib-animation` - UI animation utilities
- `crates/lib-syntax-highlight` - Syntax highlighting tokenizer
- `crates/lib-terminal-theme` - Terminal color themes
- `crates/lib-json-tree` - JSON tree view state management
- `crates/lib-terminal-grid` - VTE terminal emulation + PTY
- `crates/lib-iced-ui` - Reusable iced UI components
- `crates/tarminal` - GPU-accelerated terminal emulator
- `crates/debug-metal-shader` - Metal shader debug app

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
- lib-color: `../lib-color`
- lib-animation: `../lib-animation`
- lib-syntax-highlight: `../lib-syntax-highlight`
- lib-terminal-theme: `../lib-terminal-theme`
- lib-json-tree: `../lib-json-tree`
- lib-terminal-grid: `../lib-terminal-grid`
- lib-iced-ui: `../lib-iced-ui`
- tarminal: `../tarminal-app`
- debug-metal-shader: `../debug-metal-shader`
- each crate in the crates dir must be a submodule