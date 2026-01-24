webrtc-test-peer, e2e-testing, mock-cocoon, webrtc, rust

## WebRTC Test Peer

A minimal Rust WebRTC peer that acts as a mock "cocoon" for E2E testing of the web-app.

### Purpose
- Provides a real WebRTC endpoint for E2E tests
- Responds to PTY, Silk, and FileSystem messages with mock data
- Configurable for testing failure scenarios (ICE drops, latency, timeouts)

### Building
```bash
# From workspace root
cargo build --release -p webrtc-test-peer

# Binary location
./target/release/webrtc-test-peer
```

### CLI Usage
```bash
webrtc-test-peer [OPTIONS]

Options:
  -s, --signaling-url <URL>     Signaling server URL [default: ws://localhost:8080/ws]
      --secret <SECRET>         Secret for registration (auto-generated if not provided)
      --setup-token <TOKEN>     Setup token for authenticated registration
      --name <NAME>             Display name [default: test-cocoon]
      --bypass-auth             Bypass authentication for local testing
      --mock-fs-root <PATH>     Directory to serve as mock filesystem
      --drop-ice-rate <RATE>    ICE drop rate 0.0-1.0 for testing [default: 0.0]
      --drop-data-rate <RATE>   Data channel drop rate [default: 0.0]
      --latency-ms <MS>         Artificial response latency [default: 0]
      --no-answer               Don't send WebRTC answer (timeout testing)
      --one-shot                Exit after first WebRTC session
      --timeout-secs <SECS>     Connection timeout [default: 0 = no timeout]
      --log-level <LEVEL>       Log level [default: info]
```

### Registration Flow

1. Connects to signaling server via WebSocket
2. Sends `Register` message with secret
3. Server derives device_id from secret via HMAC-SHA256
4. Server responds with `Registered { device_id }`
5. Test peer is now ready to accept WebRTC connections

**Important:** The secret must be cryptographically strong:
- Minimum 32 characters
- Not only numbers or lowercase letters
- No weak patterns (password, secret, test, 12345, etc.)
- At least 10 unique characters

### Message Handlers

Located in `src/handlers/`:

**PTY Handler (`pty.rs`):**
| Request | Response |
|---------|----------|
| `attach_pty { command, cols, rows }` | `pty_created { session_id }` |
| `pty_input { session_id, data }` | `pty_output { session_id, data }` (echo) |
| `pty_resize { session_id, cols, rows }` | `pty_resized { session_id }` |
| `detach_pty { session_id }` | `pty_detached { session_id }` |

**Silk Handler (`silk.rs`):**
| Request | Response |
|---------|----------|
| `create_session { cwd }` | `session_created { session_id, cwd, shell }` |
| `execute { session_id, command, command_id }` | `command_started`, `output`, `command_completed { exit_code }` |
| `destroy_session { session_id }` | `session_destroyed { session_id }` |

**FileSystem Handler (`filesystem.rs`):**
| Request | Response |
|---------|----------|
| `fs_list_dir { path }` | `fs_dir_listing { entries }` |
| `fs_read_file { path }` | `fs_file_content { content, encoding }` |
| `fs_stat { path }` | `fs_file_stat { stat }` |
| `fs_walk { path, max_depth }` | `fs_walk_result { entries }` |

### Testing Failure Scenarios

**ICE Candidate Dropping:**
```bash
# Drop 50% of ICE candidates
webrtc-test-peer --drop-ice-rate 0.5
```

**Data Channel Message Dropping:**
```bash
# Drop 30% of data channel messages
webrtc-test-peer --drop-data-rate 0.3
```

**Artificial Latency:**
```bash
# Add 500ms latency to all responses
webrtc-test-peer --latency-ms 500
```

**Connection Timeout Testing:**
```bash
# Don't send WebRTC answer - client will timeout
webrtc-test-peer --no-answer
```

### Mock Filesystem

Provide a directory to serve as the mock filesystem root:
```bash
webrtc-test-peer --mock-fs-root /path/to/mock/data
```

Without `--mock-fs-root`, returns mock data:
- `/home/testuser/` - mock home directory
- `/home/testuser/file1.txt` - "Hello World"
- `/home/testuser/projects/` - mock projects directory

### Integration with E2E Tests

The test peer is automatically started by `apps/web-app/src/tests/e2e/global-setup.ts`:

1. Global setup starts signaling server
2. Starts test peer with fixed secret
3. Captures device_id from registration output
4. Exports `TEST_DEVICE_ID` and `TEST_PEER_SECRET` env vars
5. Tests use these to claim cocoon and connect

### Crate Structure
```
src/
  main.rs           # CLI entry point
  lib.rs            # TestPeer, TestPeerBuilder exports
  config.rs         # CLI args parsing, scenario config
  signaling.rs      # WebSocket signaling connection
  webrtc.rs         # RTCPeerConnection management
  handlers/
    mod.rs          # Handler trait
    pty.rs          # PTY message handler
    silk.rs         # Silk shell handler
    filesystem.rs   # FileSystem handler
```

### Adding New Handlers

1. Create handler in `src/handlers/new_handler.rs`
2. Implement the handler trait
3. Register in `src/handlers/mod.rs`
4. Add data channel name to WebRTC setup in `src/webrtc.rs`

```rust
// src/handlers/new_handler.rs
pub struct NewHandler;

impl NewHandler {
    pub fn handle(&self, message: &str) -> Option<String> {
        let msg: serde_json::Value = serde_json::from_str(message).ok()?;
        match msg.get("type")?.as_str()? {
            "my_request" => {
                Some(json!({
                    "type": "my_response",
                    "data": "..."
                }).to_string())
            }
            _ => None
        }
    }
}
```

### Debugging

```bash
# Run with debug logging
RUST_LOG=debug webrtc-test-peer --signaling-url ws://localhost:18080/ws

# Run with trace logging (very verbose)
RUST_LOG=trace webrtc-test-peer --signaling-url ws://localhost:18080/ws
```

### Common Issues

**"Registration rejected - weak secret"**
- Secret contains weak patterns like "test", "12345", "password"
- Use a cryptographically random secret

**"Connection reset without closing handshake"**
- Signaling server crashed or port conflict
- Check if another process is using the port

**WebRTC connection doesn't establish:**
- Check ICE candidates are being exchanged
- Check STUN server is reachable
- Try with `--log-level debug`
