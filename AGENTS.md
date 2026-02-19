# ADI Crate Structure

> Auto-generate with: `adi wf generate-agents-md`

## User-Facing Components
Components with plugin for `adi` CLI integration.

| Crate | Structure | Description |
|-------|-----------|-------------|
| `agent-loop` | core,http,plugin | Core library for ADI Agent Loop - autonomous LLM agent with tool use |
| `audio` | core,plugin | ADI Audio core library - WAV processing, filters, EQ, compression, normalization |
| `browser-debug` | core,mcp,plugin | Core library for ADI Browser Debug - types and signaling client |
| `flags` | core,plugin | Core library for ADI file flag tracking |
| `hive` | core,http,plugin | Hive core library - cocoon container orchestration business logic |
| `indexer` | cli,core,http,plugin | Core indexer library for ADI - parsing, storage, search |
| `knowledgebase` | cli,core,http,plugin | Core knowledgebase library for ADI - graph DB, embedding storage, semantic search |
| `linter` | core,plugin | Core library for ADI Linter - language-agnostic linting with external/plugin/command rules |
| `llm-proxy` | core,http,plugin | ADI LLM Proxy - Core library for LLM API proxying with BYOK/Platform modes |
| `tasks` | cli,core,http,mcp,plugin | Core library for ADI Tasks - task management with dependency graphs |
| `tools` | core,plugin | Core library for tool index - searchable CLI tool discovery |
| `workflow` | mcp,plugin | ADI Workflow plugin - run shell workflows defined in TOML files |

## Backend Services
HTTP services without CLI plugin.

| Crate | Structure | Description |
|-------|-----------|-------------|
| `analytics` | core,http | Core library for ADI Analytics API - types and database queries |
| `analytics-ingestion` | core,http | Core library for ADI Analytics Ingestion - event writer |
| `auth` | cli,core,http | Core library for ADI Auth - email-based passwordless authentication |
| `balance` | cli,core,http | Core library for ADI Balance API - balance and transaction tracking |
| `credentials` | cli,core,http | Core library for ADI Credentials API - secure credentials storage |
| `executor` | core,http | Core library for Docker-based task execution — types, orchestration, Docker client, output handlers, job store |
| `logging` | core,http | Core library for ADI Logging Service |
| `platform` | cli,core,http | Core library for ADI Platform API - business logic, types, and storage |
| `plugin-registry` | core,http | Core library for ADI plugin registry - storage and business logic |
| `signaling-server` | core,http | Core library for signaling server — types, state, security, token validation, and utility functions |

## Libraries
Shared libraries in `crates/lib/`.

| Library | Purpose |
|---------|---------|
| `lib-analytics-core` | - |
| `lib-animation` | UI animation utilities - easing functions, spring physics, animation manager |
| `lib-cli-common` | Common CLI utilities for ADI tools |
| `lib-client` | - |
| `lib-console-output` | Console output abstraction with support for text and JSON stream modes |
| `lib-daemon-core` | Generic daemon management library with PID files, Unix sockets, and IPC |
| `lib-embed` | Shared embedding library for ADI tools |
| `lib-env-parse` | Typed environment variable parsing — bool, string, with consistent truthy/falsy semantics |
| `lib-flowmap-core` | Core types for FlowMap code visualization |
| `lib-flowmap-parser` | Flow parser for TypeScript/JavaScript/Python/Java codebases |
| `lib-hive-daemon-client` | - |
| `lib-http-common` | Common HTTP utilities for ADI services |
| `lib-i18n-core` | Internationalization library using Mozilla Fluent with plugin-based translation discovery |
| `lib-iced-ui` | Reusable iced UI components - buttons, cards, pills, tabs, inputs |
| `lib-indexer-lang-abi` | Stable ABI definitions for indexer language plugins |
| `lib-json-tree` | JSON tree view state management - framework-agnostic |
| `lib-logging-core` | Centralized logging library with distributed tracing support for ADI services |
| `lib-mcp-core` | MCP (Model Context Protocol) implementation with JSON-RPC transport |
| `lib-migrations` | Database migration framework for ADI components |
| `lib-migrations-core` | Generic migration framework - bring your own storage and actions |
| `lib-migrations-sql` | SQL migrations built on lib-migrations-core |
| `lib-misc-color` | Unified color type with lazy conversion - RGB, RGBA, Hex support |
| `lib-plugin-abi-v3` | Unified plugin ABI for ADI ecosystem (v3 - native async traits) |
| `lib-plugin-host` | Plugin host for loading and managing v3 plugins |
| `lib-plugin-manifest` | Plugin manifest parsing (plugin.toml and package.toml) |
| `lib-plugin-registry` | Plugin registry HTTP client |
| `lib-plugin-verify` | Plugin signature and checksum verification |
| `lib-shortcuts` | Standardized shortcut URL registry for ADI products |
| `lib-signaling-protocol` | - |
| `lib-syntax-highlight` | Semantic syntax highlighting tokenizer - framework-agnostic |
| `lib-tarminal-sync` | - |
| `lib-task-store` | Device-owned task storage abstraction for hybrid cloud |
| `lib-terminal-grid` | VTE terminal emulation with grid, parser, and PTY support |
| `lib-terminal-theme` | Terminal color themes and typography - framework-agnostic |
| `lib-typespec-api` | TypeSpec parser and multi-language code generator in pure Rust |
| `lib-webrtc-manager` | - |

