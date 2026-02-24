plugin-sdk, migration, detection, rust, adi-plugins

## Plugin SDK Version Detection

### Old plugin (needs migration)

**Cargo.toml — any of these signals old:**
```toml
lib-plugin-abi-v3 = { path = "..." }   # direct abi dep
tokio = { ... }                         # explicit tokio (comes from prelude now)
async-trait = "..."                     # explicit async-trait (comes from prelude now)
# missing api_version in compatibility
```

**src/lib.rs — any of these signals old:**
```rust
use lib_plugin_abi_v3::{...};           # explicit abi import
type CmdResult = Result<String, String>; # manual alias (prelude exports it)
PluginResult<()>                        # old type alias name
PluginResult<CliResult>                 # old type alias name
CliCommand { usage: "...".to_string() } # usage field removed
PluginMetadata { id: ..., name: ... }   # struct literal instead of builder
async fn shutdown(&self) -> ... { Ok(()) } # no-op shutdown (has default impl)
plugin_create_cli                       # still valid, keep if present
```

### New plugin (correct)

**Cargo.toml:**
```toml
lib-plugin-prelude = { path = "../../lib/lib-plugin-prelude" }

[package.metadata.plugin.compatibility]
api_version = 3
```

**src/lib.rs:**
```rust
use lib_plugin_prelude::*;              # single glob import

PluginMetadata::new("id", "Name", env!("CARGO_PKG_VERSION"))
    .with_type(PluginType::Core)
    .with_author("...")
    .with_description("...")

CliCommand {
    args: vec![
        CliArg::positional(0, "name", CliArgType::String, true),
        CliArg::optional("--flag", CliArgType::String),
    ],
    ...
}

async fn init(&mut self, ctx: &PluginContext) -> Result<()>
async fn run_command(&self, ctx: &CliContext) -> Result<CliResult>
```

## Quick grep to find old plugins

```bash
# Find plugins still on direct abi dep
grep -rl "lib-plugin-abi-v3" crates --include="Cargo.toml"

# Find plugins with old usage field
grep -rl "usage:" crates --include="*.rs"

# Find plugins with old PluginResult type
grep -rl "PluginResult" crates --include="*.rs"
```
