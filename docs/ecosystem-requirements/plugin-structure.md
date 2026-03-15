# Plugin Structure

## Why This Rule Exists

A consistent crate structure across all components enables predictable navigation, parallel compilation, and clean separation between business logic, transport layers, and CLI integration.

## Required Structure

```
crates/<component>/
  core/       # Business logic, types, traits (lib crate)
  http/       # REST API server with axum (bin crate)
  plugin/     # ADI CLI plugin (cdylib crate)
  cli/        # Standalone CLI (bin crate, optional)
```

## Rules

### 1. Core Owns the Logic

All business logic, types, and traits live in `core/`. No transport details, no CLI concerns, no plugin ABI. Pure domain logic.

```
core/src/
  lib.rs        # Public API
  types.rs      # Domain types
  storage.rs    # Storage trait + implementations
  error.rs      # Error types
```

### 2. Dependencies Flow Inward

```
plugin/ ──> core/ <── http/
  cli/ ──> core/
```

Both `http/` and `plugin/` depend on `core/`, never on each other. `core/` depends on no sibling crate.

### 3. Plugin Uses `lib-plugin-prelude`

```rust
use lib_plugin_prelude::*;

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(MyPlugin)
}
```

Implement `Plugin` (required) plus any optional traits: `CliCommands`, `GlobalCommands`, `HttpRoutes`, `WebRtcHandlers`, `DaemonService`.

### 4. CLI Commands Use Derive Macros

```rust
#[derive(CliArgs)]
struct BuildArgs {
    /// Target architecture
    #[arg(long, default_value = "native")]
    target: String,

    /// Enable release mode
    #[arg(long)]
    release: bool,
}
```

### 5. Libraries Go in `crates/_lib/`

Shared code used by multiple components lives in `crates/_lib/lib-<name>/`. Never put shared logic in a component's `core/` if other components need it.

### 6. Standalone Plugins Use `crates/<name>-plugin/`

Plugins not tied to a component (e.g., `embed-plugin`, `llm-uzu-plugin`) use the `-plugin` suffix pattern.

### 7. Tools Use `crates/tool-<name>/`

Standalone tools that aren't plugins follow the `tool-` prefix pattern.

## The Test

*"Can I compile and test `core/` without any transport layer or CLI dependency?"* If no, extract the coupled logic.
