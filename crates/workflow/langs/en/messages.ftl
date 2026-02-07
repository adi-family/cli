# ============================================================================
# ADI WORKFLOW - ENGLISH TRANSLATIONS
# ============================================================================

# Help and descriptions
workflow-description = Run workflows defined in TOML files
workflow-help-title = ADI Workflow - Run workflows defined in TOML files
workflow-help-commands = Commands:
workflow-help-run = Run a workflow by name
workflow-help-list = List available workflows
workflow-help-show = Show workflow definition
workflow-help-locations = Workflow locations:
workflow-help-local = (local, highest priority)
workflow-help-global = (global)
workflow-help-usage = Usage:

# List command
workflow-list-title = Available workflows:
workflow-list-empty = No workflows found.
workflow-list-hint-create = Create workflows at:
workflow-list-scope-local = [local]
workflow-list-scope-global = [global]

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

# Input prompts
workflow-input-error-tty = Interactive prompts require a TTY
workflow-input-error-options = { $type } input requires options
workflow-input-error-options-empty = { $type } input requires at least one option
workflow-input-error-validation = Invalid validation pattern: { $error }
workflow-input-error-prompt = Prompt error: { $error }
workflow-input-validation-failed = Input must match pattern: { $pattern }

# Execution
workflow-exec-error-spawn = Failed to spawn command: { $error }
workflow-exec-error-wait = Failed to wait for command: { $error }
workflow-exec-error-exit-code = Command failed with exit code: { $code }
workflow-exec-error-template = Template error: { $error }

# Common
workflow-common-error-parse = Failed to parse workflow: { $error }
workflow-common-error-read = Failed to read workflow file: { $error }