## Standalone Plugins

| Plugin | Description |
|--------|-------------|
| `embed-plugin` | ADI Embed plugin providing text embedding services via fastembed/ONNX |
| `llm-extract-plugin` | Extract LLM-friendly documentation from ADI plugins |
| `llm-uzu-plugin` | ADI Uzu LLM plugin for local inference on Apple Silicon |

## Tools

| Tool | Description |
|------|-------------|
| `cocoon` | Cocoon: containerized environment with signaling server connectivity for remote command execution |
| `tool-generate-heartbit` | Generate heartbeat audio patterns with configurable BPM, depth, and waveform |
| `webrtc-test-peer` | Minimal WebRTC test peer for E2E testing of web-app WebRTC functionality |

## Workflows
Available workflows in `.adi/workflows/`. Run with `adi wf <name>` or directly via `.adi/workflows/<name>.sh`.

| Workflow | Description |
|----------|-------------|
| `autodoc` | Generate API documentation for Rust crates with LLM enrichment and translations |
| `build-linux` | Cross-compile services for Linux (Docker deployment) |
| `build-plugin` | Build and install plugins locally (no registry deploy) |
| `cocoon-images` | Build and release cocoon Docker image variants |
| `commit-submodule` | Commit changes in a submodule with AI-generated messages |
| `convert-sounds` | Convert raw audio files to web-optimized MP3 and OGG formats |
| `deploy` | Deploy services to Coolify |
| `generate-agents-md` | Generate AGENTS.md with crate structure documentation |
| `lint-plugin` | Lint and validate a plugin before release |
| `patch` | Build and patch CLI binary or plugin locally (with macOS codesign) |
| `release-cli` | Build and release adi CLI to GitHub |
| `release-plugin` | Build and publish a plugin to the registry |
| `release-plugins` | Release all core plugins to the registry |
| `release` | Build and release service images (cross-compile + Docker) |
| `reset` | Reset ADI installation (remove all local data for clean reinstall) |
| `seal` | Commit and push all changes including submodules |
| `sync-theme` | Sync theme JSON to CSS + Rust outputs |

## Code Style Guidelines


- **Always use `lib-console-output`** for all terminal output — never use raw `println!`/`eprintln!`
  - Use `out_info!`, `out_success!`, `out_warn!`, `out_error!`, `out_debug!` macros for messages
  - Use `Section` for headers, `Columns`/`Table` for tabular data, `List` for bullet lists, `KeyValue` for label-value pairs
  - Use `theme::*` functions for styling (`theme::success`, `theme::error`, `theme::brand_bold`, etc.)


- **Crate structure convention:**
  ```
  crates/<component>/
    core/     # Business logic, types, traits (lib)
    http/     # REST API server - axum (bin)
    plugin/   # adi CLI plugin (cdylib)
    cli/      # Standalone CLI (bin, optional)
    mcp/      # MCP server (bin, optional)
  ```
- **Dependencies flow:** `cli` → `core` ← `http` (both depend on core)
- **Libraries** go in `crates/lib/lib-<name>/`
- **Standalone plugins** use `crates/<name>-plugin/` pattern
- **Tools** use `crates/tool-<name>/` pattern


**Additional guidelines:**
- [`comments`](docs/code-style/comments.md): Comments must add value. If a comment restates what the code
- [`plugin-sdk`](docs/code-style/plugin-sdk.md): Use `lib-plugin-prelude` for all ADI plugins.
- [`translations`](docs/code-style/translations.md): Use Mozilla Fluent (.ftl) for all user-facing strings.

