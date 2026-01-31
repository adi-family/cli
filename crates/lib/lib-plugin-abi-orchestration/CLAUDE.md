plugin-abi, orchestration, shared-library, hive, docker, health-checks

## Overview
- Shared ABI definitions for orchestration plugins
- Domain-specific traits for runners, health, proxy, observability, rollout
- Used by Hive and future orchestrators
- Async-first with tokio runtime

## Plugin Categories
- **Runner**: Execute services (Docker, script, compose, podman)
- **Env**: Load environment variables (dotenv, vault, 1password, AWS secrets)
- **Health**: Check service readiness (HTTP, TCP, gRPC, databases)
- **Proxy**: HTTP middleware (CORS, rate limit, auth, IP filter)
- **Obs**: Observability (stdout, file, Loki, Prometheus)
- **Rollout**: Deployment strategies (recreate, blue-green)

## Key Files
- `src/runner.rs` - RunnerPlugin trait for service execution
- `src/health.rs` - HealthPlugin trait for readiness checks
- `src/proxy.rs` - ProxyPlugin trait for HTTP middleware
- `src/obs.rs` - ObsPlugin trait for logging/metrics
- `src/env.rs` - EnvPlugin trait for environment variables
- `src/rollout.rs` - RolloutPlugin trait for deployment strategies
- `src/hooks.rs` - Lifecycle hook definitions and execution
- `src/loader.rs` - Plugin discovery and loading utilities

## Plugin Naming Convention
- Pattern: `<orchestrator>.<category>.<name>`
- Examples: `hive.runner.docker`, `hive.obs.stdout`, `hive.health.http`

## Integration
- Orchestrators depend on this crate and implement plugin loaders
- Plugins implement traits from this crate
- Plugin metadata includes category, version, description

## Design Principles
- Stable ABI (breaking changes require major version bump)
- Async-first (all trait methods use async/await)
- Config flexibility (serde_json::Value for plugin-specific settings)
- Error handling via anyhow::Result
