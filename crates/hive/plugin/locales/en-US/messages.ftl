# ============================================================================
# ADI HIVE - ENGLISH TRANSLATIONS
# ============================================================================

# Plugin metadata
plugin-name = Hive
plugin-author = ADI Team
plugin-description = Service orchestration - spawn and manage containers via WebSocket

# Command descriptions
cmd-up-help = Start services from hive.yaml
cmd-down-help = Stop services from hive.yaml
cmd-status-help = Show service status
cmd-restart-help = Restart a service
cmd-logs-help = View service logs
cmd-daemon-help = Daemon management
cmd-source-help = Source management
cmd-proxy-help = Socket activation & proxy
cmd-doctor-help = Check and fix /etc/resolver files for proxy hostnames

# Help text
hive-help-title = ADI Hive - Service Orchestration
hive-help-service-section = Service Orchestration (hive.yaml):
hive-help-up = Start services from .adi/hive.yaml
hive-help-down = Stop all services
hive-help-status = Show service status
hive-help-restart = Restart a service
hive-help-logs = View service logs
hive-help-doctor = Check and fix /etc/resolver files + flush DNS cache
hive-help-usage-section = Usage:
hive-help-up-usage = adi hive up [service...] [-d] [--name <source>]  Start services (interactive)
hive-help-down-usage = adi hive down [--name <source>]                  Stop all services
hive-help-status-usage = adi hive status [--all] [--name <source>]    Show service status
hive-help-restart-usage = adi hive restart <service> [--name <source>] Restart a service
hive-help-logs-usage = adi hive logs [service] [-f] [--tail <n>] [--level <level>]
hive-help-source-section = Source Resolution:
hive-help-source-name = --name <source>   Target a registered source by name (from any directory)
hive-help-source-omit = (omit --name)     Auto-detect from current directory (walks up to find .adi/hive.yaml)
hive-help-startup-section = Startup Options:
hive-help-startup-detached = -d                    Detached mode: start services and exit (no log streaming)
hive-help-startup-default = (default)             Interactive mode: show startup progress and stream logs
hive-help-logs-section = Logs Options:
hive-help-logs-follow = -f                    Follow logs (stream new entries)
hive-help-logs-tail = --tail <n>            Number of lines to show (default: 100)
hive-help-logs-level = --level <level>       Minimum log level (trace, debug, info, warn, error)
hive-help-daemon-section = Daemon Management:
hive-help-daemon-status = daemon status                      Check if daemon is running
hive-help-daemon-start = daemon start                       Start the daemon in the background
hive-help-daemon-stop = daemon stop                        Stop the daemon
hive-help-daemon-update = daemon update                      Update daemon to latest version
hive-help-source-mgmt-section = Source Management:
hive-help-source-list = source list                        List all sources
hive-help-source-add = source add <path> [--name <n>]     Add a new source
hive-help-source-remove = source remove <name>               Remove a source
hive-help-source-reload = source reload <name>               Reload source configuration
hive-help-source-enable = source enable <name>               Enable a disabled source
hive-help-source-disable = source disable <name>              Disable a source
hive-help-proxy-section = Socket Activation:
hive-help-proxy-status = proxy status                       Check socket activation status
hive-help-proxy-install = proxy install                      Install service with socket activation (port 80)
hive-help-proxy-uninstall = proxy uninstall                    Remove socket-activated service
hive-help-orchestrator-note = For cocoon container orchestration (signaling server mode), see:
hive-help-orchestrator-plugin = hive.orchestrator plugin (installed separately)

# Up command
hive-up-starting = Starting services from { $source }...
hive-up-starting-count = Starting { $count } services from { $source }
hive-up-started-success = { $count } service(s) started successfully
hive-up-started-partial = { $healthy } service(s) started, { $failed } failed
hive-up-no-services = No services detected
hive-up-logs-hint = Use 'adi hive logs <service>' to view logs

# Down command
hive-down-stopping = Stopping { $count } services from { $source }
hive-down-stopping-service = Stopping { $service }...
hive-down-stopped = { $service } stopped
hive-down-failed = { $service } failed
hive-down-stopped-group = Stopped { $names }
hive-down-success = { $count } service(s) stopped successfully
hive-down-partial = { $stopped } service(s) stopped, { $failed } failed

