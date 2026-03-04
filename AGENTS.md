# ADI Crate Structure

> Auto-generate with: `adi wf generate-agents-md`

## User-Facing Components
Components with plugin for `adi` CLI integration.

| Crate | Structure | Description |
|-------|-----------|-------------|
| `agent-loop` | core,plugin | Core library for ADI Agent Loop - autonomous LLM agent with tool use |
| `analytics` | core,plugin | Core analytics types, events, errors, and migrations for ADI platform |
| `auth` | core,http,plugin,cli | Core library for ADI Auth - email-based passwordless authentication |
| `browser-debug` | core,plugin | Core library for ADI Browser Debug - types and signaling client |
| `cocoon-spawner` | core,plugin | Core library for ADI Cocoon Spawner - Docker-based cocoon lifecycle management via signaling |
| `credentials` | core,http,plugin,cli | Core library for ADI Credentials API - secure credentials storage |
| `flags` | core,plugin | Core library for ADI file flag tracking |
| `hive` | core,plugin | Hive core library - local service orchestration business logic |
| `indexer` | core,http,plugin,cli | Core indexer library for ADI - parsing, storage, search |
| `knowledgebase` | core,plugin,cli | Core knowledgebase library for ADI - graph DB, embedding storage, semantic search |
| `linter` | core,plugin | Core library for ADI Linter - language-agnostic linting with external/plugin/command rules |
| `llm-proxy` | core,plugin | ADI LLM Proxy - Core library for LLM API proxying with BYOK/Platform modes |
| `monaco-editor` | plugin | ADI Monaco Editor plugin - web-only code editor |
| `mux` | core,http,plugin | Core library for ADI Mux - HTTP request fan-out multiplexer |
| `payment` | core,plugin | Core library for ADI Payment API - checkout sessions, subscriptions, and webhook handling |
| `platform` | core,http,plugin,cli | Core library for ADI Platform API - business logic, types, and storage |
| `registry` | core,plugin | Core library for ADI plugin registry - storage and business logic |
| `signaling` | core,plugin | Core library for signaling server — types, state, security, token validation, and utility functions |
| `tasks` | core,plugin | Core library for ADI Tasks - task management with dependency graphs |
| `tools` | core,plugin | Core library for tool index - searchable CLI tool discovery |
| `video` | core,http,plugin | Core library for ADI Video - programmatic video rendering with FFmpeg |
| `workflow` | plugin | ADI Workflow plugin - run shell workflows defined in TOML files |

## Backend Services
HTTP services without CLI plugin.

| Crate | Structure | Description |
|-------|-----------|-------------|
| `executor` | core,http | Core library for Docker-based task execution — types, orchestration, Docker client, output handlers, job store |

## Libraries
Shared libraries in `crates/_lib/`.

| Library | Purpose |
|---------|---------|
| `lib-animation` | UI animation utilities - easing functions, spring physics, animation manager |
| `lib-cli-common` | Common CLI utilities for ADI tools |
| `lib-client` | - |
| `lib-console-output` | Console output abstraction with support for text and JSON stream modes |
| `lib-daemon-client` | Client library for the ADI daemon IPC protocol |
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
| `lib-migrations` | Database migration framework for ADI components |
| `lib-migrations-core` | Generic migration framework - bring your own storage and actions |
| `lib-migrations-sql` | SQL migrations built on lib-migrations-core |
| `lib-misc-color` | Unified color type with lazy conversion - RGB, RGBA, Hex support |
| `lib-plugin-abi-v3` | Unified plugin ABI for ADI ecosystem (v3 - native async traits) |
| `lib-plugin-host` | Plugin host for loading and managing v3 plugins |
| `lib-plugin-manifest` | Plugin manifest parsing (plugin.toml and package.toml) |
| `lib-plugin-prelude` | Plugin prelude for ADI ecosystem - re-exports SDK macros and runtime types |
| `lib-plugin-sdk` | Plugin SDK for ADI ecosystem - proc macros for simplified plugin development |
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

## Workflows
Available workflows in `.adi/workflows/`. Run with `adi wf <name>` or directly via `.adi/workflows/<name>.sh`.

| Workflow | Description |
|----------|-------------|
| `autodoc` | Generate API documentation for Rust crates with LLM enrichment and translations |
| `build-plugin` | Build and install plugins locally (no registry deploy) |
| `clean-install` | Reset ADI installation (remove all local data for clean reinstall) |
| `cocoon-images` | Build and release cocoon Docker image variants |
| `convert-sounds` | Convert raw audio files to web-optimized MP3 and OGG formats |
| `deploy` | Deploy services to Coolify |
| `generate-agents-md` | Generate AGENTS.md and CLAUDE.md with crate structure documentation |
| `lint-plugin` | Lint and validate a plugin before release |
| `patch` | Build and patch CLI binary or plugin locally (with macOS codesign) |
| `release` | Release CLI binary or plugin |
| `seal` | Commit and push all changes including submodules |
| `sync-theme` | Sync theme JSON to CSS + Rust outputs |

## Code Style Guidelines


