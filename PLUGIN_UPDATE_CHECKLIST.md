# Plugin Update Checklist

Reference: tasks plugin & hive plugin were recently updated with these patterns:
- `lib-plugin-prelude` instead of direct `lib-plugin-abi-v3`
- `#[command]` and `#[derive(CliArgs)]` macros instead of manual CLI boilerplate
- `min_host_version` bumped to `2.0.0`
- locales/ directory for i18n
- Dead code removed, DRY helpers extracted, tautological comments removed

---

## Domain-Specific Plugins

### agent-loop-plugin (`crates/agent-loop/plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual `list_commands()` / `run_command()` with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.8.0` to `2.0.0`
- [ ] Add `locales/` directory (en-US, de-DE, uk-UA, zh-CN)
- [ ] Replace `console::style()` calls with `lib-console-output` theme functions
- [ ] Remove redundant `plugin_create_cli()` entry point (keep only `plugin_create()`)
- [ ] Review for dead code and DRY violations

### audio-plugin (`crates/audio/plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.9.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Replace manual string building with `lib-console-output`
- [ ] Remove redundant entry points
- [ ] Review for dead code and DRY violations

### browser-debug-plugin (`crates/browser-debug/plugin/`)
- [ ] **Complete v3 migration** (currently stub with TODO)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.9.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Remove redundant entry points

### flags-plugin (`crates/flags/plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.9.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Remove redundant entry points
- [ ] Review for dead code and DRY violations

### indexer-plugin (`crates/indexer/plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.8.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Remove redundant entry points
- [ ] Review for dead code and DRY violations

### knowledgebase-plugin (`crates/knowledgebase/plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.8.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Remove redundant entry points
- [ ] Review for dead code and DRY violations

### linter-plugin (`crates/linter/plugin/`)
- [ ] **Complete v3 migration** (currently stub with TODO)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.9.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Remove redundant entry points

### llm-proxy-plugin (`crates/llm-proxy/plugin/`)
- [ ] **Add missing `[package.metadata.plugin]` section to Cargo.toml**
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Set `min_host_version` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Replace `console.workspace` usage with `lib-console-output`
- [ ] Remove redundant entry points
- [ ] Review for dead code and DRY violations

### tools-plugin (`crates/tools/plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.8.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Replace manual string building with `lib-console-output`
- [ ] Remove redundant entry points
- [ ] Review for dead code and DRY violations

### workflow-plugin (`crates/workflow/plugin/`)
- [ ] Already uses `lib-plugin-prelude` - verify up to date
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.9.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Replace manual string building with `lib-console-output`
- [ ] Remove redundant entry points
- [ ] Review for dead code and DRY violations

### embed-plugin (`crates/embed-plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.8.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Remove redundant entry points

### llm-extract-plugin (`crates/llm-extract-plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.8.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Remove redundant entry points

### llm-uzu-plugin (`crates/llm-uzu-plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.8.0` to `2.0.0`
- [ ] Add `locales/` directory
- [ ] Remove redundant entry points

---

## Language Plugins (`crates/lang/*/plugin/`)

All 11 language plugins share the same issues. They don't have CLI commands (pure `LanguageAnalyzer` trait), so `#[command]`/`#[derive(CliArgs)]` don't apply.

### Common updates for all lang plugins
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Bump `min_host_version` to `2.0.0`
- [ ] Add `locales/` directory (if plugin emits user-facing messages)

### Per-plugin issues

| Plugin | Path | Specific Issues |
|--------|------|-----------------|
| C/C++ | `crates/lang/cpp/plugin/` | Bump 0.9.0 -> 2.0.0 |
| C# | `crates/lang/csharp/plugin/` | Bump 0.9.0 -> 2.0.0 |
| Go | `crates/lang/go/plugin/` | Bump 0.9.0 -> 2.0.0 |
| Java | `crates/lang/java/plugin/` | Bump 0.9.0 -> 2.0.0 |
| Lua | `crates/lang/lua/plugin/` | Bump 0.9.0 -> 2.0.0 |
| PHP | `crates/lang/php/plugin/` | Bump 0.9.0 -> 2.0.0, review unsafe ptr usage |
| Python | `crates/lang/python/plugin/` | Bump 0.9.0 -> 2.0.0 |
| **Ruby** | `crates/lang/ruby/plugin/` | **Bump 0.8.0 -> 2.0.0**, add missing `api_version = 3` to metadata |
| Rust | `crates/lang/rust/plugin/` | Bump 0.9.0 -> 2.0.0, review unsafe ptr usage |
| Swift | `crates/lang/swift/plugin/` | Bump 0.9.0 -> 2.0.0 |
| TypeScript | `crates/lang/typescript/plugin/` | Bump 0.9.0 -> 2.0.0 |

---

## Hive Plugins (`crates/hive/plugins/`)

All 34 hive plugins share the same issues:

### Common updates for all hive plugins
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Bump `min_host_version` from `0.8.0` to `2.0.0`

### Plugins list (34)
- [ ] `env-1password`
- [ ] `env-aws-secrets`
- [ ] `env-dotenv`
- [ ] `env-vault`
- [ ] `health-cmd`
- [ ] `health-grpc`
- [ ] `health-http`
- [ ] `health-mysql`
- [ ] `health-postgres`
- [ ] `health-redis`
- [ ] `health-tcp`
- [ ] `obs-file`
- [ ] `obs-loki`
- [ ] `obs-prometheus`
- [ ] `obs-stdout`
- [ ] `orchestrator`
- [ ] `proxy-auth-api-key`
- [ ] `proxy-auth-basic`
- [ ] `proxy-auth-jwt`
- [ ] `proxy-auth-oidc`
- [ ] `proxy-cache`
- [ ] `proxy-compress`
- [ ] `proxy-cors`
- [ ] `proxy-headers`
- [ ] `proxy-ip-filter`
- [ ] `proxy-rate-limit`
- [ ] `proxy-rewrite`
- [ ] `proxy-ssl`
- [ ] `rollout-blue-green`
- [ ] `rollout-recreate`
- [ ] `runner-compose`
- [ ] `runner-docker`
- [ ] `runner-podman`
- [ ] `runner-watcher`

---

## TypeSpec Plugin (`crates/lib/lib-typespec-api/plugin/`)
- [ ] Switch from `lib-plugin-abi-v3` to `lib-plugin-prelude`
- [ ] Replace manual CLI boilerplate with `#[command]` + `#[derive(CliArgs)]`
- [ ] Bump `min_host_version` from `0.8.0` to `2.0.0`
- [ ] Add `locales/` directory

---

## CLI Translation Plugins (`crates/cli/plugins/`)
- [x] Already at `min_host_version = 3.0.0` (ahead of target)
- [x] Already have proper metadata with `api_version = 3`
- [x] Already have locales (that's their purpose)
- No updates needed

---

## TypeScript Plugin SDK (`packages/plugin-sdk/`)
- [x] Recently updated (registry-http.ts added, cocoon registry removed)
- [ ] Verify all tests pass after recent changes
- [ ] Update package version if publishing

---

## Priority Order

1. **Critical** - Complete v3 migrations: `browser-debug-plugin`, `linter-plugin`
2. **Critical** - Add missing metadata: `llm-proxy-plugin`
3. **High** - Domain-specific plugins (13 plugins): prelude + macros + version bump
4. **Medium** - Hive plugins (34 plugins): prelude + version bump
5. **Medium** - Language plugins (11 plugins): prelude + version bump
6. **Low** - TypeSpec plugin: prelude + macros + version bump
