adi-workflow-plugin, rust, workflow-runner, toml, interactive-prompts

## Overview
- Plugin for running workflows defined in TOML files
- Interactive TTY prompts: select, input, confirm, multi-select, password
- Minijinja templating with conditionals and env access
- Sequential shell command execution

## Commands
- `adi workflow <name>` - Run a workflow by name
- `adi workflow list` - List available workflows
- `adi workflow show <name>` - Show workflow definition

## Workflow Discovery
- `./.adi/workflows/<name>.toml` - Local (highest priority)
- `~/.adi/workflows/<name>.toml` - Global (fallback)

## TOML Schema
```toml
[workflow]
name = "deploy"
description = "Deploy to production"

[[inputs]]
name = "env"
type = "select"  # select | input | confirm | multi-select | password
prompt = "Select environment"
options = ["staging", "production"]
default = "staging"

[[steps]]
name = "Build"
run = "cargo build --release"
if = "{{ env }} == 'production'"  # Optional condition
env = { API_KEY = "{{ api_key }}" }  # Optional env vars
```

## Templating
- Variables: `{{ variable_name }}`
- Conditionals: `{% if var %}...{% endif %}`
- Environment: `{{ env.VAR_NAME }}`
- Built-ins: `{{ cwd }}`, `{{ home }}`, `{{ date }}`

## Source Files
- `src/parser.rs` - TOML types and parsing
- `src/discovery.rs` - Workflow file discovery
- `src/prompts.rs` - Interactive TTY prompts (dialoguer)
- `src/template.rs` - Minijinja templating
- `src/executor.rs` - Shell command execution
- `src/cli.rs` - CLI command handling
- `src/lib.rs` - Plugin entry point
