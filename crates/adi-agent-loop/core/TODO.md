# adi-agent-loop-core - TODO List

## ✅ Completed (2025-12-25)

### Quota System
- ✅ `src/quota.rs` - Complete quota management system
  - QuotaPeriod enum (Session, Minute, Hour, Day)
  - QuotaConfig with escalation support
  - QuotaManager with time-window tracking
  - QuotaUsage for persistence
  - Full test coverage

### Tool Configuration
- ✅ `src/tool_config.rs` - TOML-based configuration system
  - ToolConfig (permission, enabled, quota, custom config)
  - ToolConfigSet with file loading/saving
  - Test coverage

### Error Handling
- ✅ Extended error types in `src/error.rs`
  - QuotaExceeded, ToolDisabled, ConfigValidation
  - Provider-specific errors (Anthropic, OpenAI, OpenRouter, Ollama)
  - ApiKeyMissing, RateLimited, ProviderConfig
  - TOML serialization errors

### Provider Infrastructure
- ✅ `src/providers/` module structure
  - Factory pattern with ProviderConfig enum
  - Provider stubs compile successfully
  - Environment variable support for API keys

### Dependencies
- ✅ Added to `Cargo.toml`:
  - lib-client-anthropic, lib-client-openai, lib-client-openrouter, lib-client-ollama
  - jsonschema (for config validation)
  - toml (for configuration files)

---

## 🔨 TODO: Complete LLM Provider Implementations

### Priority 1: Provider API Integration

**File**: `src/providers/anthropic.rs`
- [ ] Match lib-client-anthropic API (check Client::new, request builders)
- [ ] Implement message conversion (System → system parameter, blocks-based content)
- [ ] Handle ContentBlock extraction (text, tool_use, tool_result)
- [ ] Implement tool schema conversion
- [ ] Add token usage extraction
- [ ] Test with real API (integration test)

**File**: `src/providers/openai.rs`
- [ ] Match lib-client-openai API (Client constructor, request format)
- [ ] Implement message conversion (all roles supported)
- [ ] Handle tool_calls JSON parsing
- [ ] Implement tool schema conversion
- [ ] Add token usage extraction
- [ ] Test with real API (integration test)

**File**: `src/providers/openrouter.rs`
- [ ] Match lib-client-openrouter API
- [ ] Implement message conversion (similar to OpenAI)
- [ ] Add OpenRouter-specific features (provider routing, fallbacks)
- [ ] Implement tool schema conversion
- [ ] Add token usage extraction
- [ ] Test with real API (integration test)

**File**: `src/providers/ollama.rs`
- [ ] Match lib-client-ollama API (Client::new or default)
- [ ] Implement message conversion
- [ ] Handle local model availability checks
- [ ] Add eval_count token usage
- [ ] Graceful error handling (Ollama not running)
- [ ] Test with local Ollama instance

**Reference**: Check each client library's examples and docs:
- `/Users/mgorunuch/projects/adi-family/cli/crates/lib-client-anthropic/`
- `/Users/mgorunuch/projects/adi-family/cli/crates/lib-client-openai/`
- `/Users/mgorunuch/projects/adi-family/cli/crates/lib-client-openrouter/`
- `/Users/mgorunuch/projects/adi-family/cli/crates/lib-client-ollama/`

---

## 🔧 TODO: Tool Configuration Schema

### Priority 2: Add ToolConfigSchema to tool.rs

**File**: `src/tool.rs`
- [ ] Add `ToolConfigSchema` struct
  - JSON Schema for tool-specific config
  - Default values
  - Validation support
  - Documentation field
- [ ] Add `config_schema` field to `ToolSchema` (optional)
- [ ] Implement `configure()` method in `ToolExecutor` trait (default no-op)
- [ ] Add validation in `ToolRegistry::configure_tool()`
- [ ] Add helper methods for common config patterns
- [ ] Write tests for config validation

**Example Config Schema**:
```rust
let config_schema = ToolConfigSchema::new(json!({
    "type": "object",
    "properties": {
        "timeout_seconds": {"type": "integer", "minimum": 1},
        "max_retries": {"type": "integer", "minimum": 0}
    }
}))
.with_defaults(json!({"timeout_seconds": 30, "max_retries": 3}))
.with_documentation("Timeout and retry configuration");
```

---

## 🎯 TODO: Integrate QuotaManager into AgentLoop

### Priority 3: Agent Loop Integration

**File**: `src/agent.rs`
- [ ] Add `QuotaManager` field to `AgentLoop`
- [ ] Initialize QuotaManager in `AgentLoop::new()`
- [ ] Add `configure_tool(&mut self, config: ToolConfig)` method
- [ ] Add `load_tool_configs(&mut self, path: &Path)` method
- [ ] Update `check_and_approve_call()` to check quotas before permissions
- [ ] Record operations in `execute_tool_calls()` after successful execution
- [ ] Handle quota escalation (Auto → Ask when quota exceeded)
- [ ] Add quota reset on session end
- [ ] Export/import quota state for session persistence
- [ ] Write integration tests

