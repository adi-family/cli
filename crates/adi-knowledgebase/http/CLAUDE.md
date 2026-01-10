adi-knowledgebase-http, rust, http, rest-api, knowledge-management

## Overview
- HTTP API server for ADI Knowledgebase
- RESTful endpoints for knowledge management
- Uses Axum web framework

## Endpoints
- `GET /` - Health check
- `GET /health` - Health check
- `GET /status` - Server status
- `POST /nodes` - Add a node
- `GET /nodes/:id` - Get node by ID
- `DELETE /nodes/:id` - Delete a node
- `POST /nodes/:id/approve` - Approve a node
- `GET /query?q=<question>&limit=5` - Query the knowledgebase
- `GET /subgraph?q=<question>` - Get subgraph for agent
- `GET /conflicts` - List conflicts
- `GET /orphans` - List orphan nodes
- `POST /edges` - Create an edge

## Configuration
- `PORT` env var - Default: 3001
- First CLI arg - Data directory path

## Dependencies
- `adi-knowledgebase-core` - Core library
- `axum` - Web framework
- `tower-http` - CORS, tracing
