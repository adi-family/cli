# Plugin ABI v3: Next Steps & Resume Guide

**Last Updated:** 2026-01-31
**Status:** 10% Complete (9/86 plugins migrated)

---

## Current State

### ✅ Completed (100%)

**Infrastructure:**
- [x] lib-plugin-abi-v3 crate (all traits implemented)
- [x] lib-plugin-host v3 loader (LoadedPluginV3, PluginManagerV3)
- [x] Migration guide (docs/MIGRATION_V2_TO_V3.md)
- [x] Design docs (docs/lib-plugin-abi-v3-design.md)
- [x] Ecosystem overview docs

**Migrations:**
- [x] All 9 translation plugins migrated to v3 ✅
  - adi-cli-lang-en, zh-CN, uk-UA, es-ES, fr-FR, de-DE, ja-JP, ko-KR, ru-RU

**Commits:**
```bash
b9c7647 📊 docs: update adoption status (9/86 plugins complete)
68c9074 🔗 chore: update adi-cli submodule (all translation plugins)
b2df9b4 🚀 migrate: convert all translation plugins to v3 ABI
c091ee6 📊 docs: add v3 adoption status tracking
efd9375 🔗 chore: update adi-cli submodule (first v3 migration)
cb80956 📚 docs: add comprehensive v2 → v3 migration guide
cbe0d9c 🔗 chore: update lib-plugin-host submodule
11029f3 ✨ feat: complete lib-plugin-abi-v3 trait implementations
a26d991 ✨ feat: introduce lib-plugin-abi-v3 unified plugin architecture
```

---

## 🔲 In Progress (Not Complete)

### Task #8: Update adi-cli to use PluginManagerV3

**Status:** Started but not finished
**Priority:** HIGH (blocker for testing v3 plugins)

**What's needed:**
1. Add PluginManagerV3 to PluginRuntime
2. Detect plugin version (v2 vs v3) from manifest
3. Load v3 plugins using LoadedPluginV3
4. Dispatch CLI commands to v3 plugins
5. Support both v2 and v3 during transition

**Files to modify:**
- `crates/adi-cli/src/plugin_runtime.rs`
- `crates/adi-cli/Cargo.toml` (add lib-plugin-abi-v3 dep)

**Implementation sketch:**
```rust
pub struct PluginRuntime {
    host: Arc<RwLock<PluginHost>>,           // v2 loader
    manager_v3: Arc<RwLock<PluginManagerV3>>, // v3 loader (NEW)
    config: RuntimeConfig,
}

impl PluginRuntime {
    pub async fn load_plugin(&mut self, manifest: &PluginManifest) -> Result<()> {
        match manifest.compatibility.api_version {
            2 => self.load_v2_plugin(manifest).await?,
            3 => self.load_v3_plugin(manifest).await?,
            _ => return Err(anyhow!("Unsupported API version")),
        }
        Ok(())
    }

    async fn load_v3_plugin(&mut self, manifest: &PluginManifest) -> Result<()> {
        let plugin_dir = self.config.plugins_dir.join(&manifest.plugin.id);
        let loaded = LoadedPluginV3::load(manifest.clone(), &plugin_dir).await?;
        self.manager_v3.write().unwrap().register(loaded)?;
        Ok(())
    }
}
```

**Testing:**
```bash
# After implementing, test with migrated translation plugin:
cd crates/adi-cli
cargo build --release
./target/release/adi plugin list  # Should show v3 plugins
```

---

## 🎯 Immediate Next Steps (Priority Order)

### Step 1: Finish adi-cli v3 Integration (2-3 hours)

**Goal:** Make adi-cli load and run v3 plugins

**Tasks:**
1. Update `crates/adi-cli/Cargo.toml`:
   ```toml
   [dependencies]
   lib-plugin-abi-v3 = { path = "../lib/lib-plugin-abi-v3" }
   lib-plugin-host = { path = "../lib/lib-plugin-host" }
   ```

2. Modify `src/plugin_runtime.rs`:
   - Add `PluginManagerV3` field
   - Implement dual loading (v2 + v3)
   - Add version detection
   - Update CLI command dispatch

3. Test loading v3 translation plugins

4. Commit:
   ```bash
   git add crates/adi-cli/
   git commit -m "✨ feat: add v3 plugin loading to adi-cli"
   ```

---

### Step 2: Migrate Core CLI Plugins (5-10 hours)

**Order:** Start simple, increase complexity

#### 2.1: Simple Plugins (1-2 hours each)

**adi.workflow** (simplest - just CLI commands)
```bash
cd crates/adi-workflow/plugin
# Follow migration guide
# Update Cargo.toml, lib.rs, plugin.toml
```

**adi.embed** (simple - just service)
```bash
cd crates/adi-embed
# Migrate to Plugin trait
```

#### 2.2: Medium Complexity (2-3 hours each)

**adi.tasks** (CLI + HTTP + MCP)
- Location: `crates/adi-tasks/plugin/`
- Services: CliCommands, HttpRoutes, McpTools
- Test: Task CRUD operations work

**adi.indexer** (CLI + HTTP + MCP)
- Location: `crates/adi-indexer/plugin/`
- Services: CliCommands, HttpRoutes, McpTools
- Test: Code search works

**adi.agent-loop** (CLI + HTTP)
- Location: `crates/adi-agent-loop/plugin/`
- Services: CliCommands, HttpRoutes
- Test: Agent execution works

#### 2.3: Lower Priority (1-2 hours each)

- adi.knowledgebase
- adi.api-proxy
- adi.audio
- adi.coolify
- adi.linter

---

### Step 3: Migrate Hive Orchestration Plugins (10-20 hours)

