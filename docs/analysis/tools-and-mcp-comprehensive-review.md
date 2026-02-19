# Comprehensive Review: AI Coding Agent Tools & MCP

> Research summary of tool implementations across major AI coding agents (Feb 2026)

## Executive Summary

This document analyzes how major AI coding agents implement tools, focusing on:
- Built-in tools vs extensible tools
- MCP (Model Context Protocol) adoption
- Function calling patterns
- Best practices and design patterns

**Key Finding**: MCP has become the de-facto standard for tool extensibility, adopted by OpenAI, Anthropic, Google, and open-source projects like OpenCode.

---

## 1. Tool Categories Across Platforms

### 1.1 Core Built-in Tools (Common Across All Platforms)

| Tool | OpenCode | Claude Code | Codex | Purpose |
|------|----------|-------------|-------|---------|
| File Read | `read` | `Read` | `read` | Read file contents |
| File Write | `write` | `Write` | `write` | Create/overwrite files |
| File Edit | `edit` | `Edit` | `edit/patch` | Modify existing files |
| Shell/Bash | `bash` | `Bash` | `shell` | Execute commands |
| Grep/Search | `grep` | `Grep` | `search` | Content search |
| Glob | `glob` | `Glob` | `glob` | File pattern matching |
| Web Fetch | `webfetch` | `WebFetch` | `web_search` | Fetch web content |
| Todo/Tasks | `todowrite/todoread` | `TodoWrite` | - | Task tracking |
| Question | `question` | `Question` | - | Interactive prompts |

### 1.2 Advanced Tools

| Tool | Platform | Description |
|------|----------|-------------|
| `lsp` | OpenCode | LSP integration (definitions, references, hover) |
| `skill` | OpenCode/Codex | Load skill files for specialized tasks |
| `websearch` | OpenCode | Web search via Exa AI |
| `patch` | Codex | Apply unified diffs |
| `code_interpreter` | OpenAI | Execute Python in sandbox |
| `computer_use` | OpenAI | Control computer interfaces |
| `image_generation` | OpenAI | Generate images |

---

## 2. MCP (Model Context Protocol) Architecture

### 2.1 Core Concepts

MCP follows a **client-server architecture**:

```
┌─────────────────────────────────────────┐
│          MCP Host (AI Application)       │
│  ┌─────────────┐ ┌─────────────────────┐ │
│  │ MCP Client 1│ │ MCP Client 2        │ │
│  └──────┬──────┘ └──────────┬──────────┘ │
└─────────┼───────────────────┼────────────┘
          │                   │
    ┌─────▼─────┐       ┌─────▼─────┐
    │MCP Server │       │MCP Server │
    │  (Local)  │       │ (Remote)  │
    └───────────┘       └───────────┘
```

### 2.2 MCP Primitives

| Primitive | Purpose | Discovery | Execution |
|-----------|---------|-----------|-----------|
| **Tools** | Actions LLM can invoke | `tools/list` | `tools/call` |
| **Resources** | Context data (files, DB records) | `resources/list` | `resources/read` |
| **Prompts** | Reusable templates | `prompts/list` | `prompts/get` |

### 2.3 MCP Transport Options

1. **STDIO** - Local process communication
   - Best for: Local MCP servers
   - Pros: No network overhead, simple setup
   - Cons: Single client per server

2. **Streamable HTTP** - Remote server communication
   - Best for: Cloud-hosted MCP servers
   - Pros: Multi-client support, standard auth (OAuth, API keys)
   - Cons: Network latency, requires hosting

### 2.4 Tool Definition Schema (JSON-RPC 2.0)

```json
{
  "name": "get_weather",
  "description": "Get weather for a location",
  "inputSchema": {
    "type": "object",
    "properties": {
      "location": {
        "type": "string",
        "description": "City and country"
      },
      "units": {
        "type": "string",
        "enum": ["celsius", "fahrenheit"]
      }
    },
    "required": ["location"]
  }
}
```

---

## 3. Platform-Specific Implementations

### 3.1 OpenAI (Codex/ChatGPT)

**Tool Types:**
- Function calling (JSON schema)
- Built-in tools (web_search, file_search, code_interpreter)
- Remote MCP servers
- Custom tools (Skills)

**Key Features:**
- `strict: true` mode for guaranteed schema adherence
- Parallel tool calls support
- `tool_choice` parameter for control
- Skills: Versioned, reusable tool bundles

**Best Practices from OpenAI:**
1. Write clear descriptions (principle of least surprise)
2. Use enums to prevent invalid states
3. Keep functions < 20 at a time
4. Don't make model fill known arguments
5. Combine sequentially-called functions

### 3.2 Claude Code (Anthropic)

**Tool Types:**
- Built-in tools (Read, Write, Edit, Bash, Grep, Glob)
- MCP servers (local/remote)
- Custom slash commands
- Hooks (pre/post action scripts)

**Key Features:**
- `CLAUDE.md` for project-specific instructions
- Sub-agents for parallel tasks
- GitHub/GitLab Actions integration
- Agent SDK for custom workflows

### 3.3 OpenCode (Open Source)

**Tool Types:**
- Built-in tools (15+ core tools)
- MCP servers (local/remote with OAuth)
- Custom tools (TypeScript/JavaScript definitions)
- Agent skills (SKILL.md files)

**Custom Tool Pattern:**
```typescript
import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "Query the database",
  args: {
    query: tool.schema.string().describe("SQL query"),
  },
  async execute(args, context) {
    // context.directory, context.worktree available
    return `Result: ${args.query}`
  },
})
```

**Key Features:**
- AGENTS.md for project context
- Permission system (allow/deny/ask)
- Glob patterns for tool control
- Per-agent tool configuration

