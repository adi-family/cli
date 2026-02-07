adi-agent-loop-http, rust, http, rest-api, agent-loop, llm-agents

## Overview
- HTTP server exposing agent loop capabilities
- REST API for agent operations

## Endpoints
- `POST /api/run` - Run agent with a task
- `GET /api/status` - Get agent status
- `POST /api/interrupt` - Interrupt running agent
- `GET /api/history` - Get conversation history

## Usage
```bash
adi-agent-http --port 8080
```
