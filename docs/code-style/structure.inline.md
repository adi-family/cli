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
- **Libraries** go in `crates/lib/lib-<name>/`
- **Standalone plugins** use `crates/<name>-plugin/` pattern
- **Tools** use `crates/tool-<name>/` pattern