# Status display
hive-status-waiting = Waiting
hive-status-waiting-for = Waiting for { $dep }
hive-status-pre-hooks = Running pre-up hooks...
hive-status-building = Building...
hive-status-starting = Starting...
hive-status-post-hooks = Running post-up hooks...
hive-status-running = Started
hive-status-healthy = Healthy
hive-status-unhealthy = Unhealthy
hive-status-failed = Failed: { $message }

# Restart command
hive-restart-missing-service = Missing service name. Usage: adi hive restart <service> [--name <source>]
hive-restart-unknown-service = Unknown service: { $service }
hive-restart-restarting = Restarting service: { $service }
hive-restart-success = Service { $service } restarted
hive-restart-failed = Failed to restart { $service }

# Logs command
hive-logs-streaming = Streaming logs{ $service_suffix }...
hive-logs-press-ctrlc = Press Ctrl+C to stop
hive-logs-stream-ended = Log stream ended.
hive-logs-empty = No logs found.

# Config errors
hive-config-not-found = No .adi/hive.yaml found in { $path }.
hive-config-not-found-hint = Create a hive.yaml configuration file to use 'hive up'.
hive-config-not-found-name-hint = The specified source does not contain .adi/hive.yaml.
hive-config-not-found-source-hint = Tip: Use --name <source> to target a registered source from any directory.
hive-config-parse-error = Failed to parse hive.yaml: { $error }
hive-config-validation-errors = Configuration errors:

# Daemon section
hive-daemon-section = Hive Daemon
hive-daemon-running = running
hive-daemon-not-running = not running
hive-daemon-hint = adi hive daemon start

# Daemon commands
hive-daemon-help-title = ADI Hive Daemon - Background Service Manager
hive-daemon-status-running = Daemon is running
hive-daemon-status-not-running = Daemon is not running
hive-daemon-status-not-running-hint = Start with: adi hive daemon start
hive-daemon-status-unresponsive = Daemon is running (PID: { $pid }) but not responding to commands
hive-daemon-status-unresponsive-hint = Socket might be stale. Try 'adi hive daemon stop' and then 'adi hive daemon start'.
hive-daemon-already-running = Daemon is already running (PID: { $pid }). Use 'adi hive daemon stop' to stop it.
hive-daemon-starting = Starting hive daemon...
hive-daemon-started = Daemon started
hive-daemon-started-bg = Daemon started in background (PID: { $pid })
hive-daemon-forked-waiting = Daemon forked, waiting for startup...
hive-daemon-stopped = Daemon stopped (PID: { $pid })
hive-daemon-stopped-sigterm = Daemon stopped via SIGTERM (PID: { $pid })
hive-daemon-not-running-short = Daemon is not running
hive-daemon-start-timeout = Failed to start daemon - timeout waiting for startup
hive-daemon-stop-timeout = Timeout waiting for daemon to stop
hive-daemon-update-checking = Daemon version: { $version }
hive-daemon-update-cli-version = CLI version: { $version }
hive-daemon-update-stopping = Stopping daemon...
hive-daemon-update-stopped = Daemon stopped
hive-daemon-update-updating = Updating binary...
hive-daemon-update-updated = Binary updated
hive-daemon-update-restarting = Restarting daemon...
hive-daemon-update-restarted = Daemon restarted (PID: { $pid })
hive-daemon-update-restart-waiting = Daemon forked but may still be starting. Check: adi hive daemon status
hive-daemon-update-failed-recovery = Update failed, restarting daemon with current version...

# Source commands
hive-source-help-title = ADI Hive Source Management
hive-source-no-sources = No sources registered.
hive-source-no-sources-hint = Add a source with: adi hive source add <path>
hive-source-missing-path = Missing path. Usage: adi hive source add <path> [--name <name>]
hive-source-missing-name = Missing source name. Usage: adi hive source { $command } <name>
hive-source-added = { $result }
hive-source-removed = Removed source: { $name }
hive-source-reloaded = Reloaded source: { $name }
hive-source-enabled = Enabled source: { $name }
hive-source-disabled = Disabled source: { $name }


# Proxy / socket activation
hive-proxy-active = Socket activation is active
hive-proxy-inactive = Socket activation is not active
hive-proxy-installed = Socket-activated service installed
hive-proxy-uninstalled = Socket-activated service removed
hive-proxy-not-installed = Socket-activated service is not installed

