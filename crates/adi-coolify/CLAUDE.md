adi-coolify, rust, coolify, deployment, devops

## Overview
- Coolify integration for ADI
- Provides API client, HTTP server, and CLI plugin
- Manages deployments, monitoring, and service status

## Components
- `core/` - Core library with async Coolify API client
- `http/` - HTTP server exposing REST API
- `plugin/` - CLI plugin for adi integration

## Architecture
```
┌─────────────────────────────────────┐
│   adi-coolify-plugin                │
│   (CLI interface via ABI)           │
└────────────┬────────────────────────┘
             │
┌────────────▼────────────────────────┐
│   adi-coolify-http                  │
│   (REST API server)                 │
└────────────┬────────────────────────┘
             │
┌────────────▼────────────────────────┐
│   adi-coolify-core                  │
│   • CoolifyClient - API client      │
│   • Service - Service definitions   │
│   • Deployment - Deployment status  │
└─────────────────────────────────────┘
```

## Quick Start
```bash
# CLI usage
adi coolify status
adi coolify deploy auth
adi coolify logs platform

# HTTP server
COOLIFY_API_KEY=xxx adi-coolify-http
```

## Configuration
- `COOLIFY_URL` - Coolify instance URL
- `COOLIFY_API_KEY` - API token
