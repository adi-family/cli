<!--
SPDX-License-Identifier: BSL-1.1

Business Source License 1.1

Licensor: ADI Team
Licensed Work: Hive YAML Specification

Change Date: Four years from the date of each release
Change License: Apache License 2.0

For more information, see https://mariadb.com/bsl11/
-->

# Hive YAML Specification

**Version:** 0.3.0-draft  
**Status:** Draft  
**Authors:** ADI Team  
**License:** BSL-1.1  
**Created:** 2026-01-25  
**Updated:** 2026-01-25

## TL;DR

```bash
# Start all services (in current source)
adi hive up

# Start specific services
adi hive up auth platform

# Stop all services
adi hive down

# View service status
adi hive status
adi hive status --all     # all sources

# View logs
adi hive logs auth
adi hive logs -f          # follow all

# Multi-source management
adi hive source list
adi hive source add ~/projects/myapp
adi hive start default:postgres   # FQN addressing

# Remote control (via signaling server)
adi hive connect <device-id>
```

**Minimal example** (`.adi/hive.yaml`):
```yaml
version: "1"

services:
  api:
    runner:
      type: script
      script:
        run: cargo run --bin api
    environment:
      1password:
        vault: Development
        item: api-secrets
    rollout:
      type: blue-green
      blue-green:
        ports:
          http:
            blue: 8080
            green: 8081
    proxy:
      host: localhost
      path: /api
    healthcheck:
      type: http
      http:
        port: "{{runtime.port.http}}"
        path: /health
        interval: 5s
        timeout: 3s
        retries: 3
```

---

## Abstract

This document specifies the configuration format for Hive, a plugin-based universal process orchestrator. Hive runs as a **single daemon per machine**, managing heterogeneous services from **multiple configuration sources** (YAML files or SQLite databases) with an integrated HTTP/WebSocket reverse proxy.