- **Always use `lib-console-output`** for all terminal output -- never use raw `println!`/`eprintln!`
  - Use `out_info!`, `out_success!`, `out_warn!`, `out_error!`, `out_debug!` macros for messages
  - Use `Section` for headers, `Columns`/`Table` for tabular data, `List` for bullet lists, `KeyValue` for label-value pairs
  - Use `theme::*` functions for styling (`theme::success`, `theme::error`, `theme::brand_bold`, etc.)

- **KISS**: Simple code over clever code. Code exists for humans. Don't import enterprise patterns from other languages. If you need a comment to explain what code does, simplify the code instead.

- **DRY**: Extract repeated logic, but wait for the third occurrence. Premature abstraction creates worse coupling than duplication. Use traits and generics as primary abstraction tools.

- **YAGNI**: Don't implement speculative features. Rust's traits eliminate many OO patterns (Strategy, Factory, Observer). Refactoring is cheap -- add abstraction when you need it.

- **Loose coupling**: Depend on traits, not concrete types. Accept `impl Trait` or generics. Use dependency injection. Split large structs for independent borrowing and testing.

- **Small crates**: One responsibility per crate. Core logic in libraries, thin wrappers for CLI/HTTP/plugin. Enables parallel compilation and code reuse.

- **Borrowed types**: Prefer `&str` over `&String`, `&[T]` over `&Vec<T>`. More flexible for callers, fewer indirections.

- **Newtype pattern**: Wrap primitives in single-field structs for type safety. `Miles(f64)` vs `Kilometers(f64)` catches bugs at compile time, zero runtime cost.

- **Custom types over bool**: Use enums (`Size::Small`) instead of booleans. Self-documenting, extensible, catches argument-order bugs.

- **Generics**: Accept `impl IntoIterator<Item = T>` over `&Vec<T>`. Express minimal requirements, accept maximum inputs.

- **Builder pattern**: For types with many optional parameters. Named setters, defaults, validation. Prefer non-consuming builders (`&mut self`) for flexibility.

- **Avoid Deref abuse**: `Deref` is for smart pointers, not inheritance. Use composition + explicit delegation or traits instead.

- **Avoid Clone abuse**: Don't sprinkle `.clone()` to silence borrow checker. Restructure borrows, scope them tightly, or decompose structs. Clone hides design problems.

- **Extensibility**: Use `#[non_exhaustive]` or private fields to allow adding fields/variants without breaking changes.

- **Error handling**: Specific enum variants, preserved error chains (`#[source]`), actionable context (paths, values). Document with `# Errors` section.

- **Common traits**: Always implement `Debug`. Add `Clone`, `PartialEq`, `Hash`, `Default` where meaningful. Don't block `Send`/`Sync` accidentally.

- **Documentation**: First line = summary. Add `# Examples`, `# Errors`, `# Panics`, `# Safety` sections as needed. Use `?` in examples, not `unwrap()`.

- **Module structure**: When a subdirectory contains only 2 files (`mod.rs` + one impl), flatten to sibling files: `foo/mod.rs` + `foo/bar.rs` → `foo.rs` + `foo_bar.rs`. Use `#[path = "foo_bar.rs"] mod bar;`. Subdirectories justified with 3+ files.


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
- **Libraries** go in `crates/_lib/lib-<name>/`
- **Standalone plugins** use `crates/<name>-plugin/` pattern
- **Tools** use `crates/tool-<name>/` pattern



**Additional guidelines:**
- [`comments`](docs/code-style/comments.md): Comments must add value.
- [`plugin-sdk`](docs/code-style/plugin-sdk.md): Schema generation
- [`rust-avoid-clone-abuse`](docs/code-style/rust-avoid-clone-abuse.md): Restructure borrows:
- [`rust-avoid-deref-abuse`](docs/code-style/rust-avoid-deref-abuse.md): Explicit delegation:
- [`rust-borrowed-types`](docs/code-style/rust-borrowed-types.md): -
- [`rust-builder`](docs/code-style/rust-builder.md): Non-consuming (preferred)
- [`rust-common-traits`](docs/code-style/rust-common-traits.md): -
- [`rust-coupling`](docs/code-style/rust-coupling.md): -
- [`rust-custom-types`](docs/code-style/rust-custom-types.md): Self-documenting code
- [`rust-documentation`](docs/code-style/rust-documentation.md): First line
- [`rust-dry`](docs/code-style/rust-dry.md): -
- [`rust-error-handling`](docs/code-style/rust-error-handling.md): Use enums for error variants:
- [`rust-extensibility`](docs/code-style/rust-extensibility.md): Benefits:
- [`rust-generics`](docs/code-style/rust-generics.md): Benefits:
- [`rust-kiss`](docs/code-style/rust-kiss.md): -
- [`rust-newtype`](docs/code-style/rust-newtype.md): Zero-cost
- [`rust-small-crates`](docs/code-style/rust-small-crates.md): Benefits:
- [`rust-yagni`](docs/code-style/rust-yagni.md): -
- [`translations`](docs/code-style/translations.md): Use Mozilla Fluent (.ftl) for all user-facing strings.

