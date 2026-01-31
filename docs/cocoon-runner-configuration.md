# Cocoon Runner Configuration

## Overview

Hive now supports **multiple runner types** for cocoons, not just Docker. Each cocoon kind specifies its own runner configuration.

## Configuration Format

### Environment Variable: COCOON_KINDS

The `COCOON_KINDS` environment variable supports three formats:

#### 1. Backward Compatible (Docker only)

```bash
# Format: "id:image"
export COCOON_KINDS="ubuntu:git.the-ihor.com/adi/cocoon:ubuntu,python:git.the-ihor.com/adi/cocoon:python"
```

This creates Docker cocoon kinds automatically.

#### 2. New Format with Runner Type

```bash
# Format: "id:runner_type:config"
export COCOON_KINDS="native:script:cocoon-worker,ubuntu:docker:git.the-ihor.com/adi/cocoon:ubuntu"
```

For simple configs, the third part can be a string (image for docker, command for script).

#### 3. Full JSON Format

```bash
# Format: "id:runner_type:json_config"
export COCOON_KINDS='native:script:{"command":"cocoon-worker","args":["--kind","{kind}"]}'
```

For complex configs, use JSON.

#### 4. Pure JSON Array

```bash
export COCOON_KINDS='[
  {
    "id": "native",
    "runner_type": "script",
    "runner_config": {
      "command": "/usr/local/bin/cocoon-worker",
      "args": ["--kind", "native"],
      "working_dir": "/tmp/cocoons/{service_name}"
    }
  },
  {
    "id": "ubuntu",
    "runner_type": "docker",
    "runner_config": {
      "image": "git.the-ihor.com/adi/cocoon:ubuntu"
    }
  }
]'
```

## Runner Types

### Docker Runner

**Type**: `docker`

**Config Fields**:
```json
{
  "image": "registry/cocoon:tag",
  "container_name": "auto-generated",  // Auto-set by Hive
  "environment": {},                    // Auto-merged by Hive
  "volumes": ["data:/data"],            // Optional
  "ports": ["8080:8080"]                // Optional
}
```

**Example**:
```bash
export COCOON_KINDS="ubuntu:docker:{\"image\":\"git.the-ihor.com/adi/cocoon:ubuntu\"}"
```

### Script Runner

**Type**: `script`

**Config Fields**:
```json
{
  "command": "cocoon-worker",           // Executable path
  "args": ["--kind", "{kind}"],         // Command arguments
  "working_dir": "/tmp/cocoons/{service_name}"  // Working directory
}
```

**Placeholders** (interpolated at spawn time):
- `{kind}` - Cocoon kind ID
- `{service_name}` - Generated service name
- `{request_id}` - Spawn request ID

**Environment** (auto-injected):
- `SIGNALING_SERVER_URL` - WebSocket URL
- `COCOON_SETUP_TOKEN` - Setup token for initial connection
- `COCOON_SECRET` - Secret for authentication
- `COCOON_KIND` - Kind ID

**Example**:
```bash
export COCOON_KINDS='native:script:{"command":"cocoon-worker","args":["--kind","{kind}"],"working_dir":"/tmp/cocoons/{service_name}"}'
```

### Podman Runner

**Type**: `podman`

**Config**: Same as Docker

**Example**:
```bash
export COCOON_KINDS="ubuntu:podman:{\"image\":\"git.the-ihor.com/adi/cocoon:ubuntu\"}"
```

## Complete Examples

### Mixed Docker and Native

```bash
export COCOON_KINDS="
  native:script:cocoon-worker,
  ubuntu:docker:git.the-ihor.com/adi/cocoon:ubuntu,
  python:docker:git.the-ihor.com/adi/cocoon:python,
  gpu:docker:git.the-ihor.com/adi/cocoon:gpu
"
```

### Custom Script with Args

```bash
export COCOON_KINDS='native:script:{"command":"/opt/cocoon/worker","args":["--kind","{kind}","--verbose"],"working_dir":"/var/cocoons/{service_name}"}'
```

### Multiple Native Variants

```bash
export COCOON_KINDS='
  native-small:script:{"command":"cocoon-worker","args":["--memory","512m"]},
  native-large:script:{"command":"cocoon-worker","args":["--memory","4g"]},
  docker-ubuntu:docker:git.the-ihor.com/adi/cocoon:ubuntu
'
```

## CocoonKind Structure

