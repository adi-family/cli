# ============================================================================
# ADI TASKS - ENGLISH TRANSLATIONS
# ============================================================================

# Plugin metadata
plugin-name = Tasks
plugin-description = Task management with dependency tracking

# Command descriptions
cmd-list-help = List all tasks
cmd-add-help = Add a new task
cmd-show-help = Show task details
cmd-status-help = Update task status
cmd-delete-help = Delete a task
cmd-depend-help = Add dependency between tasks
cmd-undepend-help = Remove dependency between tasks
cmd-graph-help = Show dependency graph
cmd-search-help = Search tasks
cmd-blocked-help = Show blocked tasks
cmd-cycles-help = Detect dependency cycles
cmd-stats-help = Show task statistics

# Help text
tasks-help-title = ADI Tasks - Task management with dependency tracking
tasks-help-commands = Commands:
tasks-help-usage = Usage: adi tasks <command> [args]

# List command
tasks-list-empty = No tasks found
tasks-list-scope-global = [global]
tasks-list-scope-project = [project]

# Add command
tasks-add-missing-title = Missing title. Usage: add <title> [--description <desc>]
tasks-add-created = Created task #{ $id }: { $title }

# Show command
tasks-show-missing-id = Missing task ID. Usage: show <id>
tasks-show-invalid-id = Invalid task ID
tasks-show-title = Task #{ $id }
tasks-show-field-title = Title: { $title }
tasks-show-field-status = Status: { $status }
tasks-show-field-description = Description: { $description }
tasks-show-field-symbol = Linked symbol: #{ $symbol_id }
tasks-show-field-scope = Scope: { $scope }
tasks-show-dependencies = Dependencies:
tasks-show-dependents = Dependents:

# Status command
tasks-status-missing-args = Missing arguments. Usage: status <id> <status>
tasks-status-invalid-id = Invalid task ID
tasks-status-invalid-status = Invalid status: { $status }. Valid: todo, in-progress, done, blocked, cancelled
tasks-status-updated = Task #{ $id } status updated to { $status }

# Delete command
tasks-delete-missing-id = Missing task ID. Usage: delete <id> [--force]
tasks-delete-invalid-id = Invalid task ID
tasks-delete-confirm = Delete task #{ $id }: { $title }?
tasks-delete-confirm-hint = Use --force to confirm deletion
tasks-delete-success = Deleted task #{ $id }: { $title }

# Depend command
tasks-depend-missing-args = Missing arguments. Usage: depend <task-id> <depends-on-id>
tasks-depend-invalid-task-id = Invalid task ID
tasks-depend-invalid-depends-id = Invalid depends-on ID
tasks-depend-success = Task #{ $task_id } now depends on task #{ $depends_on }

# Undepend command
tasks-undepend-missing-args = Missing arguments. Usage: undepend <task-id> <depends-on-id>
tasks-undepend-invalid-task-id = Invalid task ID
tasks-undepend-invalid-depends-id = Invalid depends-on ID
tasks-undepend-success = Removed dependency: #{ $task_id } -> #{ $depends_on }

# Graph command
tasks-graph-title = Task Dependency Graph
tasks-graph-empty = No tasks found
tasks-graph-depends-on = depends on #{ $id }: { $title }

# Search command
tasks-search-missing-query = Missing query. Usage: search <query> [--limit <n>]
tasks-search-empty = No tasks found
tasks-search-results = Found { $count } results for "{ $query }":

# Blocked command
tasks-blocked-empty = No blocked tasks
tasks-blocked-title = Blocked Tasks
tasks-blocked-by = blocked by #{ $id }: { $title } ({ $status })

# Cycles command
tasks-cycles-empty = No circular dependencies detected
tasks-cycles-found = Found { $count } circular dependencies:
tasks-cycles-item = Cycle { $number }:

# Stats command
tasks-stats-title = Task Statistics
tasks-stats-total = Total tasks: { $count }
tasks-stats-todo = Todo: { $count }
tasks-stats-in-progress = In Progress: { $count }
tasks-stats-done = Done: { $count }
tasks-stats-blocked = Blocked: { $count }
tasks-stats-cancelled = Cancelled: { $count }
tasks-stats-dependencies = Dependencies: { $count }
tasks-stats-cycles-yes = Cycles: Yes (run 'cycles' to see)
tasks-stats-cycles-no = Cycles: None

# Errors
error-not-initialized = Tasks not initialized
error-task-not-found = Task { $id } not found
