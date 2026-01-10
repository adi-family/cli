# Translation Plugin System Design

i18n, fluent, plugin-architecture, modular-translations, language-packs

## Overview
Translation system using **separate translation plugins** following the naming pattern `[plugin-id].[language-code]`. Each language is distributed as an independent plugin, allowing users to install only the languages they need.

## Architecture

```
Plugin System
│
├─ adi.tasks (main plugin, no translations)
├─ adi.tasks.en-US (English translation plugin)
├─ adi.tasks.zh-CN (Chinese translation plugin)
└─ adi.tasks.uk-UA (Ukrainian translation plugin)
│
├─ adi.indexer (main plugin)
├─ adi.indexer.en-US
├─ adi.indexer.zh-CN
└─ adi.indexer.uk-UA
│
└─ adi.agent-loop (main plugin)
   ├─ adi.agent-loop.en-US
   ├─ adi.agent-loop.zh-CN
   └─ adi.agent-loop.uk-UA
```

## Plugin Naming Convention

```
[plugin-id].[language-code]

Examples:
- adi.tasks.en-US          (English - United States)
- adi.tasks.zh-CN          (Chinese - Simplified)
- adi.tasks.zh-TW          (Chinese - Traditional)
- adi.tasks.uk-UA          (Ukrainian)
- adi.tasks.ja-JP          (Japanese)
- adi.tasks.fr-FR          (French)
- adi.tasks.de-DE          (German)
- adi.tasks.es-ES          (Spanish - Spain)
- adi.tasks.pt-BR          (Portuguese - Brazil)
```

## Translation Plugin Manifest

Example: `adi-tasks-lang-en/plugin.toml`

```toml
[plugin]
id = "adi.tasks.en-US"
name = "ADI Tasks - English"
version = "1.0.0"
type = "translation"
author = "ADI Team"
description = "English translations for ADI Tasks plugin"
min_host_version = "0.8.0"

# Translation metadata
[translation]
translates = "adi.tasks"        # Which plugin this translates
language = "en-US"               # Language code
language_name = "English (United States)"
namespace = "tasks"              # Namespace for keys

# Provide translation service
[[provides]]
id = "adi.i18n.tasks.en-US"
version = "1.0.0"
description = "English translations for tasks plugin"

[tags]
categories = ["i18n", "translation"]

[binary]
name = "adi_tasks_lang_en"
```

## Translation Plugin Structure

```
adi-tasks-lang-en/
├── Cargo.toml
├── plugin.toml              # Translation metadata
├── src/
│   ├── lib.rs              # Plugin implementation
│   └── service.rs          # Translation service
└── messages.ftl            # Fluent translation file
```

## Fluent Translation File Format

Example: `messages.ftl`

```fluent
# Commands
cmd-list = List all tasks
cmd-add = Add a new task
cmd-show = Show task details
cmd-complete = Mark task as completed
cmd-delete = Delete a task

# Messages
task-created = ✅ Task created: {$name}
task-completed = ✨ Task "{$name}" completed in {$duration}
task-failed = ❌ Task failed: {$error}
task-not-found = Task not found: {$id}
task-deleted = 🗑️  Task "{$name}" deleted

# Status
status-pending = Pending
status-running = Running
status-completed = Completed
status-failed = Failed

# Errors
error-invalid-id = Invalid task ID: {$id}
error-dependency-cycle = Dependency cycle detected
error-database = Database error: {$details}

# DAG
dag-title = Task Dependency Graph
dag-no-tasks = No tasks to display
dag-critical-path = Critical Path: {$duration}
```

## Translation Service Interface

Translation plugins provide a service with two methods:

### Service ID Format
```
adi.i18n.[namespace].[language-code]

Examples:
- adi.i18n.tasks.en-US
- adi.i18n.tasks.zh-CN
- adi.i18n.indexer.uk-UA
```

### Service Methods

**1. `get_messages()`**
- Returns: Fluent .ftl file content as string
- Used to load translation messages into FluentBundle