```rust
pub struct CocoonKind {
    /// Unique identifier (e.g., "native", "ubuntu", "gpu")
    pub id: String,

    /// Runner type: "docker", "script", "podman"
    pub runner_type: String,

    /// Runner configuration (JSON object)
    pub runner_config: serde_json::Value,

    /// DEPRECATED: Docker image (for backward compatibility)
    pub image: String,
}
```

## Default Cocoon Kinds

If `COCOON_KINDS` is not set, Hive provides these defaults:

| Kind | Runner | Config |
|------|--------|--------|
| alpine | docker | git.the-ihor.com/adi/cocoon:alpine |
| debian | docker | git.the-ihor.com/adi/cocoon:debian |
| ubuntu | docker | git.the-ihor.com/adi/cocoon:ubuntu |
| linux | docker | git.the-ihor.com/adi/cocoon:ubuntu (alias) |
| python | docker | git.the-ihor.com/adi/cocoon:python |
| node | docker | git.the-ihor.com/adi/cocoon:node |
| full | docker | git.the-ihor.com/adi/cocoon:full |
| gpu | docker | git.the-ihor.com/adi/cocoon:gpu |
| cuda | docker | git.the-ihor.com/adi/cocoon:gpu (alias) |

## Migration Guide

### From Old Format

**Before**:
```bash
export COCOON_KINDS="ubuntu:git.the-ihor.com/adi/cocoon:ubuntu"
```

**After** (same result):
```bash
export COCOON_KINDS="ubuntu:docker:git.the-ihor.com/adi/cocoon:ubuntu"
```

The old format still works! It automatically creates Docker kinds.

### Adding Native Cocoons

**Before** (Docker only):
```bash
export COCOON_KINDS="ubuntu:git.the-ihor.com/adi/cocoon:ubuntu"
```

**After** (Docker + Native):
```bash
export COCOON_KINDS="
  native:script:cocoon-worker,
  ubuntu:docker:git.the-ihor.com/adi/cocoon:ubuntu
"
```

## Spawn Request

When spawning a cocoon, specify the kind:

```json
{
  "type": "spawn_cocoon",
  "request_id": "req-123",
  "kind": "native",  // Use "native" kind (script runner)
  "setup_token": "token-abc",
  "name": "worker-1"
}
```

Hive will:
1. Look up the "native" kind
2. See it uses "script" runner
3. Create service with ScriptRunner
4. Spawn native process (not Docker!)
5. Process connects to signaling server

## Debugging

Check registered kinds:

```bash
# List available cocoon kinds
adi hive kinds

# Example output:
# native (script)
# ubuntu (docker)
# python (docker)
# gpu (docker)
```

Check spawned cocoons:

```bash
# List all services (including cocoons)
adi hive list

# Example output:
# cocoons:cocoon-abc123  running  PID 12345 (native)
# cocoons:cocoon-xyz789  running  Container abc123 (ubuntu)
```

## Benefits

### 1. Native Process Cocoons

No Docker overhead:
- Faster startup
- Lower memory usage
- Direct access to host resources

### 2. Flexibility

Mix and match:
- Native for CPU-intensive tasks
- Docker for isolation
- Podman for rootless containers

### 3. Unified Management

All cocoons (Docker and native) get:
- HTTP proxy routing
- Health checks
- Observability
- Auto-restart
- Service exposure

## Limitations

### Script Runner

- Runs on Hive host (not isolated)
- Process inherits Hive's permissions
- No automatic port mapping
- Manual cleanup required on crash

**Recommendation**: Use script runner only in trusted environments or with sandboxing (firejail, bubblewrap).

### Podman Runner

- Requires Podman installed on Hive host
- May need rootless configuration
- Socket permissions must be configured

## Security Considerations

### Native Cocoons

**Risk**: Native processes run with Hive's user permissions.

**Mitigations**:
1. Run Hive as unprivileged user
2. Use sandboxing (firejail, bubblewrap)
3. Set resource limits (ulimit, cgroups)
4. Validate cocoon binaries
5. Use separate user for cocoons

**Example with sandboxing**:
```bash
export COCOON_KINDS='native:script:{"command":"firejail","args":["--private","/usr/local/bin/cocoon-worker"]}'
```

### Docker Cocoons

**Risk**: Docker daemon access (socket).

**Mitigations**:
1. Use rootless Docker
2. Limit container capabilities
3. Use security profiles (AppArmor, SELinux)
4. Resource constraints in runner config

## License

BSL-1.0