**Order:** Bundled first (used most), then external

#### 3.1: Hive Bundled Plugins (critical path)

**hive.runner.docker** (most important)
```bash
cd crates/hive/plugins/hive-runner-docker
# Implement Runner trait from lib-plugin-abi-v3
# Test: Docker container lifecycle
```

**Health checks:**
- hive.health.http
- hive.health.tcp

**Proxy middleware:**
- hive.proxy.cors
- hive.proxy.rate-limit

**Observability:**
- hive.obs.stdout
- hive.obs.file

**Environment:**
- hive.env.dotenv

#### 3.2: Update Hive Core

After plugins migrated:
```bash
cd crates/hive/core
# Update to use PluginManagerV3
# Similar to adi-cli integration
```

---

### Step 4: Remaining Plugins (5-10 hours)

**Language Analysis Plugins (11):**
- adi.lang.rust, python, typescript, go, java, etc.
- Location: Look in `crates/adi-lang/` or similar
- Pattern: Similar structure, batch migrate

**Extension Plugins (5+):**
- adi.llm.uzu (Apple Silicon LLM)
- adi.browser-debug
- Others as discovered

---

## 📋 Migration Checklist Template

For each plugin:

```bash
# 1. Navigate to plugin
cd crates/<plugin>/plugin/

# 2. Update Cargo.toml
# - lib-plugin-abi → lib-plugin-abi-v3
# - Remove abi_stable
# - Add async-trait, tokio (if not present)
# - Bump version to 3.0.0

# 3. Update src/lib.rs
# - Remove: use abi_stable::*, lib_plugin_abi::*
# - Add: use lib_plugin_abi_v3::*
# - Replace plugin_entry() → plugin_create()
# - Implement Plugin trait
# - Implement service traits (CliCommands, etc.)
# - Remove all FFI types (RString → String, etc.)

# 4. Update plugin.toml
# - version = "3.0.0"
# - Add: [compatibility] api_version = 3

# 5. Build and test
cargo build
# Test functionality

# 6. Commit
git add .
git commit -m "🚀 migrate: <plugin-name> to v3 ABI"
```

---

## 🚀 How to Resume

### Quick Start

```bash
# 1. Pull latest changes
cd /Users/mgorunuch/projects/adi-family/cli
git pull
git submodule update --recursive

# 2. Check current status
cat V3_ADOPTION_STATUS.md

# 3. Continue with Step 1 (adi-cli integration)
cd crates/adi-cli
# Follow "Step 1" above

# 4. Or jump to plugin migration
cd crates/adi-workflow/plugin  # Start with simplest
# Follow migration checklist
```

---

## 📊 Progress Tracking

Update `V3_ADOPTION_STATUS.md` after each plugin:

```bash
# After migrating a plugin:
vim V3_ADOPTION_STATUS.md
# Update status from 🔲 Pending → ✅ Migrated
# Update progress percentage

git add V3_ADOPTION_STATUS.md
git commit -m "📊 docs: mark <plugin> as migrated"
```

---

## ⚠️ Known Issues / Blockers

**None currently!** 🎉

All infrastructure is in place. Pure execution phase.

---

## 🎯 Success Criteria

- [ ] adi-cli can load v3 plugins (Task #8)
- [ ] All 10 core CLI plugins migrated
- [ ] All 32 Hive plugins migrated
- [ ] All 11 language plugins migrated
- [ ] All plugins build without errors
- [ ] Integration tests pass
- [ ] Performance benchmarks show improvement
- [ ] Documentation updated

---

## 📞 Key Commands Reference

### Build Plugin
```bash
cd crates/<plugin>/plugin
cargo build --release
```

### Test Plugin Loading
```bash
cd crates/adi-cli
cargo run -- plugin list
cargo run -- <command>  # Test CLI command
```

### Commit Pattern
```bash
git add crates/<plugin>/
git commit -m "🚀 migrate: <plugin-name> to v3 ABI"
```

### Batch Migration (if similar structure)
```bash
# Like we did for translation plugins
for plugin in plugin1 plugin2 plugin3; do
  cd $plugin
  # Apply changes
  cd ..
done
```

---

## 📚 Reference Documents

- **Migration Guide:** `docs/MIGRATION_V2_TO_V3.md`
- **Design Doc:** `docs/lib-plugin-abi-v3-design.md`
- **Status Tracker:** `V3_ADOPTION_STATUS.md`
- **This File:** `V3_NEXT_STEPS.md`

---

## 💡 Tips

1. **Start simple:** Migrate workflow/embed plugins first (easiest)
2. **Test early:** After each plugin, test it loads and runs
3. **Commit often:** One commit per plugin (easy rollback)
4. **Batch similar:** Translation plugins showed 8x speedup via batching
5. **Use migration guide:** Don't memorize, reference the guide
6. **Copy-paste friendly:** Translation plugins used en-US as template

---

## 🏁 Estimated Time Remaining

| Category | Plugins | Est. Time | Status |
|----------|---------|-----------|--------|
| adi-cli integration | 1 task | 2-3 hours | 🔲 Next |
| Core CLI plugins | 10 | 5-10 hours | 🔲 After |
| Hive plugins | 32 | 10-20 hours | 🔲 After |
| Language plugins | 11 | 5-6 hours | 🔲 After |
| Extension plugins | 5+ | 3-5 hours | 🔲 Last |

**Total remaining:** 25-44 hours of focused work

**Realistic timeline:** 1-2 weeks part-time, 3-5 days full-time

---

**Last checkpoint:** Translation plugins complete (9/86)
**Next milestone:** Core CLI integration + first core plugin
**Final goal:** 86/86 plugins migrated, v2 deprecated

Ready to resume! 🚀