**2. `get_metadata()`**
- Returns: JSON with translation metadata
```json
{
  "plugin_id": "adi.tasks",
  "language": "en-US",
  "language_name": "English (United States)",
  "namespace": "tasks",
  "version": "1.0.0"
}
```

## lib-i18n-core Library

Core translation library that discovers and manages translations from plugins.

### Key Responsibilities
- Scan enabled plugins for translation services (`adi.i18n.*`)
- Load Fluent messages from translation plugins
- Provide translation lookup with fallback chain: current lang → en-US → key
- Namespace isolation (e.g., "tasks.task-created")

### API

```rust
use lib_i18n_core::I18n;

// Initialize with service registry
let mut i18n = I18n::new(service_registry);
i18n.discover_translations()?;
i18n.set_language("zh-CN")?;

// Get translations
let msg = i18n.get("tasks.cmd-list");
let msg = i18n.get_with_args("tasks.task-created",
    [("name", "Build project")].into_iter().collect());
```

### Global t!() Macro

```rust
use lib_i18n_core::t;

println!("{}", t!("tasks.cmd-list"));
println!("{}", t!("tasks.task-created", "name" => "Build project"));
println!("{}", t!("tasks.error-not-found", "id" => "123"));
```

## Usage in Applications

```rust
// crates/adi-cli/src/main.rs
use lib_i18n_core::I18n;
use lib_plugin_host::PluginHost;

fn main() -> Result<()> {
    let mut plugin_host = PluginHost::new()?;

    // Enable main plugins
    plugin_host.enable("adi.tasks")?;
    plugin_host.enable("adi.indexer")?;

    // Detect user language
    let user_lang = std::env::var("LANG")
        .ok()
        .and_then(|l| l.split('.').next().map(|s| s.to_string()))
        .unwrap_or_else(|| "en-US".to_string());

    // Auto-enable translation plugins
    for plugin_id in ["adi.tasks", "adi.indexer"] {
        let translation_id = format!("{}.{}", plugin_id, user_lang);

        if plugin_host.enable(&translation_id).is_ok() {
            println!("✅ Loaded {} translations", user_lang);
        } else {
            // Fallback to English
            plugin_host.enable(&format!("{}.en-US", plugin_id)).ok();
        }
    }

    // Initialize i18n
    let mut i18n = I18n::new(plugin_host.service_registry());
    i18n.discover_translations()?;
    i18n.set_language(&user_lang)?;

    // Make globally available
    lib_i18n_core::init_global(i18n);

    // Use translations
    println!("{}", t!("tasks.cmd-list"));

    Ok(())
}
```

## Plugin Installation

```bash
# Install main plugin
adi plugin install adi.tasks

# Install specific language
adi plugin install adi.tasks.en-US
adi plugin install adi.tasks.zh-CN
adi plugin install adi.tasks.uk-UA

# Install all at once
adi plugin install adi.tasks adi.tasks.en-US adi.tasks.zh-CN

# Search for available translations
adi plugin search adi.tasks.

# Output:
# adi.tasks.en-US - English translations
# adi.tasks.zh-CN - 简体中文
# adi.tasks.uk-UA - Українська
# adi.tasks.ja-JP - 日本語
# adi.tasks.fr-FR - Français
```

## Translation Lookup Flow

```
User runs: adi tasks list
         ↓
    adi-cli main()
         ↓
    Enable plugins:
      ✅ adi.tasks (main)
      ✅ adi.tasks.zh-CN (detected from LANG=zh-CN)
         ↓
    I18n::new(service_registry)
         ↓
    i18n.discover_translations()
      → Finds service: adi.i18n.tasks.zh-CN
      → Calls: get_metadata() → {namespace: "tasks", language: "zh-CN"}
      → Calls: get_messages() → "cmd-list = 列出所有任务\n..."
      → Builds FluentBundle
         ↓
    t!("tasks.cmd-list")
      → namespace="tasks", lang="zh-CN", key="cmd-list"
      → Returns: "列出所有任务"
         ↓
    Output: 列出所有任务
```

## Fallback Chain

When looking up a translation key:

