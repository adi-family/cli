# ============================================================================
# ADI COCOON SPAWNER - ENGLISH TRANSLATIONS
# ============================================================================

# Plugin metadata
plugin-name = Cocoon Spawner
plugin-author = ADI Team
plugin-description = Docker-based cocoon lifecycle management via signaling server

# Command descriptions
cmd-run-help = Start spawner in foreground
cmd-status-help = Show spawner status
cmd-list-help = List active cocoons

# Help text
spawner-help-title = ADI Cocoon Spawner - Docker Container Lifecycle
spawner-help-usage-section = Usage:
spawner-help-run-usage = adi cocoon-spawner run      Start spawner (foreground)
spawner-help-status-usage = adi cocoon-spawner status   Show spawner status
spawner-help-list-usage = adi cocoon-spawner list     List active cocoons

# Start command
spawner-starting = Starting cocoon spawner...
spawner-connected = Connected to signaling server
spawner-registered = Registered as hive: { $hive_id }
spawner-stopped = Spawner stopped

# Status display
spawner-status-running = running
spawner-status-stopped = stopped

# List display
spawner-list-empty = No active cocoons
spawner-list-header = Active Cocoons

# Errors
error-config = Configuration error: { $error }
error-docker = Docker connection failed: { $error }
error-unknown-command = Unknown command: { $cmd }. Run 'adi cocoon-spawner' for help.
error-spawn-daemon-thread = Failed to spawn daemon thread: { $error }
error-daemon-thread-terminated = Daemon thread terminated unexpectedly
error-build-tokio-runtime = Failed to build Tokio runtime: { $error }

# UI Labels
label-status = Status
label-hive-id = Hive ID
label-kinds = Cocoon Kinds
label-active = Active Cocoons
label-max = Max Concurrent
