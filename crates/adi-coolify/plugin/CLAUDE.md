adi-coolify-plugin, rust, plugin, cli, coolify, deployment

## Overview
- Plugin wrapper for ADI CLI integration
- Provides CLI commands for Coolify deployment management
- Uses adi-coolify-core for API operations

## Commands
- `status` - Show status of all services
- `deploy <service|all>` - Deploy a service (use --force for rebuild)
- `watch <service>` - Watch deployment progress
- `logs <service>` - Show deployment logs
- `list <service> [n]` - List recent deployments
- `services` - List available services
- `config` - Show/set configuration

## Configuration
Config loaded from (priority order):
1. Environment: `ADI_PLUGIN_ADI_COOLIFY_<KEY>`
2. Project: `.adi/plugins/adi.coolify.toml`
3. User: `~/.config/adi/plugins/adi.coolify.toml`

Keys:
- `url` - Coolify instance URL
- `api_key` - API token (encrypted at rest)

## Usage
```bash
adi coolify status
adi coolify deploy auth
adi coolify deploy all --force
adi coolify logs platform
```
