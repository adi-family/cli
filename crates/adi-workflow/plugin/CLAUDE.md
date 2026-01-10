adi-workflow-plugin, rust, workflow-runner, toml, interactive-prompts, bundled-prelude

## Overview
- Plugin for running workflows defined in TOML files
- Interactive TTY prompts: select, input, confirm, multi-select, password
- Minijinja templating with conditionals and env access
- Sequential shell command execution
- **Bundled prelude**: All workflow steps automatically get utility functions and variables

## Commands
- `adi workflow <name>` - Run a workflow by name
- `adi workflow list` - List available workflows
- `adi workflow show <name>` - Show workflow definition

## Workflow Discovery
- `./.adi/workflows/<name>.toml` - Local (highest priority)
- `~/.adi/workflows/<name>.toml` - Global (fallback)

## Bundled Prelude (Auto-Injected)

Every workflow step automatically has access to:

### Variables
| Variable | Description | Example |
|----------|-------------|---------|
| `$PROJECT_ROOT` | Project root directory | `/path/to/project` |
| `$WORKFLOWS_DIR` | Workflows directory | `/path/to/.adi/workflows` |
| `$CWD` | Current working directory | `/path/to/cwd` |
| `$OS` | Operating system | `darwin`, `linux` |
| `$ARCH` | Architecture | `x86_64`, `aarch64` |
| `$PLATFORM` | Combined | `darwin-aarch64` |
| `$GIT_ROOT` | Git repository root | `/path/to/repo` |
| `$GIT_BRANCH` | Current git branch | `main` |
| `$TIMESTAMP` | Current timestamp | `2024-01-10 14:30:00` |
| `$DATE` | Current date | `2024-01-10` |

### Logging Functions
- `info <msg>` - Info message (cyan)
- `success <msg>` - Success message (green)
- `warn <msg>` - Warning message (yellow)
- `error <msg>` - Error message (red), exits

### Spinner Functions
- `spinner_start <msg>` - Start animated spinner
- `spinner_stop [status]` - Stop spinner (success/error/warn)
- `with_spinner <msg> <cmd>` - Run command with spinner

### Progress Functions
- `progress_start <total> <msg>` - Start progress bar
- `progress_update <current>` - Update progress
- `progress_done [msg]` - Complete progress

### Step Counter
- `steps_init <total>` - Initialize step counter
- `step <msg>` - Print step with counter

### Prompts (interactive)
- `prompt_confirm <msg> [default]` - Yes/No
- `prompt_input <msg> [default]` - Text input
- `prompt_password <msg>` - Hidden input

### Utilities
- `require_file <path>` - Exit if file missing
- `require_dir <path>` - Exit if dir missing
- `ensure_dir <path>` - Create dir if missing
- `ensure_command <cmd>` - Exit if command missing
- `in_project <cmd>` - Run in $PROJECT_ROOT
- `is_ci` - Check if running in CI
- `countdown <secs> [msg]` - Countdown timer

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
run = """
# All prelude functions are available!
info "Building for $PLATFORM..."
spinner_start "Compiling..."
cargo build --release
spinner_stop "success"
"""
if = "{{ env }} == 'production'"  # Optional condition
env = { API_KEY = "{{ api_key }}" }  # Optional env vars
```

## Templating
- Variables: `{{ variable_name }}`
- Conditionals: `{% if var %}...{% endif %}`
- Environment: `{{ env.VAR_NAME }}`
- Built-ins: `{{ cwd }}`, `{{ home }}`, `{{ date }}`

## Source Files
- `src/prelude.rs` - Bundled shell prelude (auto-injected)
- `src/parser.rs` - TOML types and parsing
- `src/discovery.rs` - Workflow file discovery
- `src/prompts.rs` - Interactive TTY prompts (dialoguer)
- `src/template.rs` - Minijinja templating
- `src/executor.rs` - Shell command execution
- `src/cli.rs` - CLI command handling
- `src/lib.rs` - Plugin entry point
