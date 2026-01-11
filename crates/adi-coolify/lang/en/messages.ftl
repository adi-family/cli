# ADI Coolify Plugin - English Translations

# Commands
cmd-status = Show status of all services
cmd-deploy = Deploy a service
cmd-watch = Watch deployment progress
cmd-logs = Show deployment logs
cmd-list = List recent deployments
cmd-services = List available services
cmd-config = Show current configuration
cmd-config-set = Set a config value

# Help
help-title = ADI Coolify - Deployment Management
help-commands = Commands
help-services = Services
help-config = Configuration
help-usage = Usage: adi coolify <command> [args]

# Service names
svc-auth = Auth API
svc-platform = Platform API
svc-signaling = Signaling Server
svc-web = Web UI
svc-analytics-ingestion = Analytics Ingestion
svc-analytics = Analytics API
svc-registry = Plugin Registry

# Status
status-title = ADI Deployment Status
status-service = SERVICE
status-name = NAME
status-status = STATUS
status-healthy = healthy
status-unhealthy = unhealthy
status-unknown = unknown
status-building = building
status-running = running
status-queued = queued
status-finished = finished
status-failed = failed
status-error = error

# Deploy
deploy-starting = Deploying services...
deploy-started = Started
deploy-failed = Failed
deploy-uuid = Deployment UUIDs
deploy-use-watch = Use 'adi coolify watch <service>' to monitor progress
deploy-service-required = Service name required. Usage: deploy <service|all> [--force]
deploy-unknown-service = Unknown service '{ $service }'. Available: { $available }

# Watch
watch-title = Watching { $service } deployments...
watch-latest = Latest deployment
watch-uuid = UUID
watch-status = Status
watch-commit = Commit
watch-no-deployments = No deployments found for { $service }
watch-live-tip = Note: For live watching, use: adi workflow deploy { $service }
watch-service-required = Service name required. Usage: watch <service>

# Logs
logs-title = Deployment logs for { $service }
logs-deployment = Deployment
logs-no-logs = No logs available
logs-service-required = Service name required. Usage: logs <service>

# List
list-title = Recent deployments for { $service }
list-created = CREATED
list-commit = COMMIT
list-service-required = Service name required. Usage: list <service> [count]

# Services
services-title = Available Services
services-id = ID
services-uuid = UUID

# Config
config-title = ADI Coolify Configuration
config-current = Current Values
config-files = Config Files
config-user = User
config-project = Project
config-env-vars = Environment Variables
config-set-usage = Set config
config-encryption = Encryption
config-encrypted-at-rest = (secret, encrypted at rest)
config-encrypted = (encrypted)
config-not-set = (not set)
config-unavailable = (unavailable)
config-no-project = (no project)
config-encryption-algo = Secrets are encrypted using ChaCha20-Poly1305.
config-master-key = Master key stored at: ~/.config/adi/secrets.key

# Config set
config-set-success = Set { $key } = { $value } in { $level } config
config-set-file = File: { $path }
config-set-usage-full = Usage: config set <key> <value> [--user|--project]
config-unknown-key = Unknown config key: '{ $key }'. Valid keys: url, api_key
config-no-project-dir = No project directory set. Run from a project directory.
config-save-failed = Failed to save config: { $error }

# Errors
error-api-key-not-set = API key not configured. Set via:
error-api-key-env = - Environment: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<key>
error-api-key-user = - User config: adi coolify config set api_key <key>
error-api-key-project = - Project config: adi coolify config set api_key <key> --project
error-request-failed = Request failed: { $error }
error-json-parse = JSON parse error: { $error }
error-unknown-command = Unknown command: { $command }
error-invalid-context = Invalid context: { $error }
error-invalid-response = Invalid response format
error-no-deployment-uuid = No deployment UUID
error-unknown-service = Unknown service: { $service }