Key features:
- **Plugin system**: Runners, environment providers, health checks, logging—all via plugins
- **Multi-source**: Manage services from multiple projects/directories simultaneously
- **Service exposure**: Share services between sources with `expose`/`uses` declarations
- **Remote control**: Manage Hive via signaling server from CLI or Web UI
- **Dual config backends**: YAML (read-only, version-controllable) or SQLite (read-write, dynamic)

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
10. [Dependencies](#10-dependencies)
11. [Routing](#11-routing)
12. [Build Configuration](#12-build-configuration)
13. [Restart Policies](#13-restart-policies)
14. [Rollout Plugins](#14-rollout-plugins)
15. [Variable Interpolation](#15-variable-interpolation)
16. [Architecture](#16-architecture)
17. [Examples](#17-examples)
18. [Service Exposure](#18-service-exposure)
19. [Multi-Source Architecture](#19-multi-source-architecture)
20. [SQLite Config Backend](#20-sqlite-config-backend)
21. [Remote Control](#21-remote-control)
22. [Observability](#22-observability)

---

## 1. Terminology

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

| Term | Definition |
|------|------------|
| **Hive** | The orchestrator daemon that manages services and routes traffic (one per machine) |
| **Hive Daemon** | The single background process managing all sources on a machine |
| **Service** | A managed unit of execution |
| **Source** | A configuration origin (directory with hive.yaml or SQLite file) |
| **Default Source** | The `~/.adi/hive/` source, always loaded |
| **FQN** | Fully Qualified Name for a service: `source:service` (e.g., `default:postgres`) |
| **Plugin** | A modular component providing specific functionality |
| **Runner** | Plugin that executes services (process, docker, etc.) |
| **Environment Provider** | Plugin that supplies environment variables |
| **Health Checker** | Plugin that probes service readiness |
| **Port Provider** | Plugin that allocates ports |
| **Log Handler** | Plugin that processes service output |
| **Route** | An HTTP path prefix mapped to a service |
| **Proxy** | The built-in HTTP/WebSocket reverse proxy |
| **Expose** | Making a service available to other sources with shared variables |
| **Uses** | Declaring dependency on an exposed service from another source |

---

## 2. File Format

### 2.1 File Location

For YAML sources, the configuration file MUST be located at `.adi/hive.yaml` relative to the project root.

For SQLite sources, the database file is `hive.db` in the source directory or a standalone `.db` file.

The default source is always `~/.adi/hive/` (can contain either `hive.yaml` or `hive.db`).

See [Section 20](#20-multi-source-architecture) for multi-source configuration.

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
| `rollout` | `hive.rollout.*` | Control restart/deploy strategy | `recreate` |
| `obs` | `hive.obs.*` | Observability (logs, metrics, traces) | (none - all external) |

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
  
  hive.health.http:
    timeout: 5s
  
  hive.obs.stdout:
    format: pretty
    level: info
  
  hive.obs.file:
    dir: .hive/logs
    rotate: true
    max_size: 10MB
```

Services inherit these defaults. Service-level config overrides defaults:

```yaml
defaults:
  hive.obs.file:
    dir: .hive/logs
    rotate: true

# Observability is global, not per-service
# Individual plugins inherit from defaults
observability:
  plugins:
    - file            # Uses hive.obs.file defaults
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
type: docker  →  auto-installs: hive.runner.docker
type: vault  →  auto-installs: hive.env.vault
type: blue-green  →  auto-installs: hive.rollout.blue-green
type: loki  →  auto-installs: hive.obs.loki (observability)
```

**Example - Auto-install on first run:**
```yaml
# First time running this config:
# - hive.runner.docker will be auto-installed
# - hive.obs.loki will be auto-installed (observability plugin)

observability:
  plugins:
    - loki              # Auto-installs hive.obs.loki

services:
  postgres:
    runner:
      type: docker          # Auto-installs hive.runner.docker
      docker:
        image: postgres:15
```

**Disable auto-install:**

To prevent auto-installation (e.g., in CI/CD), set environment variable:
```bash
HIVE_AUTO_INSTALL=false hive up
```

When disabled, Hive will fail with an error listing missing plugins.

---

## 5. Proxy Configuration

The `proxy` section configures the built-in HTTP/WebSocket reverse proxy.

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
    build: { ... }            # OPTIONAL
    restart: <string>         # OPTIONAL
    expose: { ... }           # OPTIONAL - cross-source sharing
    uses: [ ... ]             # OPTIONAL - cross-source dependencies
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
| `build` | object | OPTIONAL | - | Build configuration |
| `restart` | string | OPTIONAL | `never` | Restart policy |
| `expose` | object | OPTIONAL | - | Make service available to other sources. See [Section 18](#18-service-exposure) |
| `uses` | array | OPTIONAL | `[]` | Declare dependencies on exposed services. See [Section 18](#18-service-exposure) |

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

## 10. Dependencies

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

## 11. Routing

The `proxy` section configures HTTP reverse proxy routing using explicit `host` and `path` fields.

### 11.1 Route Format

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

### 11.2 Route Matching

1. Routes with `host` are matched first (by `Host` header)
2. Within same host, longest `path` prefix wins
3. Routes without `host` match any `Host` header
4. If multiple services have identical host+path, Hive MUST fail with an error

### 11.3 WebSocket Support

Hive MUST transparently support WebSocket upgrades for all routes. When a request includes the `Upgrade: websocket` header:
- Hive MUST forward the upgrade request to the backend service
- Hive MUST NOT buffer the connection
- The backend service decides whether to accept the WebSocket upgrade

No explicit configuration is required.

---

## 12. Build Configuration

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

### 12.1 Build Triggers (`when`)

| Value | Description |
|-------|-------------|
| `missing` | Build only if output doesn't exist |
| `always` | Build every time before starting |
| `never` | Never build (assume pre-built) |

---

## 13. Restart Policies

The `restart` field controls service restart behavior.

| Value | Description |
|-------|-------------|
| `never` | Never restart. Manual control only |
| `on-failure` | Restart only on non-zero exit code |
| `always` | Always restart, including after crash |
| `unless-stopped` | Like `always`, but respects manual stop |

**Default:** `never`

### 13.1 Restart Behavior

When a service crashes and restart policy applies:
1. Hive MUST wait 1 second before first restart
2. Hive MUST use exponential backoff (1s, 2s, 4s, 8s, max 60s)
3. Hive MUST reset backoff after 60 seconds of healthy running

---

## 14. Rollout Plugins

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

### 14.1 Port Configuration

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

### 14.2 Recreate (built-in)

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

### 14.3 External Rollout Plugins

Additional rollout strategies are available as external plugins:

| Plugin | Install | Description |
|--------|---------|-------------|
| `blue-green` | `adi plugin install hive.rollout.blue-green` | Zero-downtime with traffic switching |
| `canary` | `adi plugin install hive.rollout.canary` | Gradual traffic shifting |
| `rolling` | `adi plugin install hive.rollout.rolling` | Rolling update for multiple instances |

### 14.4 Blue-Green (external plugin)

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

### 14.5 Blue-Green Sequence Diagram

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

### 14.6 Blue-Green Failure Scenarios

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

## 15. Variable Interpolation

Hive supports two types of variable interpolation:
1. **Parse-time plugins** (`${plugin.key}`) - resolved when YAML is parsed, via plugins
2. **Runtime templates** (`{{runtime...}}`) - resolved at service start from service config

### 15.1 Parse-Time Plugins (`${...}`)

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

### 15.2 Runtime Templates (`{{runtime...}}`)

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

### 15.3 Where Runtime Templates Can Be Used

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

### 15.4 Escaping

To include literal `${` or `{{`, escape with double:
```yaml
environment:
  static:
    SHELL_VAR: "$${NOT_INTERPOLATED}"      # Results in: ${NOT_INTERPOLATED}
    TEMPLATE: "{{{runtime.port.http}}}"    # Results in: {{runtime.port.http}}
```

### 15.5 Resolution Order

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

### 15.6 Parse-Time Plugin Interface

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

## 16. Architecture

### 16.1 Component Diagram

```
                              Hive Daemon (one per machine)
┌─────────────────────────────────────────────────────────────────────────┐
│  Sources                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                   │
│  │ ~/.adi/hive/ │  │ ~/p/project-a│  │ ~/p/project-b│                   │
│  │  (default)   │  │   (yaml)     │  │  (sqlite)    │                   │
│  └──────────────┘  └──────────────┘  └──────────────┘                   │
│         │                 │                 │                            │
│         └─────────────────┼─────────────────┘                            │
│                           ▼                                              │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                      Plugin Manager                                │  │
│  │  ┌─────────────────────┐  ┌─────────────────────────────┐         │  │
│  │  │     Built-in        │  │      External (auto-install) │         │  │
│  │  ├─────────────────────┤  ├─────────────────────────────┤         │  │
│  │  │ parse: env, service │  │ parse: ports, vault,        │         │  │
│  │  │ runner: script      │  │        1password, aws-ssm   │         │  │
│  │  │ env: static         │  │ runner: docker, compose,    │         │  │
│  │  │ health: http,tcp,cmd│  │         podman, kubernetes  │         │  │
│  │  │ log: file, stdout   │  │ env: dotenv, vault,         │         │  │
│  │  │ rollout: recreate   │  │      1password, aws-secrets │         │  │
│  │  │                     │  │ health: grpc, postgres,     │         │  │
│  │  │                     │  │         redis, mysql        │         │  │
│  │  │                     │  │ log: loki, cloudwatch       │         │  │
│  │  │                     │  │ rollout: blue-green,        │         │  │
│  │  │                     │  │          canary, rolling    │         │  │
│  │  └─────────────────────┘  └─────────────────────────────┘         │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                           │                                              │
│         ┌─────────────────┼─────────────────┐                           │
│         ▼                 ▼                 ▼                            │
│   ┌──────────┐      ┌──────────┐      ┌──────────┐                      │
│   │ default: │      │project-a:│      │project-b:│                      │
│   │ postgres │      │  auth    │      │   api    │                      │
│   │ redis    │      │ platform │      │  worker  │                      │
│   └──────────┘      └──────────┘      └──────────┘                      │
│         │                 │                 │                            │
│         └─────────────────┼─────────────────┘                            │
│                           ▼                                              │
│                 ┌──────────────────┐                                    │
│                 │ Unified HTTP/WS  │                                    │
│                 │      Proxy       │                                    │
│                 └──────────────────┘                                    │
│                           │                                              │
├───────────────────────────┼──────────────────────────────────────────────┤
│                           ▼                                              │
│                 ┌──────────────────┐     ┌──────────────────┐           │
│                 │     Clients      │     │ Signaling Server │           │
│                 │  (HTTP/WebSocket)│     │ (Remote Control) │           │
│                 └──────────────────┘     └──────────────────┘           │
└─────────────────────────────────────────────────────────────────────────┘
```

### 16.2 Startup Sequence

```
1. Start daemon (if not running)
   - Create socket at ~/.adi/hive/hive.sock
   - Write PID to ~/.adi/hive/hive.pid
2. Load all registered sources
   - Always load default source (~/.adi/hive/)
   - Load each registered source (YAML or SQLite)
3. For each source:
   a. Parse configuration
   b. Load and initialize plugins
   c. Validate service configurations
   d. Check for port/route conflicts with other sources
4. Collect all expose declarations
   - Validate expose names are globally unique
5. Build combined dependency graph (including cross-source uses)
6. Start services in topological order:
   For each service:
   a. Resolve uses dependencies (wait for exposed services)
   b. Inject exposed variables
   c. Load environment via env plugins
   d. Run build command (if configured)
   e. Start runner plugin
   f. Wait for health check plugin (if configured)
   g. Register route in unified proxy
7. Start unified proxy server
8. Connect to signaling server (if configured)
9. Enter supervision loop:
   - Monitor service health
   - Restart crashed services (per restart policy)
   - Handle SIGTERM/SIGINT gracefully
   - Process remote control commands
```

### 16.3 Request Flow

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

### 16.4 Shutdown Sequence

```
1. Receive SIGTERM/SIGINT (or remote shutdown command)
2. Disconnect from signaling server
3. Stop accepting new proxy connections
4. Drain existing connections (30s timeout)
5. Send SIGTERM to all services (reverse dependency order, respecting cross-source uses)
6. Wait for graceful shutdown (10s per service)
7. Send SIGKILL to remaining processes
8. Unload plugins
9. Remove socket and PID file
10. Exit
```

---

## 17. Examples

### 17.1 Minimal Configuration

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

### 17.2 Full ADI Development Stack

```yaml
version: "1"

defaults:
  hive.parse.ports:
    prefix: "adi-"            # Prefix for ports-manager keys
  hive.runner.docker:
    socket: /var/run/docker.sock
  hive.obs.stdout:
    format: pretty
    level: info
  hive.obs.file:
    dir: .hive/logs
    per_service: true
    rotate: true
    max_size: 100MB

observability:
  resource_interval: 10s
  plugins:
    - stdout
    - file
    - prometheus

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

### 17.3 With External Plugins

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

## 18. Service Exposure

Service exposure enables cross-source dependencies. By default, services are **private** to their source. To share a service with other sources, explicitly **expose** it.

### 18.1 Expose Configuration

The `expose` field makes a service available to other sources:

```yaml
services:
  postgres:
    runner:
      type: docker
      docker:
        image: postgres:15
    rollout:
      type: recreate
      recreate:
        ports:
          db: 5432
    
    expose:
      name: shared-postgres              # REQUIRED - globally unique name
      secret: ${env.POSTGRES_SECRET}     # OPTIONAL - require secret to use
      vars:                              # REQUIRED - variables to share
        DATABASE_URL: postgres://adi:adi@localhost:{{runtime.port.db}}/
        PG_HOST: localhost
        PG_PORT: "{{runtime.port.db}}"
        PG_USER: adi
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | REQUIRED | Globally unique name across all sources |
| `secret` | string | OPTIONAL | Secret required to use this service |
| `vars` | object | REQUIRED | Variables to share with consumers |

**Notes:**
- All ports from `rollout.ports` are automatically available to consumers
- `vars` values support `{{runtime.port.<name>}}` interpolation
- If `secret` is specified, consumers MUST provide matching secret

### 18.2 Uses Configuration

The `uses` field declares dependencies on exposed services from other sources:

```yaml
services:
  auth:
    runner:
      type: script
      script:
        run: cargo run -p auth
    
    uses:
      - name: shared-postgres            # REQUIRED - exposed service name
        secret: ${env.POSTGRES_SECRET}   # REQUIRED if expose has secret
        as: pg                           # OPTIONAL - local alias
        vars:                            # OPTIONAL - remap variable names
          DATABASE_URL: AUTH_DB_URL      # Inject as AUTH_DB_URL instead
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | REQUIRED | Name of exposed service |
| `secret` | string | CONDITIONAL | Required if exposed service has secret |
| `as` | string | OPTIONAL | Local alias for port references |
| `vars` | object | OPTIONAL | Remap exposed variable names |

### 18.3 Variable Injection

When a service with `uses` starts, exposed variables are injected into its environment:

```yaml
# Exposed by shared-postgres:
vars:
  DATABASE_URL: postgres://adi:adi@localhost:5432/
  PG_HOST: localhost
  PG_PORT: "5432"

# Consumer's uses:
uses:
  - name: shared-postgres
    vars:
      DATABASE_URL: AUTH_DB_URL    # Remap
      # PG_HOST and PG_PORT not remapped

# Consumer's final environment:
AUTH_DB_URL=postgres://adi:adi@localhost:5432/
PG_HOST=localhost
PG_PORT=5432
```

### 18.4 Port References

Access exposed service ports using `{{uses.<alias>.port.<name>}}`:

```yaml
uses:
  - name: shared-postgres
    as: pg

environment:
  static:
    CUSTOM_PORT: "{{uses.pg.port.db}}"   # Resolves to 5432
```

### 18.5 Startup Ordering

Services with `uses` wait for their dependencies:

1. Hive collects all `expose` declarations from all sources
2. Validates expose names are globally unique
3. When starting a service with `uses`:
   - Resolve each exposed service by name
   - Verify secret matches (bcrypt hashed)
   - Wait for exposed service to be **healthy**
   - Inject exposed variables
4. Start the consumer service

### 18.6 Security

- Secrets are stored as bcrypt hashes (not plaintext)
- No secret = service is publicly exposed to all sources
- Secret verification happens at service startup
- Failed secret verification prevents service from starting

### 18.7 Example: Shared Infrastructure

```yaml
# Source: ~/.adi/hive/ (default source)
version: "1"

services:
  postgres:
    runner:
      type: docker
      docker:
        image: postgres:15
        ports:
          - "{{runtime.port.db}}:5432"
    rollout:
      type: recreate
      recreate:
        ports:
          db: 5432
    expose:
      name: shared-postgres
      secret: ${env.INFRA_SECRET}
      vars:
        DATABASE_URL: postgres://adi:adi@localhost:{{runtime.port.db}}/
        PG_HOST: localhost
        PG_PORT: "{{runtime.port.db}}"

  redis:
    runner:
      type: docker
      docker:
        image: redis:7
        ports:
          - "{{runtime.port.cache}}:6379"
    rollout:
      type: recreate
      recreate:
        ports:
          cache: 6379
    expose:
      name: shared-redis
      secret: ${env.INFRA_SECRET}
      vars:
        REDIS_URL: redis://localhost:{{runtime.port.cache}}
```

```yaml
# Source: ~/projects/adi/ (project source)
version: "1"

services:
  auth:
    runner:
      type: script
      script:
        run: cargo run -p adi-auth-http
    uses:
      - name: shared-postgres
        secret: ${env.INFRA_SECRET}
        vars:
          DATABASE_URL: AUTH_DATABASE_URL
      - name: shared-redis
        secret: ${env.INFRA_SECRET}
    # auth gets: AUTH_DATABASE_URL, REDIS_URL

  platform:
    runner:
      type: script
      script:
        run: cargo run --bin adi-platform-api
    uses:
      - name: shared-postgres
        secret: ${env.INFRA_SECRET}
        vars:
          DATABASE_URL: PLATFORM_DATABASE_URL
    depends_on:
      - auth
```

---

## 19. Multi-Source Architecture

Hive supports managing services from **multiple config sources** simultaneously. Each source is independent but shares a unified proxy and can expose services to other sources.

### 19.1 Daemon Model

One Hive daemon runs per machine:

```
+---------------------------------------------------------------------+
|                           Machine                                    |
|                                                                      |
|  +----------------------------------------------------------------+ |
|  |                    Hive Daemon (ONE)                            | |
|  |                                                                  | |
|  |  Sources:                                                        | |
|  |  +------------+ +------------+ +------------+                   | |
|  |  | ~/.adi/hive| | ~/p/adi    | | ~/p/foo    |                   | |
|  |  | (default)  | | (yaml)     | | (sqlite)   |                   | |
|  |  +------------+ +------------+ +------------+                   | |
|  |         |              |              |                          | |
|  |         v              v              v                          | |
|  |  +-----------------------------------------------------+        | |
|  |  |              Unified Proxy Server                    |        | |
|  |  |         (routes from ALL sources merged)             |        | |
|  |  +-----------------------------------------------------+        | |
|  +----------------------------------------------------------------+ |
+---------------------------------------------------------------------+
```

### 19.2 Default Source

`~/.adi/hive/` is the **default source**, always loaded:

- Can contain `hive.yaml` OR `hive.db` (SQLite)
- Ideal for shared infrastructure (databases, caches, proxies)
- Services here are available to all other sources via `expose`

### 19.3 Source Types

| Type | Detection | Mutability |
|------|-----------|------------|
| YAML | Project directory with `.adi/hive.yaml` | Read-only |
| SQLite | Standalone `.db` file OR directory with `hive.db` | Read-write |

**Detection logic:**
1. If path is a `.db` file → SQLite source
2. If path is a directory containing `hive.db` → SQLite source  
3. If path is a directory containing `.adi/hive.yaml` → YAML source
4. Special case: `~/.adi/hive/` checks for `hive.yaml` or `hive.db` directly (no `.adi/` subdirectory)

### 19.4 Source Management

```bash
# List sources
adi hive source list
# NAME        TYPE    PATH                    SERVICES  STATUS
# default     sqlite  ~/.adi/hive/            3         running
# adi         yaml    ~/projects/adi/         12        running
# myproject   sqlite  ~/projects/foo/hive.db  2         stopped

# Add source
adi hive source add ~/projects/adi
adi hive source add ~/infra.db --name infra

# Remove source (stops services first)
adi hive source remove myproject

# Reload source config
adi hive source reload adi

# Enable/disable without removing
adi hive source disable myproject
adi hive source enable myproject
```

### 19.5 Service Addressing

Services are addressed by **Fully Qualified Name (FQN)**: `source:service`

```bash
# Start specific service
adi hive start default:postgres
adi hive start adi:auth

# View logs
adi hive logs adi:platform

# In project directory (context-aware)
cd ~/projects/adi
adi hive start auth              # Implicit: adi:auth
adi hive logs platform           # Implicit: adi:platform
```

### 19.6 Conflict Detection

Hive prevents conflicts at source add time:

**Port conflicts:**
```
Error: Port 8080 already used by default:nginx
Cannot add source 'myproject' with service 'api' using port 8080
```

**Route conflicts:**
```
Error: Route /api/auth conflicts with adi:auth
Cannot add source 'other' with service 'auth' using path /api/auth
```

**Expose name conflicts:**
```
Error: Expose name 'shared-postgres' already used by default:postgres
Cannot add source 'backup' with service 'pg' exposing 'shared-postgres'
```

### 19.7 Daemon Lifecycle

```bash
# Daemon auto-starts on first use
adi hive up                      # Starts daemon if needed

# Explicit daemon control
adi hive daemon status           # Check if running
adi hive daemon stop             # Stop daemon and all services
adi hive daemon restart          # Restart daemon
```

**Daemon files:**
- Socket: `~/.adi/hive/hive.sock`
- PID: `~/.adi/hive/hive.pid`
- Logs: `~/.adi/hive/logs/`

---

## 20. SQLite Config Backend

SQLite provides a **read-write** alternative to YAML for configuration. Both backends have full feature parity.

### 20.1 When to Use SQLite

| Use Case | YAML | SQLite |
|----------|------|--------|
| Version control | Best | Possible |
| Remote editing | Read-only | Best |
| Dynamic config | - | Best |
| Human editing | Best | Via CLI |
| Programmatic access | Parse needed | Best |

### 20.2 SQLite Schema

```sql
-- Meta
CREATE TABLE hive_meta (
    key TEXT PRIMARY KEY,
    value TEXT
);

-- Global configuration
CREATE TABLE global_defaults (
    plugin_id TEXT PRIMARY KEY,
    config JSON NOT NULL
);

CREATE TABLE global_proxy (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    bind JSON NOT NULL
);

CREATE TABLE global_environment (
    provider TEXT PRIMARY KEY,
    config JSON NOT NULL,
    priority INTEGER DEFAULT 0
);

-- Services
CREATE TABLE services (
    name TEXT PRIMARY KEY,
    enabled BOOLEAN DEFAULT true,
    restart_policy TEXT DEFAULT 'never',
    working_dir TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE service_runners (
    service_name TEXT PRIMARY KEY REFERENCES services(name) ON DELETE CASCADE,
    runner_type TEXT NOT NULL,
    config JSON NOT NULL
);

CREATE TABLE service_rollouts (
    service_name TEXT PRIMARY KEY REFERENCES services(name) ON DELETE CASCADE,
    rollout_type TEXT NOT NULL,
    config JSON NOT NULL
);

CREATE TABLE service_proxies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
    host TEXT,
    path TEXT NOT NULL,
    port_ref TEXT DEFAULT '{{runtime.port.http}}',
    strip_prefix BOOLEAN DEFAULT false,
    timeout_ms INTEGER DEFAULT 60000,
    extra JSON
);

CREATE TABLE service_healthchecks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
    check_type TEXT NOT NULL,
    config JSON NOT NULL
);

CREATE TABLE service_environment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    config JSON NOT NULL,
    priority INTEGER DEFAULT 0
);

CREATE TABLE service_dependencies (
    service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
    depends_on TEXT NOT NULL REFERENCES services(name),
    PRIMARY KEY (service_name, depends_on)
);

CREATE TABLE service_builds (
    service_name TEXT PRIMARY KEY REFERENCES services(name) ON DELETE CASCADE,
    command TEXT NOT NULL,
    working_dir TEXT,
    build_when TEXT DEFAULT 'missing'
);

CREATE TABLE service_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
    log_type TEXT NOT NULL,
    config JSON NOT NULL
);

-- Service exposure
CREATE TABLE service_expose (
    service_name TEXT PRIMARY KEY REFERENCES services(name) ON DELETE CASCADE,
    expose_name TEXT UNIQUE NOT NULL,
    secret_hash TEXT,
    vars JSON NOT NULL
);

CREATE TABLE service_uses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
    exposed_name TEXT NOT NULL,
    secret_encrypted TEXT,
    local_alias TEXT,
    var_remaps JSON,
    UNIQUE(service_name, exposed_name)
);

-- Runtime state (not config)
CREATE TABLE runtime_state (
    service_name TEXT PRIMARY KEY,
    state TEXT NOT NULL,
    pid INTEGER,
    container_id TEXT,
    started_at TIMESTAMP,
    stopped_at TIMESTAMP,
    restart_count INTEGER DEFAULT 0,
    last_exit_code INTEGER,
    last_error TEXT
);
```

### 20.3 CLI Config Commands

```bash
# View config as YAML
adi hive config show
adi hive config show --source infra

# Service management (SQLite only)
adi hive config service add              # Interactive
adi hive config service edit auth        # Edit service
adi hive config service remove auth      # Remove service

# Environment management
adi hive config env set FOO=bar          # Global env
adi hive config env unset FOO
adi hive config env list

# Export to YAML (for backup/version control)
adi hive config export > hive-backup.yaml
```

---

## 21. Remote Control

Hive registers with the signaling server as a **special cocoon type**, enabling remote control via CLI or Web UI.

### 21.1 Hive Registration

```rust
SignalingMessage::Register {
    secret: String,
    device_id: Option<String>,
    version: String,
    device_type: DeviceType::Hive {
        name: String,              // Machine/hive name
        sources: Vec<SourceInfo>,  // Config sources
    },
}
```

Hive uses the same claiming flow as cocoons:
1. Hive registers with signaling server
2. User claims hive via JWT
3. User can send control commands

### 21.2 Hive Control Protocol

```rust
// Tunneled via SignalingMessage::HiveControl { device_id, request_id, payload }

enum HiveRequest {
    // Daemon
    GetStatus,
    Shutdown { graceful: bool },
    
    // Sources
    ListSources,
    AddSource { path: String, name: Option<String> },
    RemoveSource { source_id: String },
    ReloadSource { source_id: String },
    
    // Services
    ListServices { source: Option<String> },
    GetService { fqn: String },
    StartService { fqn: String },
    StopService { fqn: String },
    RestartService { fqn: String },
    StartSource { source_id: String },
    StopSource { source_id: String },
    
    // Logs
    GetLogs { fqn: String, lines: u32, since: Option<DateTime> },
    StreamLogs { fqn: String },
    
    // Config (SQLite sources only)
    GetSourceConfig { source_id: String },
    CreateService { source_id: String, config: ServiceConfig },
    UpdateService { fqn: String, patch: ServicePatch },
    DeleteService { fqn: String },
    
    // Exposed services
    ListExposed,
    GetExposedUsers { name: String },
}

enum HiveResponse {
    Status(DaemonStatus),
    Sources(Vec<SourceInfo>),
    Services(Vec<ServiceInfo>),
    Service(ServiceInfo),
    Config(SourceConfig),
    Logs(Vec<LogLine>),
    StreamStarted { stream_id: Uuid },
    Exposed(Vec<ExposedInfo>),
    Ok,
    Error { code: ErrorCode, message: String },
}
```

### 21.3 Remote CLI

```bash
# Connect to remote hive
adi hive connect <device-id>
adi hive connect --list          # List available hives

# Once connected, all commands work remotely
adi hive status
adi hive source list
adi hive start adi:auth
adi hive logs adi:platform
adi hive config show             # Remote config
```

### 21.4 Log Streaming

```rust
// Request streaming
HiveRequest::StreamLogs { fqn: "adi:auth".to_string() }

// Response
HiveResponse::StreamStarted { stream_id: Uuid }

// Server pushes logs
SignalingMessage::HiveLogStream {
    stream_id: Uuid,
    lines: Vec<LogLine>,
}

// Client stops stream
HiveRequest::StopLogStream { stream_id: Uuid }
```

---

## 22. Observability

Hive provides a comprehensive observability system through a plugin-based architecture. The daemon collects all observability data internally and streams it to subscribed plugins via a Unix socket.

### 22.1 Architecture

```
+---------------------------------------------------------------------+
|                         Hive Daemon                                  |
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |              Internal Event Collector                          |  |
|  |  - Service logs (stdout/stderr)                               |  |
|  |  - Process metrics (CPU, memory, FDs, network)                |  |
|  |  - Health check results                                        |  |
|  |  - Proxy request traces                                        |  |
|  |  - Service lifecycle (start, stop, restart, crash)            |  |
|  |  - Resource utilization                                        |  |
|  +---------------------------------------------------------------+  |
|                            |                                         |
|                            v                                         |
|  +---------------------------------------------------------------+  |
|  |              Event Stream Socket                               |  |
|  |              ~/.adi/hive/observability.sock                    |  |
|  |                                                                |  |
|  |    Plugin 1 <----+                                             |  |
|  |    Plugin 2 <----+---- MessagePack events                      |  |
|  |    Plugin 3 <----+                                             |  |
|  +---------------------------------------------------------------+  |
+---------------------------------------------------------------------+
         |                    |                    |
         v                    v                    v
+---------------+    +---------------+    +---------------+
| hive.obs.     |    | hive.obs.     |    | hive.obs.     |
| prometheus    |    | loki          |    | otel          |
| (metrics)     |    | (logs)        |    | (traces)      |
+---------------+    +---------------+    +---------------+
```

### 22.2 Event Types

Hive collects and streams these event types:

```rust
enum ObservabilityEvent {
    // Service logs (stdout/stderr)
    Log {
        timestamp: DateTime<Utc>,
        service_fqn: String,        // "source:service"
        level: LogLevel,            // trace, debug, info, notice, warn, error, fatal
        message: String,
        fields: HashMap<String, Value>,
        stream: LogStream,          // stdout | stderr
    },
    
    // Numeric metrics
    Metric {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        name: String,               // e.g., "cpu_percent", "memory_rss_bytes"
        value: MetricValue,         // gauge, counter, histogram
        labels: HashMap<String, String>,
    },
    
    // Distributed trace spans
    Span {
        trace_id: Uuid,
        span_id: Uuid,
        parent_span_id: Option<Uuid>,
        service_fqn: String,
        operation: String,          // e.g., "proxy_request", "healthcheck"
        start: DateTime<Utc>,
        duration_us: u64,
        status: SpanStatus,         // ok, error
        attributes: HashMap<String, Value>,
    },
    
    // Health check results
    HealthCheck {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        check_type: String,         // http, tcp, cmd, grpc
        status: HealthStatus,       // healthy, unhealthy, unknown
        latency_ms: u32,
        error: Option<String>,
    },
    
    // Service lifecycle events
    ServiceEvent {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        event: ServiceEventType,    // starting, started, stopping, stopped,
                                    // crashed, restarting, health_changed
        details: HashMap<String, Value>,
    },
    
    // Proxy request traces
    ProxyRequest {
        timestamp: DateTime<Utc>,
        trace_id: Uuid,
        span_id: Uuid,
        service_fqn: String,
        method: String,
        path: String,
        status_code: u16,
        duration_us: u64,
        request_bytes: u64,
        response_bytes: u64,
        client_ip: Option<String>,
        user_agent: Option<String>,
        is_websocket: bool,
    },
    
    // Resource utilization
    ResourceMetrics {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        pid: u32,
        cpu_percent: f32,
        memory_rss_bytes: u64,
        memory_vms_bytes: u64,
        open_fds: u32,
        threads: u32,
        network_rx_bytes: u64,
        network_tx_bytes: u64,
    },
    
    // Custom service events
    Custom {
        timestamp: DateTime<Utc>,
        service_fqn: String,
        event_type: String,
        data: Value,
    },
}
```

### 22.3 Metric Types

```rust
enum MetricValue {
    Gauge(f64),                     // Current value (e.g., CPU %)
    Counter(u64),                   // Monotonic counter (e.g., requests)
    Histogram {                     // Distribution (e.g., latency)
        count: u64,
        sum: f64,
        buckets: Vec<(f64, u64)>,   // (le, count)
    },
}

enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Notice = 3,
    Warn = 4,
    Error = 5,
    Fatal = 6,
}
```

### 22.4 Event Stream Protocol

#### Socket Location

```
~/.adi/hive/observability.sock
```

#### Frame Format

Length-prefixed MessagePack:

```
[4-byte length (big-endian)][MessagePack payload]
```

#### Subscription Handshake

On connect, client sends subscription:

```rust
struct Subscribe {
    event_types: Vec<String>,       // ["log", "metric", "span", ...] (empty = all)
    services: Vec<String>,          // ["default:postgres", "adi:*"] (empty = all)
    min_log_level: Option<LogLevel>,
}
```

Server responds:

```rust
struct SubscribeAck {
    stream_id: Uuid,
    hive_version: String,
}
```

Then server streams events matching the subscription.

#### Backpressure

- Socket has configurable send buffer limit
- If client is slow, oldest events are dropped
- `EventDropped` notification sent when dropping occurs

### 22.5 Configuration

#### Top-Level Configuration

```yaml
version: "1"

observability:
  # Resource metrics collection interval
  resource_interval: 5s
  
  # Socket buffer size (events)
  buffer_size: 10000
  
  # Enable/disable specific collectors
  collectors:
    logs: true
    metrics: true
    traces: true
    health: true
    resources: true
    proxy: true
  
  # Active observability plugins
  plugins:
    - stdout                    # hive.obs.stdout
    - file                      # hive.obs.file
    - prometheus                # hive.obs.prometheus

defaults:
  hive.obs.stdout:
    format: pretty              # pretty | json | compact
    colors: auto                # auto | always | never
    timestamp: true
    level: info
  
  hive.obs.file:
    dir: .hive/logs
    per_service: true           # Separate file per service
    rotate: true
    max_size: 100MB
    max_files: 10
  
  hive.obs.prometheus:
    bind: "0.0.0.0:9090"
    path: /metrics
  
  hive.obs.loki:
    url: http://loki:3100/loki/api/v1/push
    batch_size: 100
    flush_interval: 5s
    labels:
      env: production
  
  hive.obs.otel:
    endpoint: http://otel-collector:4317
    protocol: grpc              # grpc | http
    headers:
      authorization: Bearer ${env.OTEL_TOKEN}
  
  hive.obs.adi:
    signaling_url: ${env.SIGNALING_URL}
    logging_service: true       # Forward to adi-logging-service
    analytics_service: true     # Forward to adi-analytics
```

#### SQLite Schema

```sql
CREATE TABLE observability_config (
    key TEXT PRIMARY KEY,
    value JSON NOT NULL
);
-- Keys: 'resource_interval', 'buffer_size', 'collectors'

CREATE TABLE observability_plugins (
    plugin_id TEXT PRIMARY KEY,
    enabled BOOLEAN DEFAULT true,
    config JSON NOT NULL
);
```

### 22.6 Plugin Interface

```rust
pub trait ObservabilityPlugin: Send + Sync {
    /// Plugin identifier (e.g., "prometheus", "loki", "otel")
    fn name(&self) -> &str;
    
    /// Start the plugin (called once on hive startup)
    fn start(&mut self, config: &PluginConfig) -> Result<()>;
    
    /// Stop the plugin (called on hive shutdown)
    fn stop(&mut self) -> Result<()>;
    
    /// Get subscription filter (what events this plugin wants)
    fn subscription(&self) -> Subscribe;
    
    /// Handle incoming event
    fn handle_event(&mut self, event: &ObservabilityEvent) -> Result<()>;
}
```

### 22.7 Available Plugins

| Plugin ID | Install | Description |
|-----------|---------|-------------|
| `stdout` | `adi plugin install hive.obs.stdout` | Formatted console output |
| `file` | `adi plugin install hive.obs.file` | File logging with rotation |
| `prometheus` | `adi plugin install hive.obs.prometheus` | Prometheus metrics endpoint |
| `loki` | `adi plugin install hive.obs.loki` | Grafana Loki log shipping |
| `otel` | `adi plugin install hive.obs.otel` | OpenTelemetry export (OTLP) |
| `jaeger` | `adi plugin install hive.obs.jaeger` | Jaeger trace export |
| `adi` | `adi plugin install hive.obs.adi` | ADI services integration |
| `alertmanager` | `adi plugin install hive.obs.alertmanager` | Prometheus Alertmanager |
| `datadog` | `adi plugin install hive.obs.datadog` | Datadog APM integration |
| `cloudwatch` | `adi plugin install hive.obs.cloudwatch` | AWS CloudWatch |

### 22.8 CLI Commands

```bash
# View real-time logs
adi hive logs                     # All services
adi hive logs auth                # Specific service
adi hive logs -f                  # Follow mode
adi hive logs --level warn        # Filter by level
adi hive logs --since 5m          # Last 5 minutes

# View metrics
adi hive metrics                  # Current metrics summary
adi hive metrics auth             # Service metrics
adi hive metrics --watch          # Live update

# View health
adi hive health                   # All services health
adi hive health --watch           # Live health status

# View resources (htop-like)
adi hive top                      # Interactive resource view
adi hive resources                # Detailed resource usage

# Plugin management
adi hive obs plugins              # List observability plugins
adi hive obs enable prometheus    # Enable plugin
adi hive obs disable loki         # Disable plugin
adi hive obs config prometheus    # View/edit plugin config
```

### 22.9 Remote Observability

The `hive.obs.adi` plugin enables remote observability access via the signaling server:

```rust
// Events relayed through signaling (batched)
SignalingMessage::HiveObservability {
    hive_id: String,
    stream_id: Uuid,
    events: Vec<ObservabilityEvent>,
}

// Remote subscription request
HiveRequest::SubscribeObservability {
    event_types: Vec<String>,
    services: Vec<String>,
    min_log_level: Option<LogLevel>,
}

// Response
HiveResponse::ObservabilitySubscribed {
    stream_id: Uuid,
}
```

This enables:
- Remote log viewing via `adi hive logs` (when connected to remote hive)
- Web UI log viewer and metrics dashboard
- Centralized logging to `adi-logging-service`
- Analytics events to `adi-analytics`

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
| Multi-source management | - | - | + |
| Cross-source dependencies | - | - | + (expose/uses) |
| SQLite config backend | - | - | + |
| Remote control | - | - | + (signaling) |
| Daemon mode | - | - | + |
| Observability plugins | logging only | - | + (full stack) |
| Prometheus metrics | - | - | + (plugin) |
| OpenTelemetry export | - | - | + (plugin) |
| Distributed tracing | - | - | + |
| Resource monitoring | - | - | + |

---

## Revision History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0-draft | 2026-01-25 | Initial draft with plugin architecture |
| 0.2.0-draft | 2026-01-25 | Added service exposure (expose/uses), multi-source architecture, SQLite backend, remote control |
| 0.3.0-draft | 2026-01-25 | Replaced log plugins with comprehensive observability system (Section 22) |
