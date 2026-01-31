# V3 Migration Progress Report

**Date:** 2026-01-31
**Status:** Partially Complete
**Approach:** Automated batch migration + manual fixes

---

## ✅ Completed (100%)

### Infrastructure & Tooling
- [x] lib-plugin-abi-v3 crate (all traits)
- [x] lib-plugin-host v3 loader (LoadedPluginV3, PluginManagerV3)
- [x] adi-cli v3 integration (dual v2/v3 loader)
- [x] Migration guide (docs/MIGRATION_V2_TO_V3.md)
- [x] Batch migration scripts (.adi/scripts/batch-migrate-plugins.sh)
- [x] Plugin templates (.adi/templates/v3-cli-only-plugin.rs)

### Fully Migrated Plugins (10)
1. ✅ adi-cli-lang-en
2. ✅ adi-cli-lang-zh-CN
3. ✅ adi-cli-lang-uk-UA
4. ✅ adi-cli-lang-es-ES
5. ✅ adi-cli-lang-fr-FR
6. ✅ adi-cli-lang-de-DE
7. ✅ adi-cli-lang-ja-JP
8. ✅ adi-cli-lang-ko-KR
9. ✅ adi-cli-lang-ru-RU
10. ✅ adi.workflow

---

## 🚧 Partially Migrated (Configs Ready, lib.rs Needs Fixes)

### Core CLI Plugins (4)
- 🔧 adi.audio - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.linter - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.coolify - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.browser-debug - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳

### Language Analysis Plugins (11)
- 🔧 adi.lang.go - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.python - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.typescript - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.rust - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.java - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.php - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.lua - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.cpp - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.swift - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.csharp - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳
- 🔧 adi.lang.ruby - Cargo.toml ✅, plugin.toml ✅, lib.rs ⏳

**Total Partially Complete:** 15 plugins (30% of all plugins)

---

## 🔲 Not Started

### Complex Core Plugins (7)
- ⏳ adi.tasks - CLI + HTTP + MCP
- ⏳ adi.indexer - CLI + HTTP + MCP
- ⏳ adi.agent-loop - CLI + HTTP
- ⏳ adi.knowledgebase - CLI + HTTP
- ⏳ adi.api-proxy - CLI + HTTP
- ⏳ lib-typespec-api - HTTP only
- ⏳ hive - CLI + HTTP (orchestration)

### Hive Orchestration Plugins (31)
**Environment Providers (4)**
- ⏳ hive.env.1password
- ⏳ hive.env.aws-secrets
- ⏳ hive.env.dotenv
- ⏳ hive.env.vault

**Health Checks (7)**
- ⏳ hive.health.cmd
- ⏳ hive.health.grpc
- ⏳ hive.health.http
- ⏳ hive.health.mysql
- ⏳ hive.health.postgres
- ⏳ hive.health.redis
- ⏳ hive.health.tcp

**Observability Sinks (4)**
- ⏳ hive.obs.file
- ⏳ hive.obs.loki
- ⏳ hive.obs.prometheus
- ⏳ hive.obs.stdout

**Proxy Middleware (11)**
- ⏳ hive.proxy.auth-api-key
- ⏳ hive.proxy.auth-basic
- ⏳ hive.proxy.auth-jwt
- ⏳ hive.proxy.auth-oidc
- ⏳ hive.proxy.cache
- ⏳ hive.proxy.compress
- ⏳ hive.proxy.cors
- ⏳ hive.proxy.headers
- ⏳ hive.proxy.ip-filter
- ⏳ hive.proxy.rate-limit
- ⏳ hive.proxy.rewrite

**Rollout Strategies (2)**
- ⏳ hive.rollout.blue-green
- ⏳ hive.rollout.recreate

**Runners (3)**
- ⏳ hive.runner.compose
- ⏳ hive.runner.docker
- ⏳ hive.runner.podman

**Orchestrator (1)**
- ⏳ hive.orchestrator

**Total Not Started:** 38 plugins

