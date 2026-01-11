# LLM Extract Specification
plugin-documentation, llm-context

## Overview

The `llm-extract` system provides a standardized way for ADI plugins to expose LLM-friendly documentation. This enables:
- AI assistants to understand plugin capabilities
- Automatic generation of claude.md/context files

## Architecture

```
plugin.toml + src/lib.rs  -->  llm_extract method  -->  JSON/Markdown output
```

## Plugin Interface

### CLI Service Method: `llm_extract`

Each plugin with a CLI service should implement the `llm_extract` method:

```rust
"llm_extract" => {
    let info = json!({
        "plugin": { /* metadata */ },
        "cli": { /* cli config */ },
        "commands": [ /* command list */ ],
        "services": [ /* service list */ ]
    });
    RResult::ROk(RString::from(serde_json::to_string(&info).unwrap()))
}
```

### Response Schema

```typescript
interface LLMExtract {
  plugin: {
    id: string;           // "adi.tasks"
    name: string;         // "ADI Tasks"
    version: string;      // "0.8.8"
    description: string;  // actionable description
    categories: string[]; // ["tasks", "workflow"]
    summary?: string;     // extended description for complex plugins
    use_cases?: string[]; // when to use this plugin
  };
  
  cli?: {
    command: string;      // "tasks"
    aliases: string[];    // ["t"]
    usage: string;        // "adi tasks <command> [options]"
  };
  
  commands: Command[];
  services: Service[];
}

interface Command {
  name: string;           // "list"
  description: string;    // what it does
  usage: string;          // "list [--status <s>] [--ready]"
  examples?: string[];    // ["adi tasks list --ready"]
  options?: Option[];     // detailed option descriptions
}

interface Option {
  name: string;           // "--status"
  short?: string;         // "-s"
  description: string;    // "Filter by status"
  type: string;           // "string" | "boolean" | "number"
  default?: any;          // default value
  choices?: string[];     // ["todo", "done", "blocked"]
}

interface Service {
  id: string;             // "adi.tasks.cli"
  version: string;        // "1.0.0"
  description: string;    // what the service provides
  methods?: ServiceMethod[];
}

interface ServiceMethod {
  name: string;           // "run_command"
  description: string;    // "Execute a CLI command"
  parameters?: object;    // JSON Schema for params
  returns?: object;       // JSON Schema for return
}
```

## Command Metadata in Code

```rust
let commands = json!([
    {
        "name": "list",
        "description": "List all tasks with optional filtering",
        "usage": "list [--status <status>] [--ready] [--blocked]",
        "examples": [
            "adi tasks list",
            "adi tasks list --status todo",
            "adi tasks list --ready --format json"
        ],
        "options": [
            {"name": "--status", "type": "string", "choices": ["todo", "in-progress", "done", "blocked"]},
            {"name": "--ready", "type": "boolean", "description": "Show only ready tasks"},
            {"name": "--format", "type": "string", "choices": ["text", "json"], "default": "text"}
        ]
    }
]);
```

## CLI Usage

```bash
# Extract LLM info for a plugin
adi llm-extract extract adi.tasks

# Extract all plugins
adi llm-extract all

# Output format
adi llm-extract extract adi.tasks --format json
adi llm-extract extract adi.tasks --format md
```

## Output Formats

### JSON
Raw structured data for programmatic use.

### Markdown (default)
Human-readable documentation optimized for AI context files:

```markdown
## adi.tasks (ADI Tasks)
task-management, dependency-tracking, dag-visualization

Task management with dependency tracking and DAG visualization

### Usage
```
adi tasks <command> [options]
```

Aliases: `t`

### Commands
- `list` - List all tasks
- `add` - Create a new task
- `depend` - Add dependency between tasks
```

## Reference Implementation

See `adi.linter` plugin (`crates/adi-linter/plugin/src/lib.rs`) for a complete example with:
- `llm_extract` method returning structured JSON
- Full command details with examples and options
- Plugin summary and use cases
- Config file example
