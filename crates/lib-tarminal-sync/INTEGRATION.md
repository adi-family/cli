# Integration Guide

integration-guide, cross-platform, rust, swift, typescript, javascript

## Overview
This guide shows how to integrate `lib-tarminal-sync` into different platforms.

## Rust Integration

Add to `Cargo.toml`:
```toml
[dependencies]
lib-tarminal-sync = { path = "../lib-tarminal-sync" }
# Or from workspace:
lib-tarminal-sync = { workspace = true }
```

Usage:
```rust
use lib_tarminal_sync::*;
use uuid::Uuid;

// Create syncable entity
let device = Uuid::new_v4();
let workspace = SyncableWorkspace {
    id: Uuid::new_v4(),
    name: "My Workspace".to_string(),
    sync_metadata: SyncMetadata::new(device),
    // ...
};

// Create message
let msg = SyncMessage::WorkspaceUpdate { workspace };

// Serialize to JSON
let json = serde_json::to_string(&msg)?;

// Send over any transport (WebSocket, etc.)
websocket.send(json).await?;
```

## Swift Integration

Convert between Swift and JSON:

```swift
import Foundation

// Define matching structs (or generate from JSON)
struct SyncableWorkspace: Codable {
    let id: UUID
    let name: String
    let syncMetadata: SyncMetadata
    // ...
}

// Encode to JSON
let workspace = SyncableWorkspace(/* ... */)
let encoder = JSONEncoder()
let data = try encoder.encode(workspace)

// Send over transport
webSocket.send(data)

// Receive and decode
let decoder = JSONDecoder()
let workspace = try decoder.decode(SyncableWorkspace.self, from: data)
```

### Migration from Native Swift Types

If you have existing Swift types in `TarminalSync`, you can:

1. **Option A - Dual approach**: Keep Swift types for local use, convert to JSON for sync
2. **Option B - Full migration**: Replace Swift types with JSON-based protocol

Example dual approach:
```swift
extension SyncableWorkspace {
    func toJSON() -> Data {
        // Convert to JSON-serializable format
        let encoder = JSONEncoder()
        return try! encoder.encode(self)
    }

    static func fromJSON(_ data: Data) -> SyncableWorkspace {
        let decoder = JSONDecoder()
        return try! decoder.decode(SyncableWorkspace.self, from: data)
    }
}
```

## TypeScript/JavaScript Integration

```typescript
import type { SyncMessage, SyncableWorkspace } from './types';
import { v4 as uuidv4 } from 'uuid';

// Create workspace
const workspace: SyncableWorkspace = {
  id: uuidv4(),
  name: 'My Workspace',
  sync_metadata: {
    created_at: new Date().toISOString(),
    modified_at: new Date().toISOString(),
    version: { clocks: { [deviceId]: 1 } },
    origin_device_id: deviceId,
    tombstone: false,
  },
  // ...
};

// Create message
const msg: SyncMessage = {
  type: 'workspace_update',
  workspace,
};

// Send over WebSocket
websocket.send(JSON.stringify(msg));

// Receive
websocket.onmessage = (event) => {
  const msg: SyncMessage = JSON.parse(event.data);
  handleMessage(msg);
};
```

## Python Integration

```python
import json
import uuid
from datetime import datetime
from typing import Dict

# Create workspace (dict matching JSON structure)
workspace = {
    "id": str(uuid.uuid4()),
    "name": "My Workspace",
    "sync_metadata": {
        "created_at": datetime.utcnow().isoformat() + "Z",
        "modified_at": datetime.utcnow().isoformat() + "Z",
        "version": {"clocks": {str(device_id): 1}},
        "origin_device_id": str(device_id),
        "tombstone": False
    }
}

# Create message
msg = {
    "type": "workspace_update",
    "workspace": workspace
}

# Send
websocket.send(json.dumps(msg))

# Receive
data = json.loads(websocket.recv())
```

## Transport Implementation

### WebSocket Transport

```rust
use lib_tarminal_sync::{TransportLayer, SignalingMessage};
use tokio_tungstenite::{connect_async, tungstenite::Message};

async fn connect_to_signaling_server() {
    let (ws, _) = connect_async("ws://localhost:8080/ws").await?;

    // Register device
    let msg = SignalingMessage::Register {
        device_id: device_id.to_string(),
    };
    let json = serde_json::to_string(&msg)?;
    ws.send(Message::Text(json)).await?;

    // Handle messages
    while let Some(msg) = ws.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let signaling_msg: SignalingMessage = serde_json::from_str(&text)?;
            handle_signaling_message(signaling_msg);
        }
    }
}
```

### Custom Transport

```rust
use lib_tarminal_sync::{TransportLayer, TransportDelegate, TransportResult};

struct MyCustomTransport {
    device_id: DeviceId,
    delegate: Option<Box<dyn TransportDelegate>>,
}

impl TransportLayer for MyCustomTransport {
    fn device_id(&self) -> DeviceId {
        self.device_id
    }

    fn send(&mut self, message: SyncMessage, to: DeviceId) -> TransportResult<()> {
        // Implement your custom sending logic
        let json = serde_json::to_string(&message)
            .map_err(|e| TransportError::EncodingFailed(e.to_string()))?;

        // Send via your custom transport (HTTP, gRPC, etc.)
        my_custom_send(&json, to)?;

        Ok(())
    }

    // ... implement other methods
}
```

## Conflict Resolution

The protocol uses CRDT-based merging:

```rust
// Check relationship between versions
if meta1.happens_before(&meta2) {
    // meta2 is newer, use it
} else if meta1.concurrent_with(&meta2) {
    // Concurrent updates, merge needed
    let merged = meta1.merged(&meta2);

    // Use timestamp or custom logic for content merge
    if meta1.modified_at > meta2.modified_at {
        // Use meta1's content
    } else {
        // Use meta2's content
    }
} else {
    // meta1 is newer, use it
}
```

## Testing Integration

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_platform_serialization() {
        let workspace = create_test_workspace();

        // Serialize
        let json = serde_json::to_string(&workspace).unwrap();

        // This JSON can be consumed by Swift, JavaScript, Python, etc.
        assert!(json.contains("\"type\":\"workspace_update\""));

        // Deserialize
        let decoded: SyncableWorkspace = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.name, workspace.name);
    }
}
```

## Migration Path

If you have an existing Swift-only implementation:

1. **Phase 1**: Add `lib-tarminal-sync` alongside existing code
2. **Phase 2**: Implement JSON bridge in Swift
3. **Phase 3**: Test with both implementations running in parallel
4. **Phase 4**: Gradually replace Swift-specific code with JSON protocol
5. **Phase 5**: Enable cross-platform clients (web, Rust, etc.)

## Performance Considerations

- **JSON overhead**: Minimal for sync messages (typically < 1KB)
- **Grid deltas**: Use compression for large terminal snapshots
- **Batching**: Batch multiple updates into single message when possible
- **Delta vs snapshot**: Use deltas for incremental updates, snapshots for full sync

## Security

- All messages are authenticated by the signaling server
- Device pairing uses temporary codes (5-minute expiry)
- Paired devices are stored server-side with bidirectional mapping
- Consider adding E2E encryption for sync data payload

## Examples

See `examples/` directory:
- `basic_sync.rs`: Rust example showing protocol usage
- `web_client.html`: Browser-based WebSocket client
- More examples coming for Swift, Python, etc.