---

## 📊 Overall Progress

| Category | Complete | Partial | Not Started | Total |
|----------|----------|---------|-------------|-------|
| Infrastructure | 6 | 0 | 0 | 6 |
| Translation Plugins | 9 | 0 | 0 | 9 |
| Core CLI (Simple) | 1 | 4 | 0 | 5 |
| Core CLI (Complex) | 0 | 0 | 7 | 7 |
| Language Plugins | 0 | 11 | 0 | 11 |
| Hive Plugins | 0 | 0 | 31 | 31 |
| **TOTAL** | **16** | **15** | **38** | **69** |

**Completion Rate:** 16/69 (23% fully complete) + 15/69 (22% partially complete) = **45% overall**

---

## 🎯 Next Steps

### Immediate (Complete Partial Migrations)

1. **Finish lib.rs for partially migrated plugins (15)**
   - Use adi.workflow as template
   - Each needs ~10-30 minutes of manual adjustment
   - Estimated time: 5-10 hours total

### Phase 2 (Complex Plugins)

2. **Migrate complex core plugins (7)**
   - adi.tasks, adi.indexer, adi.agent-loop (CLI + HTTP + MCP)
   - Need HTTP + MCP trait implementations
   - Estimated time: 2-3 hours each = 14-21 hours

### Phase 3 (Hive Plugins)

3. **Migrate Hive orchestration plugins (31)**
   - Similar structure (implement orchestration traits)
   - Can be batched by category (env, health, obs, proxy, rollout, runner)
   - Estimated time: 1-2 hours per plugin = 31-62 hours

---

## 📝 Migration Checklist (Per Plugin)

For each partially migrated plugin:

```bash
cd crates/<plugin>/plugin

# 1. Check backup exists
ls src/lib.rs.v2.bak

# 2. Review generated lib.rs
cat src/lib.rs

# 3. Fix module imports (if needed)
# - Rename cli.rs → cli_impl.rs if exists
# - Add other modules as needed

# 4. Update list_commands() with actual commands
# - Copy from lib.rs.v2.bak

# 5. Build and test
cargo build --release

# 6. If successful, remove backup
rm src/lib.rs.v2.bak Cargo.toml.bak

# 7. Commit
git add .
git commit -m "🚀 migrate: <plugin-id> to v3 ABI"
```

---

## 💡 Lessons Learned

1. **Automated config migration works well**
   - Cargo.toml and plugin.toml can be batch-updated
   - 100% success rate for dependency/version changes

2. **lib.rs requires manual attention**
   - Each plugin has unique implementation details
   - Generic templates work for simple CLI-only plugins
   - Complex plugins (HTTP, MCP) need custom code

3. **Batch processing by similarity**
   - Translation plugins: 9 in one batch (similar structure)
   - Language plugins: Should be similar (11 total)
   - Hive plugins: Can batch by category

4. **Time estimates**
   - Simple plugin: 10-30 minutes
   - Medium plugin (CLI + HTTP): 1-2 hours
   - Complex plugin (CLI + HTTP + MCP): 2-3 hours

---

## 🚀 Estimated Remaining Time

| Phase | Plugins | Est. Time per Plugin | Total Time |
|-------|---------|---------------------|------------|
| Finish Partial (15) | 15 | 20 min | **5 hours** |
| Complex Core (7) | 7 | 2 hours | **14 hours** |
| Hive Plugins (31) | 31 | 1.5 hours | **46.5 hours** |
| **TOTAL** | **53** | - | **65.5 hours** |

**Realistic timeline:** 2-3 weeks part-time, 1.5-2 weeks full-time

---

## ✅ Ready for Production

**Current state:** adi-cli can load both v2 and v3 plugins simultaneously.

**Migration strategy:** Gradual rollout
- v2 and v3 plugins coexist
- Migrate plugins one by one
- No breaking changes for users
- Test each migration before proceeding

**Final goal:** 100% v3, deprecate v2 loader
