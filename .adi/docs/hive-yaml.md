# Hive YAML Specification

**Version:** 0.1.0-draft  
**Status:** Draft  
**Authors:** ADI Team  
**Created:** 2026-01-25

## Abstract

This document specifies the `hive.yaml` configuration format for Hive, a plugin-based universal process orchestrator. Hive manages heterogeneous services through a declarative configuration file with integrated HTTP/WebSocket reverse proxy capabilities. All functionality—runners, environment providers, health checks, port allocation, and logging—is implemented via a plugin system.

## Table of Contents

1. [Terminology](#1-terminology)
2. [File Format](#2-file-format)
3. [Top-Level Fields](#3-top-level-fields)
4. [Plugin System](#4-plugin-system)
5. [Proxy Configuration](#5-proxy-configuration)
6. [Services](#6-services)
7. [Runner Plugins](#7-runner-plugins)
8. [Environment Plugins](#8-environment-plugins)
9. [Health Check Plugins](#9-health-check-plugins)
10. [Log Plugins](#10-log-plugins)
11. [Dependencies](#11-dependencies)
12. [Routing](#12-routing)
13. [Build Configuration](#13-build-configuration)
14. [Restart Policies](#14-restart-policies)
15. [Rollout Plugins](#15-rollout-plugins)
16. [Variable Interpolation](#16-variable-interpolation)
17. [Architecture](#17-architecture)
18. [Examples](#18-examples)

---

## 1. Terminology

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

| Term | Definition |
|------|------------|
| **Hive** | The orchestrator process that manages services and routes traffic |
| **Service** | A managed unit of execution |
| **Plugin** | A modular component providing specific functionality |
| **Runner** | Plugin that executes services (process, docker, etc.) |
| **Environment Provider** | Plugin that supplies environment variables |
| **Health Checker** | Plugin that probes service readiness |
| **Port Provider** | Plugin that allocates ports |
| **Log Handler** | Plugin that processes service output |
| **Route** | An HTTP path prefix mapped to a service |
| **Proxy** | The built-in HTTP/WebSocket reverse proxy |

---

## 2. File Format

### 2.1 File Location

The configuration file MUST be located at `.adi/hive.yaml` relative to the project root.

### 2.2 File Encoding

The file MUST be encoded in UTF-8.

### 2.3 YAML Version

The file MUST be valid YAML 1.2.

### 2.4 Working Directory Resolution

All relative paths in the configuration file MUST be resolved relative to the **project root** (parent directory of `.adi/`).

```yaml
working_dir: crates/adi-auth  # Resolves to <project>/crates/adi-auth
```

---

## 3. Top-Level Fields

```yaml
version: "1"           # REQUIRED
defaults: { ... }      # OPTIONAL
proxy: { ... }         # OPTIONAL
environment: { ... }   # OPTIONAL
services: { ... }      # REQUIRED
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | string | REQUIRED | Schema version. MUST be `"1"` |
| `defaults` | object | OPTIONAL | Default configuration for plugins |
| `proxy` | object | OPTIONAL | Proxy configuration |
| `environment` | object | OPTIONAL | Global environment configuration |
| `services` | object | REQUIRED | Service definitions |

---

## 4. Plugin System

Hive uses a plugin architecture where all functionality is provided by plugins. Plugins are organized by type.

### 4.1 Plugin Types

| Type | Plugin ID Prefix | Purpose | Built-in Plugins |
|------|------------------|---------|------------------|
| `parse` | `hive.parse.*` | Parse-time variable interpolation | `env`, `service` |
| `runner` | `hive.runner.*` | Execute services | `script` |
| `env` | `hive.env.*` | Provide environment variables | `static` |
| `health` | `hive.health.*` | Check service readiness | `http`, `tcp`, `cmd` |
| `log` | `hive.log.*` | Handle service output | `file`, `stdout` |
| `rollout` | `hive.rollout.*` | Control restart/deploy strategy | `recreate` |

### 4.2 Plugin Defaults

Use `defaults:` to set default configuration for plugins. Keys are full plugin identifiers.

```yaml
defaults:
  hive.parse.ports:
    prefix: "adi-"            # Prefix for ports-manager keys
  
  hive.runner.docker:
    socket: /var/run/docker.sock
  
  hive.env.dotenv:
    files:
      - .env
      - .env.local
  
  hive.log.file:
    dir: .hive/logs
    rotate: true
    max_size: 10MB
  
  hive.health.http:
    timeout: 5s
```

Services inherit these defaults. Service-level config overrides defaults:

```yaml
defaults:
  hive.log.file:
    dir: .hive/logs
    rotate: true

services:
  auth:
    log:
      type: file
      file:
        # Inherits dir: .hive/logs and rotate: true from defaults
        max_size: 20MB    # Override just this field
```

### 4.3 Built-in vs External Plugins

Built-in plugins are always available. External plugins are auto-installed when first referenced (see 4.4).

### 4.4 Plugin Capability Matrix

Some providers offer both parse-time and environment plugins with different use cases:

| Provider | Parse-time (`${...}`) | Environment (`environment:`) |
|----------|----------------------|------------------------------|
| vault | `hive.parse.vault` | `hive.env.vault` |
| | Single value: `${vault.secret/path.key}` | Bulk load from path |
| 1password | `hive.parse.1password` | `hive.env.1password` |
| | Single item: `${op.item.field}` | Load entire vault/item |

**When to use parse-time:** You need a single secret value inline (e.g., in port configuration).

**When to use environment:** You want to load multiple secrets at once into environment variables.

```yaml
# Parse-time - single value interpolation:
rollout:
  recreate:
    ports:
      http: ${vault.secret/ports.api}    # One value from Vault

# Environment - bulk load:
environment:
  vault:
    path: secret/data/adi/auth           # Load ALL keys from this path
```

### 4.5 Plugin Resolution and Auto-Install

When a service uses `type: X`, Hive:
1. Checks if plugin `X` is built-in
2. If not built-in, checks if plugin is already installed
3. If not installed, **auto-installs** the plugin from registry (`hive.<type>.<name>`)
4. Validates plugin-specific configuration
5. Invokes the plugin with the service configuration

**Auto-install naming convention:**
```
type: loki  →  auto-installs: hive.log.loki
type: docker  →  auto-installs: hive.runner.docker
type: vault  →  auto-installs: hive.env.vault
type: blue-green  →  auto-installs: hive.rollout.blue-green
```

**Example - Auto-install on first run:**
```yaml
# First time running this config:
# - hive.log.loki will be auto-installed
# - hive.runner.docker will be auto-installed

services:
  postgres:
    runner:
      type: docker          # Auto-installs hive.runner.docker
      docker:
        image: postgres:15
    log:
      - type: file          # Built-in, no install needed
        file:
          path: .hive/logs/postgres.log
      - type: loki          # Auto-installs hive.log.loki
        loki:
          url: http://loki:3100
```

**Disable auto-install:**

To prevent auto-installation (e.g., in CI/CD), set environment variable:
```bash
HIVE_AUTO_INSTALL=false hive up
```

When disabled, Hive will fail with an error listing missing plugins.

---

## 5. Proxy Configuration

The `proxy` section configures the built-in HTTP/WebSocket reverse proxy. Domains are auto-detected from service `route` definitions.

```yaml
proxy:
  bind:
    - "0.0.0.0:80"
    - "0.0.0.0:443"
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `bind` | string or array | OPTIONAL | `["127.0.0.1:8080"]` | Address(es) and port(s) to bind |

If `proxy` is omitted, Hive MUST still start the proxy on the default bind address.

### 5.1 Domain Auto-Detection

Domains are extracted from service `proxy.route` fields. When a route includes a domain (e.g., `adi.local/api/auth`), Hive automatically:
1. Extracts the domain (`adi.local`)
2. Routes requests with matching `Host` header to that service
3. Strips the domain from the path for proxying

---

## 6. Services

The `services` section defines managed services. Each key is the service name.

```yaml
services:
  <service-name>:
    runner: { ... }           # REQUIRED
    rollout: { ... }          # OPTIONAL - port allocation and deployment strategy
    proxy: { ... }            # OPTIONAL - HTTP exposure config
    depends_on: [ ... ]       # OPTIONAL
    healthcheck: { ... }      # OPTIONAL
    environment: { ... }      # OPTIONAL
    log: { ... }              # OPTIONAL
    build: { ... }            # OPTIONAL
    restart: <string>         # OPTIONAL
```

### 6.1 Service Name

Service names MUST:
- Contain only lowercase letters, numbers, hyphens, and underscores
- Start with a letter
- Be unique within the configuration

### 6.2 Common Service Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `runner` | object | REQUIRED | - | Runner plugin configuration |
| `rollout` | object | OPTIONAL | - | Port allocation and deployment strategy. Required if `proxy` or `healthcheck` is configured |
| `proxy` | object | OPTIONAL | - | HTTP exposure configuration (external network) |
| `depends_on` | array | OPTIONAL | `[]` | Services that MUST be healthy before starting |
| `healthcheck` | object | OPTIONAL | - | Health check plugin configuration |
| `environment` | object | OPTIONAL | `{}` | Environment plugin configuration |
| `log` | object | OPTIONAL | - | Log plugin configuration |
| `build` | object | OPTIONAL | - | Build configuration |
| `restart` | string | OPTIONAL | `never` | Restart policy |

**Note on `rollout`:**
- With `proxy` configured: `rollout` is REQUIRED (Hive needs to know which port to proxy to)
- With `healthcheck` configured: `rollout` is REQUIRED (health checks need a port to probe)
- Without `proxy` or `healthcheck`: `rollout` is OPTIONAL (service runs without Hive port management)

This applies to all runners (`script`, `docker`, `compose`, etc.).

### 6.3 Service Proxy Configuration

The `proxy` section configures how a service is exposed via HTTP (external network). Services without `proxy` are not exposed externally.

**Note:** Port configuration is always handled by `rollout`, regardless of runner type. See [Section 15](#15-rollout-plugins).

#### Single Proxy (shorthand)

For services with a single HTTP endpoint:

```yaml
proxy:
  host: <string>              # OPTIONAL - domain to match (default: any)
  path: <string>              # REQUIRED - HTTP path prefix (must start with /)
  port: <string>              # OPTIONAL - port reference (default: {{runtime.port.http}})
  strip_prefix: <boolean>     # OPTIONAL - strip matched path prefix
  timeout: <duration>         # OPTIONAL - proxy timeout
  buffer_size: <size>         # OPTIONAL - response buffer size
  headers:                    # OPTIONAL - custom headers
    add: { ... }
    remove: [ ... ]
```

#### Multiple Proxies

For services exposing multiple endpoints on different ports:

```yaml
proxy:
  - host: <string>            # First endpoint
    path: <string>
    port: <string>            # REQUIRED when multiple proxies
    ...
  - host: <string>            # Second endpoint
    path: <string>
    port: <string>
    ...
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `host` | string | OPTIONAL | (any) | Domain to match via `Host` header |
| `path` | string | REQUIRED | - | HTTP path prefix (must start with `/`) |
| `port` | string | OPTIONAL | `{{runtime.port.http}}` | Port to proxy to. Use `{{runtime.port.<name>}}` |
| `strip_prefix` | boolean | OPTIONAL | `false` | Strip matched path prefix before forwarding |
| `timeout` | duration | OPTIONAL | `60s` | Proxy request timeout |
| `buffer_size` | string | OPTIONAL | `1MB` | Response buffer size |
| `headers` | object | OPTIONAL | - | Custom header manipulation |

**Example - Single port (simple):**
```yaml
services:
  auth:
    runner:
      type: script
      script:
        run: cargo run -p adi-auth-http
        working_dir: crates/adi-auth
    rollout:
      type: recreate
      recreate:
        ports:
          http: 8012
    proxy:
      host: adi.local
      path: /api/auth
      timeout: 30s
```

**Example - Multiple ports with different proxies:**
```yaml
services:
  api:
    runner:
      type: script
      script:
        run: cargo run --bin api-server
    rollout:
      type: recreate
      recreate:
        ports:
          http: 8080
          grpc: 9090
          metrics: 9091
    proxy:
      - host: adi.local
        path: /api
        port: "{{runtime.port.http}}"
        strip_prefix: true
      - host: adi.local
        path: /grpc
        port: "{{runtime.port.grpc}}"
      - host: adi.local
        path: /metrics
        port: "{{runtime.port.metrics}}"
```

**Example - gRPC + HTTP on same service:**
```yaml
services:
  gateway:
    runner:
      type: script
      script:
        run: cargo run --bin gateway
    rollout:
      type: recreate
      recreate:
        ports:
          rest: 8080
          grpc: 50051
    proxy:
      - host: api.example.com
        path: /v1
        port: "{{runtime.port.rest}}"
      - host: grpc.example.com
        path: /
        port: "{{runtime.port.grpc}}"
    healthcheck:
      - type: http
        http:
          port: "{{runtime.port.rest}}"
          path: /health
      - type: grpc
        grpc:
          port: "{{runtime.port.grpc}}"
```

**Services without proxy (internal only):**
```yaml
services:
  worker:
    runner:
      type: script
      script:
        run: cargo run --bin worker
    rollout:
      type: recreate
      recreate:
        ports:
          http: 9000            # Internal port, not exposed via proxy
    # No proxy section = service not exposed externally
```

---

## 7. Runner Plugins

Runner plugins execute services. The `runner` field MUST specify a `type` and plugin-specific configuration.

```yaml
runner:
  type: <plugin-name>
  <plugin-name>:
    <plugin-specific-options>
```

### 7.1 Script Runner (built-in)

The built-in runner executes shell commands. Supports single commands and multi-line scripts.

```yaml
runner:
  type: script
  script:
    run: <string>               # REQUIRED
    working_dir: <string>       # OPTIONAL
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `run` | string | REQUIRED | - | Command or script to execute |
| `working_dir` | string | OPTIONAL | `.` | Working directory (relative to project root) |

**Example - Single command:**
```yaml
services:
  auth:
    runner:
      type: script
      script:
        run: cargo run -p adi-auth-http
        working_dir: crates/adi-auth
    rollout:
      type: recreate
      recreate:
        ports:
          http: 8012
    proxy:
      host: adi.local
      path: /api/auth
```

**Example - Multi-line script:**
```yaml
services:
  migrations:
    runner:
      type: script
      script:
        run: |
          set -e
          echo "Running auth migrations..."
          cd crates/adi-auth
          cargo run --bin migrate -- up
          
          echo "Running platform migrations..."
          cd ../adi-platform-api
          cargo run --bin migrate -- up
          
          echo "All migrations complete!"
    depends_on:
      - postgres
    restart: never
```

**Example - Using specific interpreter:**
```yaml
services:
  setup:
    runner:
      type: script
      script:
        run: |
          #!/usr/bin/env python3
          import os
          print("Setting up environment...")
```

**Example - Database setup:**
```yaml
services:
  setup-db:
    runner:
      type: script
      script:
        run: |
          psql -h localhost -U adi -c "CREATE DATABASE IF NOT EXISTS adi_auth;"
          psql -h localhost -U adi -c "CREATE DATABASE IF NOT EXISTS adi_platform;"
    depends_on:
      - postgres
    restart: never
```

### 7.2 External Runner Plugins

Additional runners are available as external plugins:

| Plugin | Install | Description |
|--------|---------|-------------|
| `docker` | `adi plugin install hive.runner.docker` | Docker container management |
| `compose` | `adi plugin install hive.runner.compose` | docker-compose delegation |
| `podman` | `adi plugin install hive.runner.podman` | Podman container management |
| `kubernetes` | `adi plugin install hive.runner.kubernetes` | Kubernetes pod management |
| `nix` | `adi plugin install hive.runner.nix` | Nix shell environments |

**Example - Docker runner (external plugin):**
```yaml
defaults:
  hive.runner.docker:
    socket: /var/run/docker.sock

services:
  postgres:
    runner:
      type: docker            # Auto-installs hive.runner.docker
      docker:
        image: postgres:15
        ports:
          # Format: "HOST_PORT:CONTAINER_PORT"
          # HOST_PORT = what Hive proxies to (from rollout.ports)
          # CONTAINER_PORT = what app listens on inside container
          - "{{runtime.port.db}}:5432"  # Host: 5433, Container: 5432
        volumes:
          - postgres-data:/var/lib/postgresql/data
        environment:
          POSTGRES_USER: adi
          POSTGRES_PASSWORD: adi
    rollout:
      type: recreate
      recreate:
        ports:
          db: 5433            # {{runtime.port.db}} = 5433 (host port)
```

**Note:** For all runners (including `docker`), `rollout.ports` defines the **host** ports that Hive manages for proxying and health checks. The container listens on its internal port (e.g., 5432 for postgres), while Hive routes traffic to the host port.

**Example - Docker with Hive proxy:**
```yaml
services:
  api:
    runner:
      type: docker
      docker:
        image: my-api:latest
        ports:
          # App inside container listens on 8080
          # Hive proxies to host port 3000
          - "{{runtime.port.http}}:8080"
    rollout:
      type: recreate
      recreate:
        ports:
          http: 3000          # {{runtime.port.http}} = 3000 (host port)
    proxy:
      host: adi.local
      path: /api
```

**Example - Docker with blue-green deployment:**
```yaml
services:
  api:
    runner:
      type: docker
      docker:
        image: my-api:latest
        ports:
          # {{runtime.port.http}} resolves to currently active port (3000 or 3001)
          - "{{runtime.port.http}}:8080"
    rollout:
      type: blue-green
      blue-green:
        ports:
          http:
            blue: 3000        # First instance runs on host port 3000
            green: 3001       # Second instance runs on host port 3001
    proxy:
      host: adi.local
      path: /api
    healthcheck:
      type: http
      http:
        port: "{{runtime.port.http}}"
        path: /health
```

**Example - Compose runner (external plugin):**
```yaml
services:
  infra:
    runner:
      type: compose           # Auto-installs hive.runner.compose
      compose:
        file: docker-compose.dev.yml
        service: postgres
    rollout:
      type: recreate
      recreate:
        ports:
          db: 5433            # Host port that Hive manages
```

---

## 8. Environment Plugins

Environment plugins provide environment variables to services. Multiple providers can be combined - each provider key maps directly to its plugin configuration.

### 8.1 Syntax

```yaml
environment:
  <plugin-name>:           # Plugin-specific configuration
    <plugin-options>
  <another-plugin>:        # Multiple plugins can be combined
    <plugin-options>
  static:                  # Built-in static provider (always available)
    KEY: value
```

No `type` field is needed - the key name identifies the plugin.

### 8.2 Global Environment

Global environment applies to all services:

```yaml
environment:
  static:
    RUST_LOG: info
    LOGGING_URL: http://localhost:${ports.logging}
```

### 8.3 Service Environment

Service-level environment extends and overrides global:

```yaml
services:
  auth:
    environment:
      static:
        DATABASE_URL: postgres://...
        RUST_LOG: debug    # Overrides global
```

### 8.4 Static Environment (built-in)

Inline key-value pairs. Always available without plugin installation:

```yaml
environment:
  static:
    DATABASE_URL: postgres://adi:adi@localhost:5432/adi_auth
    RUST_LOG: debug
```

### 8.5 External Environment Plugins

Additional environment providers are available as external plugins:

| Plugin | Install | Description |
|--------|---------|-------------|
| `dotenv` | `adi plugin install hive.env.dotenv` | Load from `.env` files |
| `vault` | `adi plugin install hive.env.vault` | HashiCorp Vault secrets |
| `1password` | `adi plugin install hive.env.1password` | 1Password secrets |
| `aws-secrets` | `adi plugin install hive.env.aws-secrets` | AWS Secrets Manager |

**Example - Multiple providers combined:**
```yaml
environment:
  dotenv:                  # Auto-installs hive.env.dotenv
    files:
      - .env
      - .env.local
  static:                  # Built-in, no install needed
    RUST_LOG: info
    OVERRIDE_KEY: value    # Overrides same key from dotenv
```

**Example - Vault with static fallbacks:**
```yaml
environment:
  vault:                   # Auto-installs hive.env.vault
    address: https://vault.example.com
    path: secret/data/adi/auth
    token: ${env.VAULT_TOKEN}
  static:
    RUST_LOG: debug        # Static values override vault
```

**Example - Full stack (dotenv + vault + static):**
```yaml
environment:
  dotenv:
    files:
      - .env
  vault:
    address: https://vault.example.com
    path: secret/data/adi
  static:
    RUST_LOG: info         # Highest priority
```

### 8.6 Precedence

Environment variable precedence (highest to lowest):
1. Service `static` values
2. Service plugin values (in reverse declaration order - later overrides earlier)
3. Global `static` values
4. Global plugin values (in reverse declaration order)
5. System environment variables

**Example precedence:**
```yaml
# Global
environment:
  dotenv:
    files: [.env]          # Priority 4: FOO=from-dotenv
  static:
    FOO: from-global       # Priority 3: overrides dotenv

services:
  auth:
    environment:
      vault:
        path: secret/auth  # Priority 2: FOO=from-vault (if exists)
      static:
        FOO: from-service  # Priority 1: final value = "from-service"
```

---

## 9. Health Check Plugins

Health check plugins determine when a service is ready. A service can have multiple health checks - all must pass for the service to be considered healthy.

### 9.1 Single Health Check

```yaml
healthcheck:
  type: <plugin-name>
  <plugin-name>:
    <plugin-specific-options>
    interval: <duration>      # OPTIONAL
    timeout: <duration>       # OPTIONAL
    retries: <integer>        # OPTIONAL
    start_period: <duration>  # OPTIONAL
```

### 9.2 Multiple Health Checks

```yaml
healthcheck:
  - type: http
    http:
      path: /health
      interval: 10s
  - type: cmd
    cmd:
      command: pg_isready -U adi
      interval: 5s
```

When multiple health checks are configured:
- All checks run independently at their own intervals
- Service is healthy only when ALL checks pass
- Service becomes unhealthy if ANY check fails

### 9.3 Common Fields (per plugin)

These fields are available inside each plugin's configuration block:

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `port` | string | OPTIONAL | `{{runtime.port.http}}` | Port to check. Use `{{runtime.port.<name>}}` to reference rollout ports |
| `interval` | duration | OPTIONAL | `10s` | Time between checks |
| `timeout` | duration | OPTIONAL | `5s` | Timeout for each check |
| `retries` | integer | OPTIONAL | `3` | Consecutive failures before unhealthy |
| `start_period` | duration | OPTIONAL | `0s` | Grace period before checks count |

### 9.4 HTTP Health Check (built-in)

```yaml
healthcheck:
  type: http
  http:
    port: "{{runtime.port.http}}"  # OPTIONAL, default: first port from rollout.ports
    path: /health
    method: GET               # OPTIONAL, default: GET
    status: 200               # OPTIONAL, default: 2xx
    interval: 10s
```

### 9.5 TCP Health Check (built-in)

```yaml
healthcheck:
  type: tcp
  tcp:
    port: "{{runtime.port.db}}"   # OPTIONAL, default: first port from rollout.ports
    interval: 5s
```

### 9.6 Command Health Check (built-in)

```yaml
healthcheck:
  type: cmd
  cmd:
    command: pg_isready -U adi
    working_dir: .            # OPTIONAL
    interval: 5s
```

### 9.7 External Health Check Plugins

Additional health checks are available as external plugins:

| Plugin | Install | Description |
|--------|---------|-------------|
| `grpc` | `adi plugin install hive.health.grpc` | gRPC health check protocol |
| `postgres` | `adi plugin install hive.health.postgres` | PostgreSQL-specific check |
| `redis` | `adi plugin install hive.health.redis` | Redis PING check |
| `mysql` | `adi plugin install hive.health.mysql` | MySQL connection check |

### 9.8 Duration Format

Durations MUST be specified as a number followed by a unit:
- `s` - seconds (e.g., `30s`)
- `m` - minutes (e.g., `5m`)
- `ms` - milliseconds (e.g., `500ms`)

---

## 10. Log Plugins

Log plugins handle service output.

### 10.1 Global Log Configuration

```yaml
defaults:
  hive.log.file:
    dir: .hive/logs
    rotate: true
    max_size: 10MB
```

### 10.2 Service Log Configuration

```yaml
services:
  auth:
    log:
      type: file
      file:
        path: logs/auth.log
```

### 10.3 File Log Plugin (built-in)

Write logs to files:

```yaml
log:
  type: file
  file:
    path: .hive/logs/${service}.log   # ${service} = service name
    rotate: true
    max_size: 10MB
    max_files: 5
```

### 10.4 Stdout Log Plugin (built-in)

Stream logs to console with service name prefix and optional color coding:

```yaml
log:
  type: stdout
  stdout:
    prefix: true              # OPTIONAL, default: true - prefix lines with service name
    color: auto               # OPTIONAL: auto, always, never
```

### 10.5 External Log Plugins

Additional log handlers are available as external plugins:

| Plugin | Install | Description |
|--------|---------|-------------|
| `loki` | `adi plugin install hive.log.loki` | Send to Grafana Loki |
| `cloudwatch` | `adi plugin install hive.log.cloudwatch` | Send to AWS CloudWatch |

---

## 11. Dependencies

The `depends_on` field specifies services that MUST be healthy before the current service starts.

```yaml
depends_on:
  - postgres
  - redis
```

Hive MUST:
1. Build a dependency graph from all `depends_on` declarations
2. Detect circular dependencies and fail with an error
3. Start services in topological order
4. Wait for each dependency's health check to pass before starting dependents
5. If a dependency has no health check, wait for its process/container to be running

**Example:**
```yaml
services:
  postgres:
    runner:
      type: docker
      docker:
        image: postgres:15
    healthcheck:
      type: cmd
      cmd:
        command: pg_isready -U adi

  auth:
    runner:
      type: script
      script:
        run: cargo run -p adi-auth-http
    depends_on:
      - postgres
```

---

## 12. Routing

The `proxy` section configures HTTP reverse proxy routing using explicit `host` and `path` fields.

### 12.1 Route Format

```yaml
proxy:
  host: <domain>           # OPTIONAL - match Host header
  path: <path>             # REQUIRED - must start with /
```

| Field | Example | Description |
|-------|---------|-------------|
| `host` | `adi.local` | Match requests with this `Host` header |
| `path` | `/api/auth` | Match requests starting with this path |

**Examples:**

| host | path | Matches |
|------|------|---------|
| (omitted) | `/api/auth` | Any host, path `/api/auth/*` |
| `adi.local` | `/api/auth` | Host `adi.local`, path `/api/auth/*` |
| `admin.adi.local` | `/` | Host `admin.adi.local`, all paths |

**Rules:**
1. `path` MUST start with `/`
2. `host` MUST NOT include protocol (no `http://`)
3. Trailing `/` in path is ignored

### 12.2 Route Matching

1. Routes with `host` are matched first (by `Host` header)
2. Within same host, longest `path` prefix wins
3. Routes without `host` match any `Host` header
4. If multiple services have identical host+path, Hive MUST fail with an error

### 12.3 WebSocket Support

Hive MUST transparently support WebSocket upgrades for all routes. When a request includes the `Upgrade: websocket` header:
- Hive MUST forward the upgrade request to the backend service
- Hive MUST NOT buffer the connection
- The backend service decides whether to accept the WebSocket upgrade

No explicit configuration is required.

---

## 13. Build Configuration

The `build` field specifies how to build a service before running.

```yaml
build:
  command: <string>         # REQUIRED
  working_dir: <string>     # OPTIONAL
  when: <string>            # OPTIONAL
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `command` | string | REQUIRED | - | Build command |
| `working_dir` | string | OPTIONAL | runner's `working_dir` or `.` | Build working directory (falls back to project root if runner has no working_dir) |
| `when` | string | OPTIONAL | `missing` | When to build |

### 13.1 Build Triggers (`when`)

| Value | Description |
|-------|-------------|
| `missing` | Build only if output doesn't exist |
| `always` | Build every time before starting |
| `never` | Never build (assume pre-built) |

---

## 14. Restart Policies

The `restart` field controls service restart behavior.

| Value | Description |
|-------|-------------|
| `never` | Never restart. Manual control only |
| `on-failure` | Restart only on non-zero exit code |
| `always` | Always restart, including after crash |
| `unless-stopped` | Like `always`, but respects manual stop |

**Default:** `never`

### 14.1 Restart Behavior

When a service crashes and restart policy applies:
1. Hive MUST wait 1 second before first restart
2. Hive MUST use exponential backoff (1s, 2s, 4s, 8s, max 60s)
3. Hive MUST reset backoff after 60 seconds of healthy running

---

## 15. Rollout Plugins

Rollout plugins control how services are started, restarted, and updated. They manage **port allocation** (internal network) and the **deployment strategy**.

```yaml
rollout:
  type: <plugin-name>         # REQUIRED - rollout strategy
  <plugin-name>:
    ports: { ... }            # REQUIRED - named port(s) for service
    <plugin-specific-options>
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | REQUIRED | Rollout strategy plugin |

### 15.1 Port Configuration

Ports are defined as a named map. Names are used to reference ports in `proxy` and `healthcheck` configurations via `{{runtime.port.<name>}}`.

```yaml
ports:
  <name>: <port-number>       # Single instance port (recreate strategy)
  <name>:                     # Blue-green: explicit blue/green ports
    blue: <port1>
    green: <port2>
```

**Examples:**
```yaml
# Single port (most common - for recreate strategy)
ports:
  http: 8080

# Multiple ports (different protocols)
ports:
  http: 8080
  grpc: 9090
  metrics: 9091

# Blue-green deployment (explicit blue/green ports)
ports:
  http:
    blue: 8080
    green: 8081
  grpc:
    blue: 9090
    green: 9091
```

### 15.2 Recreate (built-in)

Stop the old instance, then start the new one. Simple but causes downtime.

```yaml
rollout:
  type: recreate
  recreate:
    ports:
      http: 8012
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `ports` | object | REQUIRED | Named ports map (`name: port-number`) |

**Behavior:**
1. Stop old instance
2. Start new instance on the same ports
3. Wait for healthcheck (if configured)

**Use when:**
- Service is stateful and can't have two instances
- Downtime is acceptable
- No healthcheck configured

**Example - Multiple ports:**
```yaml
rollout:
  type: recreate
  recreate:
    ports:
      http: 8080
      grpc: 9090
      metrics: 9091
```

### 15.3 External Rollout Plugins

Additional rollout strategies are available as external plugins:

| Plugin | Install | Description |
|--------|---------|-------------|
| `blue-green` | `adi plugin install hive.rollout.blue-green` | Zero-downtime with traffic switching |
| `canary` | `adi plugin install hive.rollout.canary` | Gradual traffic shifting |
| `rolling` | `adi plugin install hive.rollout.rolling` | Rolling update for multiple instances |

### 15.4 Blue-Green (external plugin)

Run new instance alongside old, switch traffic when healthy. Zero-downtime updates.

**Requirements:**
- `healthcheck` MUST be configured (Hive needs to know when new instance is ready)
- `proxy` MUST be configured (Hive switches traffic via proxy)
- Each port MUST have explicit `blue` and `green` values

**Port format for blue-green** - each named port needs blue/green:

```yaml
rollout:
  type: blue-green
  blue-green:
    ports:
      http:
        blue: 8012            # First instance runs here
        green: 8013           # Second instance runs here
    healthy_duration: 10s     # OPTIONAL, default: 10s
    timeout: 60s              # OPTIONAL, default: 60s
    on_failure: keep-old      # OPTIONAL, default: keep-old
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `ports` | object | REQUIRED | - | Named ports, each with `blue` and `green` values |
| `healthy_duration` | duration | OPTIONAL | `10s` | New instance must be healthy for this long |
| `timeout` | duration | OPTIONAL | `60s` | Max time to wait for new instance |
| `on_failure` | string | OPTIONAL | `keep-old` | Action if new instance fails |

**on_failure options:**
- `keep-old` - Keep old instance running, log error
- `abort` - Stop both instances, fail loudly

**Behavior:**
1. Start new instance on the alternate ports
2. Wait for healthcheck to pass
3. Wait for `healthy_duration` to confirm stability
4. Switch all proxy routes to new instance ports
5. Stop old instance

Hive tracks which color (blue/green) is currently active. On restart/update:
- Start new instance on inactive color's port
- Once healthy → switch all proxies to new port
- Stop old instance
- Next restart uses the now-free color

**Example - Single endpoint:**
```yaml
services:
  auth:
    runner:
      type: script
      script:
        run: cargo run -p adi-auth-http
        working_dir: crates/adi-auth
    rollout:
      type: blue-green
      blue-green:
        ports:
          http:
            blue: 8012
            green: 8013
        healthy_duration: 10s
        timeout: 60s
        on_failure: keep-old
    proxy:
      host: adi.local
      path: /api/auth
      port: "{{runtime.port.http}}"
    healthcheck:
      type: http
      http:
        port: "{{runtime.port.http}}"
        path: /health
        interval: 5s
```

**Example - Multiple endpoints with blue-green:**
```yaml
services:
  gateway:
    runner:
      type: script
      script:
        run: cargo run --bin gateway
    rollout:
      type: blue-green
      blue-green:
        ports:
          http:
            blue: 8080
            green: 8081
          grpc:
            blue: 9090
            green: 9091
        healthy_duration: 10s
    proxy:
      - host: api.example.com
        path: /
        port: "{{runtime.port.http}}"
      - host: grpc.example.com
        path: /
        port: "{{runtime.port.grpc}}"
    healthcheck:
      - type: http
        http:
          port: "{{runtime.port.http}}"
          path: /health
      - type: grpc
        grpc:
          port: "{{runtime.port.grpc}}"
```

**With ports-manager:**
```yaml
rollout:
  type: blue-green
  blue-green:
    ports:
      http:
        blue: ${ports.auth-http-blue}
        green: ${ports.auth-http-green}
      grpc:
        blue: ${ports.auth-grpc-blue}
        green: ${ports.auth-grpc-green}
```

### 15.5 Blue-Green Sequence Diagram

```
Config: rollout.blue-green.ports.http: { blue: 8012, green: 8013 }
State:  blue (8012) is active

Time    Port 8012            Hive Proxy           Port 8013
─────────────────────────────────────────────────────────────────
  0     [running] ◄───────── [route:8012]
        
  1     [running] ◄───────── [route:8012]         [starting...]
        
  2     [running] ◄───────── [route:8012]         [healthcheck...]
        
  3     [running] ◄───────── [route:8012]         [healthy ✓]
        
  4     [running] ◄───────── [route:8012]         [healthy 10s ✓]
        
  5     [running]            [route:8013] ────►   [active]
        
  6     [stopping...]        [route:8013] ────►   [active]
        
  7     [stopped]            [route:8013] ────►   [active]

State:  green (8013) is now active (next deploy will use blue/8012)
```

### 15.6 Blue-Green Failure Scenarios

**Scenario: New instance fails healthcheck**
```
Time    Blue (8012)          Hive Proxy           Green (8013)
─────────────────────────────────────────────────────────────────
  0     [running] ◄───────── [route:blue]
  1     [running] ◄───────── [route:blue]         [starting...]
  2     [running] ◄───────── [route:blue]         [healthcheck ✗]
  3     [running] ◄───────── [route:blue]         [retry...]
  4     [running] ◄───────── [route:blue]         [timeout!]
  5     [running] ◄───────── [route:blue]         [killed]
        
Result: Blue continues serving traffic, error logged
        Green is free for next attempt
```

**Scenario: New instance healthy then crashes during healthy_duration**
```
Time    Blue (8012)          Hive Proxy           Green (8013)
─────────────────────────────────────────────────────────────────
  0     [running] ◄───────── [route:blue]
  1     [running] ◄───────── [route:blue]         [starting...]
  2     [running] ◄───────── [route:blue]         [healthy ✓]
  3     [running] ◄───────── [route:blue]         [crashed ✗]
  4     [running] ◄───────── [route:blue]         [killed]
        
Result: healthy_duration not met, blue continues serving
```

---

## 16. Variable Interpolation

Hive supports two types of variable interpolation:
1. **Parse-time plugins** (`${plugin.key}`) - resolved when YAML is parsed, via plugins
2. **Runtime templates** (`{{runtime...}}`) - resolved at service start from service config

### 16.1 Parse-Time Plugins (`${...}`)

Parse-time interpolation uses plugins to resolve values **before** services start. Plugins are invoked during YAML parsing.

```yaml
${<plugin>.<key>}         # Value from parse-time plugin
${<plugin>.<key>:-default}  # With default value if plugin returns nothing
```

#### Built-in Parse-Time Plugins

| Plugin | Syntax | Description |
|--------|--------|-------------|
| `env` | `${env.VAR}` | System environment variable |
| `service` | `${service.name}` | Current service name |

#### External Parse-Time Plugins

| Plugin | Install | Syntax | Description |
|--------|---------|--------|-------------|
| `ports` | `adi plugin install hive.parse.ports` | `${ports.<name>}` | Port from `ports-manager get <name>` |
| `vault` | `adi plugin install hive.parse.vault` | `${vault.<path>}` | Secret from HashiCorp Vault |
| `op` | `adi plugin install hive.parse.1password` | `${op.<item>.<field>}` | Secret from 1Password CLI |
| `aws-ssm` | `adi plugin install hive.parse.aws-ssm` | `${aws-ssm.<param>}` | AWS SSM Parameter Store |

**Example - Environment variables:**
```yaml
environment:
  static:
    LOG_LEVEL: ${env.LOG_LEVEL:-info}
    DATABASE_URL: ${env.DATABASE_URL}
```

**Example - ports-manager plugin:**
```yaml
rollout:
  type: recreate
  recreate:
    ports:
      http: ${ports.api}          # Resolved from: ports-manager get api
      grpc: ${ports.api-grpc}     # Resolved from: ports-manager get api-grpc
```

**Example - Vault secrets at parse time:**
```yaml
# Requires: adi plugin install hive.parse.vault
# Set VAULT_ADDR and VAULT_TOKEN environment variables

environment:
  static:
    DB_PASSWORD: ${vault.secret/data/db.password}
    API_KEY: ${vault.secret/data/api.key}
```

**Example - 1Password secrets:**
```yaml
# Requires: adi plugin install hive.parse.1password
# Requires: 1Password CLI (op) authenticated

environment:
  static:
    GITHUB_TOKEN: ${op.Developer.github-token}
    AWS_SECRET: ${op.AWS.secret-key}
```

#### Parse-Time Plugin Configuration

Configure parse-time plugins in `defaults`:

```yaml
defaults:
  hive.parse.vault:
    address: https://vault.example.com
    # token from VAULT_TOKEN env var
  
  hive.parse.ports:
    prefix: "adi-"            # Prefix for ports-manager keys
```

### 16.2 Runtime Templates (`{{runtime...}}`)

Resolved **at service start** from the current service's configuration. Use this to reference values defined within the same service.

```yaml
{{runtime.port.<name>}}   # Port value from rollout.ports.<name>
```

**Why separate from parse-time?**
- Parse-time (`${...}`) runs before full config is loaded - can't reference other config values
- Runtime (`{{runtime...}}`) runs after config is parsed - can reference `rollout.ports`, etc.

**Example - Docker port mapping:**
```yaml
services:
  api:
    runner:
      type: docker
      docker:
        image: my-api:latest
        ports:
          - "{{runtime.port.http}}:8080"     # Host port from rollout -> container 8080
          - "{{runtime.port.grpc}}:9090"     # Host port from rollout -> container 9090
    rollout:
      type: recreate
      recreate:
        ports:
          http: 3000              # {{runtime.port.http}} = 3000
          grpc: 3001              # {{runtime.port.grpc}} = 3001
```

**Example - Combining parse-time and runtime:**
```yaml
services:
  api:
    runner:
      type: docker
      docker:
        ports:
          - "{{runtime.port.http}}:8080"
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.api}      # Parse-time: ${ports.api} -> 3000 (from ports-manager)
                                  # Runtime: {{runtime.port.http}} -> 3000 (from rollout.ports)
```

**Example - Blue-green deployment:**
```yaml
services:
  api:
    runner:
      type: docker
      docker:
        ports:
          - "{{runtime.port.http}}:8080"    # Resolves to currently active port (blue or green)
    rollout:
      type: blue-green
      blue-green:
        ports:
          http:
            blue: 3000            # Hive picks active color at runtime
            green: 3001
```

### 16.3 Where Runtime Templates Can Be Used

Runtime templates (`{{runtime.port.<name>}}`) are valid in:
- `runner.docker.ports` - container port mapping
- `runner.docker.environment` - passing port to container
- `environment.static` - service environment variables
- `healthcheck.*.port` - health check target (alternative to port name)

**Example - Passing port to container:**
```yaml
services:
  api:
    runner:
      type: docker
      docker:
        ports:
          - "{{runtime.port.http}}:8080"
        environment:
          METRICS_PORT: "{{runtime.port.metrics}}"
    rollout:
      type: recreate
      recreate:
        ports:
          http: 8080
          metrics: 9091
```

### 16.4 Escaping

To include literal `${` or `{{`, escape with double:
```yaml
environment:
  static:
    SHELL_VAR: "$${NOT_INTERPOLATED}"      # Results in: ${NOT_INTERPOLATED}
    TEMPLATE: "{{{runtime.port.http}}}"    # Results in: {{runtime.port.http}}
```

### 16.5 Resolution Order

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Parse YAML structure                                      │
├─────────────────────────────────────────────────────────────┤
│ 2. Load parse-time plugins (hive.parse.*)                   │
├─────────────────────────────────────────────────────────────┤
│ 3. Resolve ${...} via parse-time plugins                    │
│    - ${env.VAR} → environment variable                      │
│    - ${ports.name} → ports-manager get                      │
│    - ${vault.path} → Vault secret                           │
├─────────────────────────────────────────────────────────────┤
│ 4. Validate service configurations                          │
├─────────────────────────────────────────────────────────────┤
│ 5. For each service start:                                  │
│    - Resolve {{runtime...}} templates                       │
│    - {{runtime.port.http}} → rollout.ports.http value       │
├─────────────────────────────────────────────────────────────┤
│ 6. Start service process/container                          │
└─────────────────────────────────────────────────────────────┘
```

### 16.6 Parse-Time Plugin Interface

Parse-time plugins implement the `ParsePlugin` trait:

```rust
pub trait ParsePlugin: Send + Sync {
    /// Plugin identifier (e.g., "ports", "vault")
    fn name(&self) -> &str;
    
    /// Resolve a key to a value
    /// Called for each ${plugin.key} occurrence
    fn resolve(&self, key: &str) -> Result<Option<String>>;
}
```

**Example implementation (ports-manager):**
```rust
impl ParsePlugin for PortsManagerPlugin {
    fn name(&self) -> &str { "ports" }
    
    fn resolve(&self, key: &str) -> Result<Option<String>> {
        // Execute: ports-manager get <key>
        let output = Command::new("ports-manager")
            .args(["get", key])
            .output()?;
        
        if output.status.success() {
            Ok(Some(String::from_utf8(output.stdout)?.trim().to_string()))
        } else {
            Ok(None)
        }
    }
}
```

---

## 17. Architecture

### 17.1 Component Diagram

```
                              Hive Core
┌─────────────────────────────────────────────────────────────┐
│                      Plugin Manager                          │
│  ┌─────────────────────┐  ┌─────────────────────────────┐   │
│  │     Built-in        │  │      External (auto-install) │   │
│  ├─────────────────────┤  ├─────────────────────────────┤   │
│  │ parse: env, service │  │ parse: ports, vault,        │   │
│  │ runner: script      │  │        1password, aws-ssm   │   │
│  │ env: static         │  │ runner: docker, compose,    │   │
│  │ health: http,tcp,cmd│  │         podman, kubernetes  │   │
│  │ log: file           │  │ env: dotenv, vault,         │   │
│  │ rollout: recreate   │  │      1password, aws-secrets │   │
│  │                     │  │ health: grpc, postgres,     │   │
│  │                     │  │         redis, mysql        │   │
│  │                     │  │ log: stdout, loki,          │   │
│  │                     │  │      cloudwatch             │   │
│  │                     │  │ rollout: blue-green,        │   │
│  │                     │  │          canary, rolling    │   │
│  └─────────────────────┘  └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │ Service  │   │ Service  │   │ Service  │
        │  (auth)  │   │(platform)│   │  (web)   │
        └──────────┘   └──────────┘   └──────────┘
              │               │               │
              └───────────────┼───────────────┘
                              ▼
                    ┌──────────────────┐
                    │   HTTP/WS Proxy  │
                    │   (bind: :8080)  │
                    └──────────────────┘
                              │
                              ▼
                         Clients
```

### 17.2 Startup Sequence

```
1. Parse hive.yaml
2. Load and initialize plugins
   - Discover built-in plugins
   - Load external plugins
   - Validate plugin configurations
3. Validate service configurations
   - Check for circular dependencies
   - Verify runner/health/env plugins exist
   - Validate routes (no conflicts)
4. Resolve ports via port plugins
5. Build dependency graph
6. Start services in topological order:
   For each service:
   a. Load environment via env plugins
   b. Run build command (if configured)
   c. Start runner plugin
   d. Wait for health check plugin (if configured)
   e. Register route (if configured)
7. Start proxy server
8. Enter supervision loop:
   - Monitor service health
   - Restart crashed services (per restart policy)
   - Handle SIGTERM/SIGINT gracefully
```

### 17.3 Request Flow

```
Client Request
      |
      v
+------------------+
|    Hive Proxy    |
|  (bind address)  |
+------------------+
      |
      | Match route by Host + longest path prefix
      v
+------------------+
|  Route Table     |
|  adi.local/api/auth/* --> auth:8012
|  adi.local/api/platform/* --> platform:8015
|  adi.local/ --> web:8013
+------------------+
      |
      | Forward request (preserve path)
      | If Upgrade: websocket header present,
      | proxy as WebSocket connection
      v
+------------------+
|    Service       |
+------------------+
```

### 17.4 Shutdown Sequence

```
1. Receive SIGTERM/SIGINT
2. Stop accepting new connections
3. Drain existing connections (30s timeout)
4. Send SIGTERM to all services (reverse dependency order)
5. Wait for graceful shutdown (10s per service)
6. Send SIGKILL to remaining processes
7. Unload plugins
8. Exit
```

---

## 18. Examples

### 18.1 Minimal Configuration

```yaml
version: "1"

services:
  web:
    runner:
      type: script
      script:
        run: npm run dev
        working_dir: apps/web
    rollout:
      type: recreate
      recreate:
        ports:
          http: 3000
    proxy:
      path: /
```

### 18.2 Full ADI Development Stack

```yaml
version: "1"

defaults:
  hive.parse.ports:
    prefix: "adi-"            # Prefix for ports-manager keys
  hive.runner.docker:
    socket: /var/run/docker.sock
  hive.log.file:
    dir: .hive/logs
    rotate: true

proxy:
  bind:
    - "0.0.0.0:80"

environment:
  static:
    RUST_LOG: info
    LOGGING_URL: http://localhost:${ports.logging}

services:
  # =============================================================================
  # Databases (using docker plugin)
  # =============================================================================
  postgres:
    runner:
      type: docker
      docker:
        image: postgres:15
        ports:
          - "{{runtime.port.db}}:5432"
        volumes:
          - postgres-data:/var/lib/postgresql/data
        environment:
          POSTGRES_USER: adi
          POSTGRES_PASSWORD: adi
    rollout:
      type: recreate
      recreate:
        ports:
          db: ${ports.postgres}
    healthcheck:
      type: cmd
      cmd:
        command: pg_isready -U adi
        interval: 5s
    restart: unless-stopped

  timescaledb:
    runner:
      type: docker
      docker:
        image: timescale/timescaledb:latest-pg15
        ports:
          - "{{runtime.port.db}}:5432"
        volumes:
          - timescaledb-data:/var/lib/postgresql/data
        environment:
          POSTGRES_USER: adi
          POSTGRES_PASSWORD: adi
    rollout:
      type: recreate
      recreate:
        ports:
          db: ${ports.timescaledb}
    healthcheck:
      type: cmd
      cmd:
        command: pg_isready -U adi
        interval: 5s
    restart: unless-stopped

  # =============================================================================
  # Infrastructure (using docker plugin)
  # =============================================================================
  coturn:
    runner:
      type: docker
      docker:
        image: coturn/coturn:latest
        ports:
          - "{{runtime.port.turn}}:3478/udp"
          - "{{runtime.port.turn}}:3478/tcp"
        environment:
          TURN_USERNAME: adi
          TURN_PASSWORD: adi
    rollout:
      type: recreate
      recreate:
        ports:
          turn: ${ports.coturn}
    restart: unless-stopped

  # =============================================================================
  # Core Services (using built-in script runner)
  # =============================================================================
  signaling:
    runner:
      type: script
      script:
        run: cargo run
        working_dir: crates/tarminal-signaling-server
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.signaling}
    proxy:
      host: adi.local
      path: /api/signaling
    restart: on-failure

  logging:
    runner:
      type: script
      script:
        run: cargo run --bin adi-logging-service
        working_dir: crates/adi-logging-service
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.logging}
    proxy:
      host: adi.local
      path: /api/logging
    depends_on:
      - timescaledb
    environment:
      static:
        DATABASE_URL: postgres://adi:adi@localhost:${ports.timescaledb}/adi_logging
    restart: on-failure

  auth:
    runner:
      type: script
      script:
        run: cargo run -p adi-auth-http
        working_dir: crates/adi-auth
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.auth}
    proxy:
      host: adi.local
      path: /api/auth
    depends_on:
      - postgres
    environment:
      static:
        DATABASE_URL: postgres://adi:adi@localhost:${ports.postgres}/adi_auth
    healthcheck:
      type: http
      http:
        path: /health
        interval: 10s
    restart: on-failure

  platform:
    runner:
      type: script
      script:
        run: cargo run --bin adi-platform-api
        working_dir: crates/adi-platform-api
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.platform}
    proxy:
      host: adi.local
      path: /api/platform
    depends_on:
      - postgres
      - auth
    environment:
      static:
        DATABASE_URL: postgres://adi:adi@localhost:${ports.postgres}/adi_platform
        CORS_ORIGIN: http://adi.local
    healthcheck:
      type: http
      http:
        path: /health
        interval: 10s
    restart: on-failure

  # =============================================================================
  # Analytics
  # =============================================================================
  analytics-ingestion:
    runner:
      type: script
      script:
        run: cargo run
        working_dir: crates/adi-analytics-ingestion
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.analytics-ingestion}
    proxy:
      host: adi.local
      path: /api/analytics-ingestion
    depends_on:
      - timescaledb
    environment:
      static:
        DATABASE_URL: postgres://adi:adi@localhost:${ports.timescaledb}/adi_analytics
    restart: on-failure

  analytics:
    runner:
      type: script
      script:
        run: cargo run
        working_dir: crates/adi-analytics-api
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.analytics}
    proxy:
      host: adi.local
      path: /api/analytics
    depends_on:
      - timescaledb
    environment:
      static:
        DATABASE_URL: postgres://adi:adi@localhost:${ports.timescaledb}/adi_analytics
    healthcheck:
      type: http
      http:
        path: /health
        interval: 10s
    restart: on-failure

  # =============================================================================
  # API Services
  # =============================================================================
  llm-proxy:
    runner:
      type: script
      script:
        run: cargo run --bin adi-api-proxy
        working_dir: crates/adi-api-proxy/http
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.llm-proxy}
    proxy:
      host: adi.local
      path: /api/llm-proxy
    depends_on:
      - postgres
      - analytics-ingestion
    environment:
      static:
        DATABASE_URL: postgres://adi:adi@localhost:${ports.postgres}/adi_llm_proxy
        ANALYTICS_URL: http://localhost:${ports.analytics-ingestion}
    restart: on-failure

  balance:
    runner:
      type: script
      script:
        run: cargo run --bin adi-balance-api
        working_dir: crates/adi-balance-api
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.balance}
    proxy:
      host: adi.local
      path: /api/balance
    depends_on:
      - postgres
    environment:
      static:
        DATABASE_URL: postgres://adi:adi@localhost:${ports.postgres}/adi_balance
    restart: on-failure

  credentials:
    runner:
      type: script
      script:
        run: cargo run --bin adi-credentials-api
        working_dir: crates/adi-credentials-api
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.credentials}
    proxy:
      host: adi.local
      path: /api/credentials
    depends_on:
      - postgres
    environment:
      static:
        DATABASE_URL: postgres://adi:adi@localhost:${ports.postgres}/adi_credentials
    restart: on-failure

  # =============================================================================
  # Web Frontends
  # =============================================================================
  web:
    runner:
      type: script
      script:
        run: npm run dev
        working_dir: apps/infra-service-web
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.web}
    proxy:
      host: adi.local
      path: /
    depends_on:
      - auth
      - platform
    environment:
      static:
        AUTH_API_URL: http://localhost:${ports.auth}
        NEXT_PUBLIC_PLATFORM_API_URL: http://localhost:${ports.platform}
    restart: on-failure

  flowmap:
    runner:
      type: script
      script:
        run: cargo run --release
        working_dir: apps/flowmap-api
    rollout:
      type: recreate
      recreate:
        ports:
          http: ${ports.flowmap}
    proxy:
      host: adi.local
      path: /api/flowmap
    build:
      command: cargo build --release
      when: missing
    healthcheck:
      type: http
      http:
        path: /health
        interval: 10s
    restart: on-failure
```

### 18.3 With External Plugins

```yaml
version: "1"

# External plugins are auto-installed when first referenced
defaults:
  hive.runner.podman:
    socket: /run/podman/podman.sock
  hive.env.vault:
    address: https://vault.example.com

environment:
  vault:                   # Auto-installs hive.env.vault
    path: secret/data/adi
    token: ${env.VAULT_TOKEN}

services:
  postgres:
    runner:
      type: podman         # Auto-installs hive.runner.podman
      podman:
        image: postgres:15
        ports:
          - "{{runtime.port.db}}:5432"
    rollout:
      type: recreate
      recreate:
        ports:
          db: 5432
    healthcheck:
      type: postgres       # Auto-installs hive.health.postgres
      postgres:
        port: "{{runtime.port.db}}"
        user: adi
        database: adi_auth
```

---

## Appendix A: Plugin Interface

Plugins MUST implement a specific trait depending on their type. See `crates/hive/core/src/plugins/` for trait definitions.

### Runner Plugin Trait

```rust
pub trait RunnerPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn start(&self, config: &ServiceConfig) -> Result<RunningService>;
    fn stop(&self, service: &RunningService) -> Result<()>;
    fn logs(&self, service: &RunningService) -> Result<LogStream>;
}
```

### Environment Plugin Trait

```rust
pub trait EnvPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn load(&self, config: &EnvConfig) -> Result<HashMap<String, String>>;
}
```

### Health Check Plugin Trait

```rust
pub trait HealthPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self, config: &HealthConfig, port: u16) -> Result<HealthStatus>;
}
```

---

## Appendix B: Comparison with Alternatives

| Feature | docker-compose | process-compose | hive.yaml |
|---------|---------------|-----------------|-----------|
| Shell commands | - | + | + (built-in) |
| Docker containers | + | - | + (plugin) |
| docker-compose delegation | - | - | + (plugin) |
| HTTP reverse proxy | - | - | + |
| WebSocket proxy | - | - | + |
| Health checks | + | + | + |
| Dependencies | + | + | + |
| Restart policies | + | + | + |
| Build step | + | - | + |
| Plugin architecture | - | - | + |
| Environment plugins | - | - | + |
| ports-manager integration | - | - | + |

---

## Revision History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0-draft | 2026-01-25 | Initial draft with plugin architecture |