# Errors
error-daemon-not-running = Daemon is not running. Start it with 'adi hive daemon start'.
error-bg-unix-only = Background daemon is only supported on Unix systems.
error-unknown-command = Unknown command: { $cmd }. Run 'adi hive' for help.
error-unknown-source-command = Unknown source command: { $cmd }. Run 'adi hive source help' for help.
error-create-runtime = Failed to create runtime: { $error }
error-start-log-stream = Failed to start log stream: { $error }
error-get-logs = Failed to get logs: { $error }
error-stream = Stream error: { $error }
error-register-source = Failed to register source: { $error }
error-start-source = Failed to start source: { $error }
error-init-service-manager = Failed to initialize service manager: { $error }
error-sort-services = Failed to sort services: { $error }
error-restart-service = Failed to restart service: { $error }
error-start-daemon = Failed to start adi daemon: { $error }
error-list-services = Failed to list services: { $error }
error-start-hive-service = Failed to start hive service: { $error }
error-get-current-dir = Failed to get current directory: { $error }
error-list-sources = Failed to list sources: { $error }
error-add-source = Failed to add source: { $error }
error-remove-source = Failed to remove source: { $error }
error-reload-source = Failed to reload source: { $error }
error-enable-source = Failed to enable source: { $error }
error-disable-source = Failed to disable source: { $error }
error-spawn-daemon-thread = Failed to spawn daemon thread: { $error }
error-daemon-thread-terminated = Daemon thread terminated unexpectedly
error-build-tokio-runtime = Failed to build Tokio runtime: { $error }

# UI Labels
label-status = Status
label-pid = PID
label-version = Version
label-uptime = Uptime
label-sources = Sources
label-services = Services
label-hint = Hint

# Section headers
section-services = Services
section-recent-logs = Recent Logs (issues)
section-recent-activity = Recent Activity

# Table headers
header-service = Service
header-state = State
header-health = Health
header-pid = PID
header-ports = Ports
header-url = URL
header-name = NAME
header-type = TYPE
header-path = PATH
header-services = SERVICES
header-status = STATUS

# State / status strings
state-running = running
state-stopped = stopped
state-port-conflict = port conflict
state-healthy = healthy
state-unhealthy = unhealthy
state-loaded = loaded
state-error = error: { $error }

# Summary strings
summary-running = { $count } running
summary-unhealthy = { $count } unhealthy
summary-stopped = { $count } stopped

# Down command - parallel stopping
hive-down-stopping-parallel = Stopping { $names }...

# Logs service suffix
hive-logs-service-suffix = { " " }for { $service }

# Source help
hive-source-help-commands = Commands:
hive-source-help-cmd-list = list                       List all registered sources
hive-source-help-cmd-add = add <path> [--name <n>]    Add a new source
hive-source-help-cmd-remove = remove <name>              Remove a source
hive-source-help-cmd-reload = reload <name>              Reload source configuration
hive-source-help-cmd-enable = enable <name>              Enable a disabled source
hive-source-help-cmd-disable = disable <name>             Disable a source (stops services)
hive-source-help-usage = Usage:
hive-source-help-usage-list = adi hive source list
hive-source-help-usage-add = adi hive source add ~/projects/myapp
hive-source-help-usage-add-name = adi hive source add ~/projects/myapp --name myapp
hive-source-help-usage-remove = adi hive source remove myapp
hive-source-help-usage-reload = adi hive source reload myapp
hive-source-help-sources-desc = Sources are configuration directories containing:
hive-source-help-yaml-desc = .adi/hive.yaml  (YAML configuration, read-only)
hive-source-help-sqlite-desc = hive.db         (SQLite configuration, read-write)
hive-source-help-default = The default source is always ~/.adi/hive/

# Doctor command
doctor-no-hosts-found = No proxy hosts found in hive.yaml
doctor-resolver-ok = exists
doctor-resolver-missing = missing
doctor-creating-resolvers = Creating /etc/resolver files (requires sudo)...
doctor-resolvers-created = Resolver files created
doctor-resolver-failed = Failed to create /etc/resolver/{ $tld }
doctor-flushing-dns = Flushing DNS cache...
doctor-dns-flushed = DNS cache flushed
doctor-dns-flush-failed = Failed to flush DNS cache
doctor-non-macos-hint = DNS resolver setup is only needed on macOS

# Uptime formatting
uptime-days = { $days }d { $hours }h { $mins }m
uptime-hours = { $hours }h { $mins }m { $secs }s
uptime-minutes = { $mins }m { $secs }s
uptime-seconds = { $secs }s