---

## 4. Best Practices for Tool Design

### 4.1 Tool Definition Best Practices

| Practice | Description |
|----------|-------------|
| **Clear naming** | Use descriptive, action-oriented names (`get_weather`, not `weather`) |
| **Detailed descriptions** | Explain when/how to use, edge cases, output format |
| **Type safety** | Use strict schemas with enums where applicable |
| **Required vs optional** | Mark truly required fields, use `null` type for optional |
| **Atomic operations** | Single responsibility per tool |

### 4.2 MCP Server Best Practices

1. **Context Management**: MCP servers add tokens - be selective
2. **OAuth Support**: Implement RFC 7591 Dynamic Client Registration
3. **Timeout Handling**: Default 5s timeout, configure for slow operations
4. **Error Handling**: Return structured errors with codes and messages
5. **Logging**: STDIO servers must log to stderr, never stdout

### 4.3 Permission Patterns

```json
{
  "permission": {
    "edit": "ask",           // Require approval
    "bash": "allow",         // Auto-approve
    "mcp_*": "deny",         // Block all MCP
    "sentry_*": "allow"      // Allow specific MCP
  }
}
```

---

## 5. Comparison: Function Calling vs MCP

| Aspect | Function Calling | MCP |
|--------|------------------|-----|
| **Definition** | JSON Schema in API request | Server-hosted schemas |
| **Execution** | Client-side | Server-side |
| **State** | Stateless | Stateful (session-based) |
| **Discovery** | Static (compile-time) | Dynamic (runtime) |
| **Ecosystem** | Per-application | Cross-application |
| **Best for** | Simple integrations | Complex, reusable tools |

---

## 6. Implementation Recommendations for ADI

### 6.1 Tool Architecture

```
┌─────────────────────────────────────────────────────┐
│                   ADI Plugin                         │
├──────────────────┬──────────────────────────────────┤
│  CLI Commands    │  MCP Tools                       │
│  (adi tasks ...)│  (list_tasks, create_task, ...)  │
├──────────────────┼──────────────────────────────────┤
│  GlobalCommands  │  MCP Resources                   │
│  (adi up/down)   │  (tasks://config, schema)        │
├──────────────────┼──────────────────────────────────┤
│  HTTP Routes     │  MCP Prompts                     │
│  (REST API)      │  (task_review, planning)         │
└──────────────────┴──────────────────────────────────┘
```

### 6.2 Recommended MCP Tool Schema Pattern

```rust
#[mcp_tool(name = "list_tasks", description = "List all tasks")]
async fn list_tasks(&self, status: Option<String>, limit: Option<i64>) -> Result<String> {
    // Auto-generates:
    // - JSON schema from function signature
    // - Input validation
    // - Error handling wrapper
}
```

### 6.3 Plugin SDK Enhancements

Based on research, ADI's Plugin SDK should support:

1. **Tool Categories**: CLI, MCP, HTTP (already done)
2. **Permission System**: Similar to OpenCode's allow/deny/ask
3. **Dynamic Discovery**: Runtime `tools/list` for MCP
4. **Strict Mode**: Schema validation like OpenAI
5. **Context Injection**: Session/project context in tool calls

### 6.4 MCP Server Best Practices for Plugins

```rust
// 1. Clear, action-oriented names
#[mcp_tool(name = "create_task")]  // Good
#[mcp_tool(name = "task")]         // Bad

// 2. Detailed descriptions
#[mcp_tool(
    name = "update_status",
    description = "Update task status. Valid values: todo, in_progress, done, blocked, cancelled"
)]

// 3. Use translations for descriptions
#[mcp_tool(name = "list_tasks", description = t!("mcp-tool-list-tasks-desc"))]

// 4. Structured error responses
async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult> {
    match name {
        "create_task" => ...,
        _ => Ok(McpToolResult::error(format!("Unknown tool: {}", name))),
    }
}
```

---

## 7. Token Efficiency Considerations

| Strategy | Description | Impact |
|----------|-------------|--------|
| **Selective tools** | Only enable needed MCP servers | -30-50% tokens |
| **Per-agent tools** | Configure tools per subagent | -20% tokens |
| **Compaction** | Summarize long tool outputs | Variable |
| **Caching** | Cache tool schemas | Faster init |

---

## 8. Security Considerations

1. **Permission Prompts**: Show all tool capabilities on plugin install
2. **Sudo Commands**: Separate approval for privileged operations
3. **OAuth Scopes**: Request minimal necessary scopes
4. **Input Validation**: Always validate tool arguments server-side
5. **Output Sanitization**: Sanitize tool outputs before display

---

## 9. References

- [Model Context Protocol Specification](https://modelcontextprotocol.io)
- [OpenAI Function Calling Guide](https://platform.openai.com/docs/guides/function-calling)
- [Claude Code Documentation](https://code.claude.com/docs)
- [OpenCode Documentation](https://opencode.ai/docs)
- [OpenAI Codex Documentation](https://platform.openai.com/docs/codex)

---

## 10. Glossary

| Term | Definition |
|------|------------|
| **MCP** | Model Context Protocol - open standard for AI-external system communication |
| **Tool** | Function/capability exposed to an LLM for execution |
| **Resource** | Read-only data source exposed via MCP |
| **Prompt** | Reusable template for LLM interactions |
| **STDIO Transport** | Local IPC via stdin/stdout |
| **Streamable HTTP** | Remote MCP communication via HTTP/SSE |
| **Skill** | Reusable, versioned tool bundle (OpenAI/OpenCode) |
| **Hook** | Pre/post action script (Claude Code) |
