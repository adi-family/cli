adi-coolify-http, rust, http, rest-api, coolify, deployment

## Overview
- HTTP server exposing Coolify management capabilities
- REST API for deployment operations

## Endpoints
- `GET /health` - Health check
- `GET /api/status` - Get all services status
- `GET /api/services` - List available services
- `POST /api/deploy/:service` - Deploy a service
- `GET /api/deployments/:service` - Get recent deployments
- `GET /api/logs/:deployment_uuid` - Get deployment logs

## Usage
```bash
COOLIFY_URL=https://coolify.example.com COOLIFY_API_KEY=xxx adi-coolify-http --port 8080
```

## Environment Variables
- `COOLIFY_URL` - Coolify instance URL (required)
- `COOLIFY_API_KEY` - Coolify API key (required)
- `PORT` - Server port (default: 8095)
