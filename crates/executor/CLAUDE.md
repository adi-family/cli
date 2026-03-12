adi-executor, rust, docker, task-runner, async-jobs, http-api

## Overview
- Docker-based task execution service with async job management
- Runs ANY Docker image - no special worker code required in user images
- Injects `adi-worker` binary into containers via volume mount
- Simple entrypoint-based execution (no HTTP/socket communication)
- Supports output handlers (GitHub branch push, webhooks)

## Architecture

```
┌────────────────────────────────────────────────────────┐
│  User Container (any Docker image)                     │
│                                                        │
│  Mounts (injected by executor):                        │
│  - /usr/local/bin/adi-worker  (worker binary)          │
│  - /adi/input/request.json    (WorkerRequest)          │
│  - /adi/output/               (artifacts dir)          │
│                                                        │
│  ENTRYPOINT: /usr/local/bin/adi-worker                 │
│  ENV: ORIGINAL_CMD=<from image inspect>                │
└────────────────────────────────────────────────────────┘
```

## How It Works
1. Executor pulls user's Docker image
2. Inspects image to get original CMD/ENTRYPOINT
3. Creates container with adi-worker as entrypoint
4. Mounts: worker binary, input JSON, output dir
5. Container starts, adi-worker reads input, executes original command
6. User's command reads input, writes files to /adi/output/
7. adi-worker collects output files, writes response.json, exits
8. Executor waits for container exit, reads /adi/output/response.json

## API Endpoints
- `POST /v1/verify` - Verify Docker package is accessible
- `POST /v1/run` - Submit job for async execution, returns job_id
- `GET /v1/jobs` - List all jobs
- `GET /v1/jobs/:id` - Get job status and result

## Environment Variables (in container)
- `JOB_ID` - Unique job identifier
- `ORIGINAL_CMD` - Original image CMD/ENTRYPOINT to execute

## Input/Output Convention
- Input: `/adi/input/request.json` contains WorkerRequest
- Output: Place files in `/adi/output/` directory
- Response: `/adi/output/response.json` written by adi-worker
- Stdout/stderr: Captured and returned in response

## Request/Response Types

### Package (tagged enum by `type`)
```json
{ "type": "github/public", "image": "org/worker:v1" }
{ "type": "github/private", "image": "org/worker:v1", "user": "...", "token": "ghp_..." }
{ "type": "dockerhub/public", "image": "org/worker:v1" }
{ "type": "dockerhub/private", "image": "org/worker:v1", "user": "...", "password": "..." }
{ "type": "registry/public", "url": "registry.example.com/worker:v1" }
{ "type": "registry/private", "url": "registry.example.com/worker:v1", "user": "...", "password": "..." }
```

### WorkerRequest
```json
{ "type": "message", "message": "..." }
```

### OutputConfig
```json
{ "type": "github_branch", "repo": "org/repo", "branch": "feature", "token": "ghp_..." }
{ "type": "webhook", "url": "https://example.com/hook", "headers": {...} }
```

### WorkerResponse
```json
{ "success": true, "data": {...}, "files": [{ "path": "...", "content": "...", "binary": false }] }
```

## Building
```bash
cargo build -p adi-executor -p adi-worker
cargo run -p adi-executor  # Starts on port 3000
PORT=8080 cargo run -p adi-executor
```

## Cross-compiling adi-worker for containers
```bash
cross build -p adi-worker --release --target x86_64-unknown-linux-musl
cross build -p adi-worker --release --target aarch64-unknown-linux-musl
```

## Configuration
- `ADI_WORKER_BINARY` - Path to adi-worker binary (auto-detected if not set)
- `ADI_WORKER_BINARY_X86_64` - Path to x86_64 worker binary
- `ADI_WORKER_BINARY_AARCH64` - Path to aarch64 worker binary

## Code Structure
- `docker/` - Docker client for container lifecycle
- `output/` - Output handlers (GitHub, webhooks)
- `store/` - In-memory job store with DashMap
- `api/` - Axum HTTP routes
- `executor.rs` - Core orchestration logic
