# ============================================================================
# ADI WORKFLOW - ENGLISH TRANSLATIONS
# ============================================================================

# Plugin metadata
plugin-name = ADI Workflow
plugin-description = Run workflows defined in TOML files with interactive prompts

# Help and descriptions
workflow-description = Run workflows defined in TOML files
workflow-help-title = ADI Workflow - Run workflows defined in TOML files
workflow-help-commands = Commands:
workflow-help-run = Run a workflow by name
workflow-help-list = List available workflows
workflow-help-show = Show workflow definition
workflow-help-options = Options:
workflow-help-option-input = Pre-fill input value (repeatable)
workflow-help-option-schema = Output workflow inputs as JSON schema (for LLM/automation)
workflow-help-option-help = Show this help message
workflow-help-locations = Workflow locations:
workflow-help-local = (local, highest priority)
workflow-help-global = (global)
workflow-help-examples = Examples:
workflow-help-completions = Output completion suggestions (internal use)

# List command
workflow-list-title = Available workflows:
workflow-list-empty = No workflows found.
workflow-list-hint-create = Create workflows at:
workflow-list-scope-local = [local]
workflow-list-scope-global = [global]

# Select workflow
workflow-select-prompt = Select a workflow to run:

# Show command
workflow-show-title = Workflow: { $name }
workflow-show-description = Description: { $description }
workflow-show-path = Path: { $path }
workflow-show-inputs = Inputs:
workflow-show-input-options = Options: { $options }
workflow-show-input-default = Default: { $default }
workflow-show-steps = Steps:
workflow-show-step-if = if: { $condition }
workflow-show-step-run = run: { $command }
workflow-show-error-missing-name = Missing workflow name. Usage: show <name>
workflow-show-error-not-found = Workflow '{ $name }' not found

# Run command
workflow-run-title = Running workflow: { $name }
workflow-run-collecting-inputs = Collecting inputs...
workflow-run-executing-steps = Executing steps...
workflow-run-step-running = Running step { $number }: { $name }
workflow-run-step-skipping = Skipping step { $number }: { $name } (condition not met)
workflow-run-success = Workflow '{ $name }' completed successfully!
workflow-run-error-not-found = Workflow '{ $name }' not found
workflow-run-error-no-steps = Workflow has no steps to execute

# Cancellation
workflow-cancelled-selection = Selection cancelled
workflow-cancelled-input = Input cancelled
workflow-cancelled-confirm = Confirmation cancelled
workflow-cancelled-multiselect = Multi-select cancelled
workflow-cancelled-password = Password input cancelled

# Input prompts
workflow-input-error-tty = Interactive prompts require a TTY
workflow-input-error-options = { $type } input requires options
workflow-input-error-options-empty = { $type } input requires at least one option
workflow-input-error-validation = Invalid validation pattern: { $error }
workflow-input-error-prompt = Prompt error: { $error }
workflow-input-validation-failed = Input must match pattern: { $pattern }
workflow-input-error-invalid-boolean = Invalid boolean value for '{ $name }': '{ $value }' (use true/false/yes/no)
workflow-input-error-invalid-value = Invalid value for '{ $name }': '{ $value }'
workflow-input-error-valid-options = Valid options: { $options }
workflow-input-error-missing-required = Missing required inputs in non-interactive mode: { $inputs }
workflow-input-error-missing-hint = Provide them via CLI: { $hint }
workflow-input-error-pattern-mismatch = Value for '{ $name }' doesn't match pattern: { $pattern }

# Execution
workflow-exec-error-spawn = Failed to spawn command: { $error }
workflow-exec-error-wait = Failed to wait for command: { $error }
workflow-exec-error-exit-code = Command failed with exit code: { $code }
workflow-exec-error-template = Template error: { $error }

# Options resolution
workflow-options-error-requires = Input '{ $name }' requires options, options_cmd, or options_source
workflow-options-error-cmd-exec = Failed to execute options_cmd: { $error }
workflow-options-error-cmd-failed = options_cmd failed: { $error }
workflow-options-error-cmd-empty = options_cmd returned no options
workflow-options-error-git-branches = Failed to get git branches: { $error }
workflow-options-error-not-git = Not a git repository or git not installed
workflow-options-error-no-branches = No git branches found
workflow-options-error-git-tags = Failed to get git tags: { $error }
workflow-options-error-no-tags = No git tags found
workflow-options-error-git-remotes = Failed to get git remotes: { $error }
workflow-options-error-no-remotes = No git remotes found
workflow-options-error-read-file = Failed to read { $path }: { $error }
workflow-options-error-no-services = No services found in { $file }
workflow-options-error-dir-not-found = Directory not found: { $path }
workflow-options-error-glob = Invalid glob pattern: { $error }
workflow-options-error-no-dirs = No directories found in { $path }
workflow-options-error-path-not-found = Path not found: { $path }
workflow-options-error-no-files = No files found in { $path }
workflow-options-error-no-lines = No options found in { $path }
workflow-options-error-no-cargo = No Cargo.toml found in current directory
workflow-options-error-cargo-parse = Failed to parse Cargo.toml: { $error }
workflow-options-error-no-members = No workspace members found

# Common
workflow-common-error-parse = Failed to parse workflow: { $error }
workflow-common-error-read = Failed to read workflow file: { $error }
workflow-common-error-context = Invalid context: { $error }
