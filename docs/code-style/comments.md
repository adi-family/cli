# Comments Convention

## Core Principle

**Comments must add value.** If a comment restates what the code already says, delete it.

## Rules

### 1. No Tautological Comments

Don't restate the function/struct/field name:

```rust
// BAD
/// Creates a new Foo
pub fn new() -> Self { ... }

/// The user's name
pub name: String,

/// Start the server
pub fn start_server() { ... }

// GOOD - no comment needed, code is self-explanatory
pub fn new() -> Self { ... }
pub name: String,
pub fn start_server() { ... }
```

### 2. Explain Why, Not What

```rust
// BAD - describes what the code does (obvious)
// Increment the counter
counter += 1;

// GOOD - explains why
// Offset by 1 because API uses 1-indexed pages
counter += 1;
```

### 3. Document Non-Obvious Behavior

```rust
// GOOD - explains edge case
/// Returns None if the path contains invalid UTF-8
pub fn path_str(&self) -> Option<&str>

// GOOD - explains constraint
/// Must be < 1024 (privileged port)
pub port: u16,

// GOOD - explains the choice
/// String path, not PathBuf (for serialization)
pub working_dir: Option<String>,
```

### 4. Doc Comments (`///`) for Public API

Use when explaining:
- **Behavior** that isn't obvious from the signature
- **Panics** - when and why the function panics
- **Errors** - what errors can be returned and when
- **Examples** - for complex APIs
- **Safety** - for unsafe code

```rust
/// Runs with root privileges via `adi-root` user (NOPASSWD sudo).
/// Only call after validating the plugin has permission for this command.
pub async fn sudo_run(&self, cmd: &str, args: &[String]) -> Result<Output>

/// Uses iptables (Linux) or pfctl (macOS) to redirect privileged ports.
pub async fn bind_port(&self, port: u16, target_port: u16) -> Result<()>
```

### 5. Inline Comments (`//`) Sparingly

Use only for:
- Complex algorithms that need step-by-step explanation
- Workarounds with links to issues
- Non-obvious magic values

```rust
// Workaround for https://github.com/issue/123
let timeout = Duration::from_secs(30);

// 0x1F600 = grinning face emoji start
let emoji_range = 0x1F600..0x1F64F;
```

### 6. Delete Over Rewrite

If a comment doesn't add value, remove it entirely. Don't try to make it "better" - just delete it.

## Quick Reference

| Situation | Action |
|-----------|--------|
| `/// Creates a new X` on `fn new()` | Delete |
| `/// The name` on `pub name: String` | Delete |
| `/// Start/Stop/Get X` on `fn start/stop/get_x()` | Delete |
| Explains *why* something is done | Keep |
| Documents edge cases or invariants | Keep |
| Describes platform-specific behavior | Keep |
| Links to issues or external docs | Keep |
| Security considerations | Keep |

## Examples from Codebase

**Deleted (tautological):**
```rust
/// Client for communicating with the ADI daemon  // struct name says this
/// Create a daemon client with custom socket path  // fn name says this
/// Stop a service  // fn name says this
/// Service configuration  // field type says this
```

**Kept (adds value):**
```rust
/// Number of restarts since daemon started  // clarifies scope
/// Check last_error for details  // points to related field
/// SIGKILL instead of graceful SIGTERM  // explains the bool
/// Spawns as a background task to monitor service health  // explains usage
/// Reads plugin manifests to find services with `[package.metadata.plugin.service]`  // documents format
```