1. **Try current language**: e.g., `zh-CN`
2. **Fallback to English**: `en-US` (if current lang is not English)
3. **Fallback to key**: Return the key itself (e.g., "tasks.cmd-list")

This ensures the application never crashes due to missing translations.

## Benefits

| Benefit | Description |
|---------|-------------|
| 🎯 **Modular** | Install only the languages you need |
| 📦 **Small size** | Each translation plugin ~50KB |
| 🔄 **Independent updates** | Update translations without touching main plugin |
| 🌍 **Community-driven** | Anyone can publish translation plugins |
| 🚀 **Zero overhead** | Uninstalled languages = zero disk/memory usage |
| 🔌 **Plugin-native** | Uses existing plugin infrastructure |
| 📊 **Discoverable** | `adi plugin search adi.tasks.` shows all translations |
| ✅ **Standards-based** | Uses Fluent (Mozilla's i18n standard) |

## Creating a New Translation Plugin

### 1. Create plugin structure
```bash
cd crates
cargo new --lib adi-tasks-lang-fr
cd adi-tasks-lang-fr
```

### 2. Write plugin.toml
```toml
[plugin]
id = "adi.tasks.fr-FR"
name = "ADI Tasks - Français"
version = "1.0.0"
type = "translation"

[translation]
translates = "adi.tasks"
language = "fr-FR"
language_name = "Français"
namespace = "tasks"

[[provides]]
id = "adi.i18n.tasks.fr-FR"
version = "1.0.0"

[binary]
name = "adi_tasks_lang_fr"
```

### 3. Create messages.ftl
Copy from `adi-tasks-lang-en/messages.ftl` and translate all strings.

### 4. Implement service
Copy implementation from `adi-tasks-lang-en/src/` (service.rs and lib.rs are generic, just update service IDs).

### 5. Test
```bash
cargo build --release
adi plugin install ./crates/adi-tasks-lang-fr
LANG=fr-FR adi tasks list
```

### 6. Publish
```bash
./scripts/publish-plugin.sh adi-tasks-lang-fr
```

## Implementation Roadmap

**Phase 1: Core Infrastructure**
- [ ] Add `[translation]` section to `lib-plugin-manifest`
- [ ] Create `lib-i18n-core` with Fluent integration
- [ ] Implement service discovery and loading
- [ ] Add `t!()` macro and global instance
- [ ] Write tests

**Phase 2: Translation Plugins**
- [ ] Create `adi-tasks-lang-en` (English baseline)
- [ ] Create `adi-tasks-lang-zh` (Chinese)
- [ ] Create `adi-tasks-lang-uk` (Ukrainian)
- [ ] Create template for new translation plugins

**Phase 3: Integration**
- [ ] Update `adi-cli` to initialize i18n
- [ ] Add `--lang` CLI flag
- [ ] Auto-detect from `LANG` environment variable
- [ ] Convert all CLI output to use `t!()`

**Phase 4: Expand**
- [ ] Add translations for other plugins (indexer, agent-loop, etc.)
- [ ] Add more languages (Japanese, French, German, Spanish, etc.)
- [ ] Create contribution guide for translators
- [ ] Set up translation validation CI

## Technical Details

### Dependencies

**lib-i18n-core:**
```toml
[dependencies]
fluent = "0.16"
fluent-bundle = "0.15"
unic-langid = "0.9"
lib-plugin-abi = { path = "../lib-plugin-abi" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
```

**Translation plugin:**
```toml
[dependencies]
lib-plugin-abi = { path = "../lib-plugin-abi" }
serde_json = "1.0"
```

### File Sizes

- Translation plugin binary: ~100KB (static)
- messages.ftl file: ~5-10KB per plugin
- Total per language: ~110KB

### Performance

- Translation lookup: O(1) HashMap lookup
- Fluent formatting: ~1-5μs per message
- Plugin loading: ~10ms for all translations
- Memory: ~500KB per loaded language

## References

- Fluent syntax: https://projectfluent.org/
- Plugin system: `crates/lib-plugin-abi/`
- Service registry: `crates/lib-plugin-host/src/service_registry.rs`
