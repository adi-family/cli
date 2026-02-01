adi-tools, rust, cli-tool-index, llm-agents, mcp-alternative

## Overview
- Searchable index of CLI tools for LLM agents
- Single meta-command to search tools by intent
- Pulls full `--help` docs only when needed
- Convention-based: drop executables in `~/.local/share/adi/tools/`
- **No MCP required** - pure CLI interface

## Architecture
```
crates/adi-tools/
  core/     # Business logic: discovery, storage, search
  plugin/   # CLI plugin (adi tools <command>)
```

## Tool Convention
Tools are executables that follow this convention:
- `tool --help` - Human-readable usage (required)
- `tool --json` - Output JSON (optional)
- `tool describe` - One-line description (optional, else parse --help)

## CLI Commands
| Command | Description | LLM Use Case |
|---------|-------------|--------------|
| `adi tools find <query>` | Semantic + fuzzy search | "Find tool to list containers" |
| `adi tools help <tool>` | Full --help output | Get usage when needed |
| `adi tools list` | List all indexed tools | Browse available |
| `adi tools run <tool> [args]` | Execute tool | Call tool |
| `adi tools index` | Re-index all tools | Force refresh |
| `adi tools add <path>` | Add tool to index | Register new tool |
| `adi tools remove <id>` | Remove from index | Unregister tool |
| `adi tools stats` | Show index statistics | Debugging |

## Tool Sources
1. **ADI Plugins** - Scans `~/.local/share/adi/plugins/*/plugin.toml`
2. **Tools Directory** - Scans `~/.local/share/adi/tools/*` for executables

## Storage
- SQLite database at `~/.local/share/adi/tools.db`
- FTS5 for full-text search on names and descriptions
- Stores tool metadata, hash for change detection

## LLM Integration Example
```
Human: I need to check what containers are running

LLM: [searches tools]
$ adi tools find "list running containers"
-> docker ps: List running Docker containers
-> hive status: Show hive cocoon status

$ adi tools run docker-ps --json
-> [{"id": "abc123", "image": "nginx", ...}]

LLM: You have 3 containers running: nginx, postgres, redis.
```

## Build
```bash
cargo build -p adi-tools-core    # Core library
cargo build -p adi-tools-plugin  # CLI plugin
```

## Test
```bash
cargo test -p adi-tools-core
```
