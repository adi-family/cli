adi-coolify-core, rust, coolify, api-client, deployment

## Overview
- Core library for Coolify integration
- Async Coolify API client
- Service management and deployment operations

## Architecture
- **CoolifyClient**: Async API client for Coolify
- **Service**: Service definition with metadata
- **Deployment**: Deployment status and logs
- **Error**: Typed error handling

## Key Types
- `CoolifyClient` - Async HTTP client for Coolify API
- `Service` - Service definition (id, name, uuid, status)
- `Deployment` - Deployment info (uuid, status, logs, commit)
- `DeploymentStatus` - Queued/Building/Running/Failed/Success
- `CoolifyError` - Typed error variants

## Usage
```rust
let client = CoolifyClient::new("https://coolify.example.com", "api-key")?;

// Get all services status
let services = client.list_services().await?;

// Deploy a service
let deployment = client.deploy("service-uuid", false).await?;

// Get deployment logs
let logs = client.get_deployment_logs(&deployment.uuid).await?;
```
