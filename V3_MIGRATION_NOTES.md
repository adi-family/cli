# V3 Migration Notes

## Plugin Categories

### Category 1: Simple CLI Plugins ✅
**Can migrate now** - Use lib-plugin-abi-v3 CLI traits

Examples:
- ✅ adi.workflow (DONE)
- ⏳ adi.audio
- ⏳ adi.linter
- ⏳ adi.coolify
- ⏳ adi.browser-debug

### Category 2: Complex CLI Plugins (CLI + HTTP + MCP)
**Can migrate now** - Use lib-plugin-abi-v3 CLI + HTTP + MCP traits

Examples:
- ⏳ adi.tasks
- ⏳ adi.indexer
- ⏳ adi.agent-loop
- ⏳ adi.knowledgebase
- ⏳ adi.api-proxy

### Category 3: Language Analyzer Plugins ⚠️
**Cannot migrate yet** - Requires lib-indexer-lang-abi v3

These plugins implement language analysis services for the indexer, not CLI commands.
They use `lib-indexer-lang-abi` which is still v2 (FFI-based).

Plugins:
- adi.lang.go
- adi.lang.python
- adi.lang.typescript
- adi.lang.rust
- adi.lang.java
- adi.lang.php
- adi.lang.lua
- adi.lang.cpp
- adi.lang.swift
- adi.lang.csharp
- adi.lang.ruby

**Action Required:** Create lib-indexer-lang-abi-v3 first, then migrate these plugins.

### Category 4: Hive Orchestration Plugins
**Can migrate now** - Use lib-plugin-abi-v3 orchestration traits

The orchestration traits are already in v3:
- Runner
- HealthCheck
- EnvProvider
- ProxyMiddleware
- ObservabilitySink
- RolloutStrategy

Plugins (31 total):
- env-* (4 plugins)
- health-* (7 plugins)
- obs-* (4 plugins)
- proxy-* (11 plugins)
- rollout-* (2 plugins)
- runner-* (3 plugins)

---

## Revised Migration Strategy

### Phase 1: CLI-Only Plugins (4 plugins)
- adi.audio
- adi.linter
- adi.coolify
- adi.browser-debug

**Time:** ~2 hours (30 min each)

### Phase 2: Complex CLI Plugins (6 plugins)
- adi.tasks
- adi.indexer
- adi.agent-loop
- adi.knowledgebase
- adi.api-proxy
- hive (main orchestrator)

**Time:** ~12-18 hours (2-3 hours each)

### Phase 3: Hive Orchestration Plugins (31 plugins)
Can batch by category since they're similar within each category.

**Time:** ~31-62 hours (1-2 hours each, or faster with batching)

### Phase 4: Language Plugins (11 plugins) - DEFERRED
**Blocked on:** lib-indexer-lang-abi v3

This is a separate project requiring:
1. Design lib-indexer-lang-abi-v3 (async traits)
2. Implement the new ABI
3. Migrate all 11 language plugins

**Estimated:** 20-30 hours total

---

## Updated Progress

**Immediately Migratable:**
- Simple CLI: 4 plugins
- Complex CLI: 6 plugins
- Hive Orchestration: 31 plugins
- **Total: 41 plugins**

**Requires Infrastructure First:**
- Language Plugins: 11 plugins (need lib-indexer-lang-abi-v3)

**Already Complete:**
- Translation plugins: 9 plugins
- adi.workflow: 1 plugin
- **Total: 10 plugins**

---

## Realistic Timeline

**Immediate (can do now):** 41 plugins = ~45-82 hours
**Future (after lang ABI v3):** 11 plugins = ~20-30 hours

**Total:** ~65-112 hours for full migration

**Recommendation:** Focus on the 41 immediately migratable plugins first, defer language plugins to a separate effort.