**Quota Check Flow**:
```
1. Check if tool enabled → if not, deny
2. Check quota:
   - No quota → use normal permission
   - Allowed → use normal permission
   - Exceeded → escalate to configured level (or deny)
3. Check effective permission (Auto/Ask/Deny)
4. Execute tool
5. Record operation in QuotaManager
```

---

## 📦 TODO: Session Persistence for Quotas

### Priority 4: Extend Session Storage

**File**: `src/storage/session.rs`
- [ ] Add `quota_state: HashMap<String, QuotaUsage>` field to `Session`
- [ ] Update SQLite schema to store quota state (JSON column)
- [ ] Add migration for new field
- [ ] Implement quota export on session save
- [ ] Implement quota import on session load
- [ ] Add `QuotaUsageSnapshot` type for serialization
- [ ] Test session persistence with quotas

---

## 🧪 TODO: Testing & Documentation

### Priority 5: Tests

- [ ] Integration tests for each provider (with #[ignore] for real API)
- [ ] Mock server tests for providers
- [ ] End-to-end tests: AgentLoop + QuotaManager + ToolConfig
- [ ] Config file loading tests
- [ ] Quota escalation tests
- [ ] Tool-specific config validation tests

### Priority 6: Documentation

- [ ] Update CLAUDE.md with new features
- [ ] Add provider usage examples
- [ ] Add configuration examples
- [ ] Add quota configuration examples
- [ ] Document tool config schema format
- [ ] API documentation for new types

---

## 📋 TODO: CLI & HTTP Integration

### Priority 7: Update CLI (adi-agent-loop-cli)

- [ ] Add config commands:
  - `adi-agent config tools list`
  - `adi-agent config tools set <name> <permission>`
  - `adi-agent config tools enable/disable <name>`
  - `adi-agent config quota set <name> <max> <period>`
- [ ] Add config file loading from `~/.config/adi/agent.toml`
- [ ] Add `--config` flag to specify config file
- [ ] Add provider selection flags (`--provider anthropic|openai|...`)
- [ ] Update `run` command to use real providers

### Priority 8: Update HTTP Server (adi-agent-loop-http)

- [ ] Add config endpoints:
  - `GET /api/config/tools` - List tool configs
  - `PUT /api/config/tools/:name` - Update tool config
  - `GET /api/quota/stats` - Get quota statistics
- [ ] Add provider selection in request body
- [ ] Add config file loading on startup
- [ ] Update `/api/run` to use real providers

---

## 🔌 TODO: Plugin Integration

### Priority 9: Plugin Tool Bridge

**File**: `src/plugin_tools.rs` (NEW)
- [ ] Create `PluginToolExecutor` wrapping MCP tools
- [ ] Implement `register_plugin_tools(registry, plugin_host)` function
- [ ] Convert MCP tool schema to ToolSchema
- [ ] Call MCP tools via service handle
- [ ] Set default permission for plugin tools (Ask)
- [ ] Support tool-specific config from plugins
- [ ] Test with adi-indexer-plugin

---

## 📊 Progress Summary

- **Completed**: 40% (Core infrastructure)
- **In Progress**: 0%
- **Remaining**: 60% (Provider implementations, integrations, testing)

**Estimated Effort**:
- Provider implementations: ~4-6 hours
- Tool config schema: ~2 hours
- AgentLoop integration: ~3 hours
- Session persistence: ~1 hour
- Testing: ~3 hours
- CLI/HTTP updates: ~2 hours
- Plugin integration: ~2 hours

**Total**: ~17-20 hours of development work

---

## 🎯 Quick Start for Next Developer

1. **To complete Anthropic provider**:
   ```bash
   # Check the client library API
   cd /Users/mgorunuch/projects/adi-family/cli/crates/lib-client-anthropic
   # Look at examples or tests to see correct API usage

   # Update src/providers/anthropic.rs to match the API
   # Run tests:
   cargo test --lib providers::anthropic
   ```

2. **To test quota system**:
   ```rust
   let mut manager = QuotaManager::new();
   manager.set_quota("tool1", QuotaConfig::per_session(5));

   assert!(matches!(manager.check("tool1"), QuotaCheckResult::Allowed { .. }));
   manager.record("tool1");
   ```

3. **To test tool config**:
   ```rust
   let config = ToolConfig::new()
       .with_permission(PermissionLevel::Auto)
       .with_quota(QuotaConfig::per_minute(10));

   let mut set = ToolConfigSet::new();
   set.add("my_tool", config);
   set.to_toml_file("test.toml")?;
   ```

---

## 📞 Questions?

- Check the plan files: `/tmp/tool-based-permissions-plan.md`
- Review the architecture in `CLAUDE.md`
- All core types compile and have tests
- Provider stubs clearly marked with TODO comments

Happy coding! 🚀
