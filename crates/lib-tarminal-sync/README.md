# lib-tarminal-sync

sync-protocol, crdt, version-vectors, cross-platform

## Overview
- Client-agnostic synchronization protocol for Tarminal terminal emulator
- CRDT-based conflict resolution using Version Vectors
- JSON serialization for cross-platform compatibility (Rust, Swift, JavaScript/TypeScript, etc.)
- Transport-agnostic (WebSocket, peer-to-peer, local network, etc.)

## Features
- **Version Vectors**: Causal ordering and conflict detection
- **Sync Metadata**: Per-entity metadata for CRDT merging
- **Delta/Snapshot Sync**: Efficient terminal grid synchronization
- **Device Pairing**: Secure device discovery and connection
- **Transport Abstraction**: Implement any transport layer (WebSocket, P2P, etc.)

## Protocol Overview

### Core Concepts

#### 1. Version Vectors
Every entity has a version vector tracking causal history across all devices. This enables:
- Detecting concurrent modifications
- Determining happens-before relationships
- Merging conflicting changes

```rust
use lib_tarminal_sync::{VersionVector, DeviceId};
use uuid::Uuid;

let device = Uuid::new_v4();
let mut vv = VersionVector::new();
vv.increment(device);
```

#### 2. Sync Metadata
Every syncable entity carries metadata for conflict resolution:

```rust
use lib_tarminal_sync::SyncMetadata;

let device = Uuid::new_v4();
let mut meta = SyncMetadata::new(device);
meta.touch(device); // Update on modification
```

#### 3. Message Types

**Sync Messages** (peer-to-peer):
- `Hello`: Initial handshake
- `RequestFullSync`: Request complete state
- `FullState`: Complete application state
- `WorkspaceUpdate`, `SessionUpdate`, `CommandBlockUpdate`: Incremental updates
- `Delete`: Tombstone notification
- `Ack`, `Ping`, `Pong`: Protocol management

**Signaling Messages** (server-mediated):
- `Register`: Connect to signaling server
- `CreatePairingCode`, `UsePairingCode`: Device pairing
- `SyncData`: Relayed sync data
- `PeerConnected`, `PeerDisconnected`: Connection status

### Grid Synchronization

Terminal state uses delta operations for efficiency:

```rust
use lib_tarminal_sync::{GridDelta, GridOperation};

let delta = GridDelta {
    operations: vec![
        GridOperation::CursorMove { x: 10, y: 5 },
        GridOperation::SetCells {
            row: 5,
            start_col: 10,
            cells: vec![/* ... */],
        },
    ],
    base_version: 1,
    new_version: 2,
};
```

## Usage

### Rust

```rust
use lib_tarminal_sync::{
    SyncMessage, SyncableWorkspace, SyncMetadata,
    VersionVector, DeviceId,
};
use uuid::Uuid;

// Create a workspace
let device = Uuid::new_v4();
let workspace = SyncableWorkspace {
    id: Uuid::new_v4(),
    name: "My Workspace".to_string(),
    icon: None,
    session_ids: vec![],
    active_session_id: None,
    sync_metadata: SyncMetadata::new(device),
};

// Create sync message
let msg = SyncMessage::WorkspaceUpdate { workspace };

// Serialize to JSON
let json = serde_json::to_string(&msg).unwrap();

// Send over any transport (WebSocket, P2P, etc.)
```

### TypeScript/JavaScript

```typescript
import type {
  SyncMessage,
  SyncableWorkspace,
  SyncMetadata,
  VersionVector,
  VersionVectorUtils,
} from './types';
import { v4 as uuidv4 } from 'uuid';

// Create a workspace
const deviceId = uuidv4();
const workspace: SyncableWorkspace = {
  id: uuidv4(),
  name: 'My Workspace',
  icon: null,
  session_ids: [],
  active_session_id: null,
  sync_metadata: {
    created_at: new Date().toISOString(),
    modified_at: new Date().toISOString(),
    version: { clocks: { [deviceId]: 1 } },
    origin_device_id: deviceId,
    tombstone: false,
  },
};

// Create sync message
const msg: SyncMessage = {
  type: 'workspace_update',
  workspace,
};

// Send over WebSocket
websocket.send(JSON.stringify(msg));

// Receive and handle
websocket.onmessage = (event) => {
  const msg: SyncMessage = JSON.parse(event.data);

  switch (msg.type) {
    case 'workspace_update':
      // Merge workspace using CRDT logic
      mergeWorkspace(msg.workspace);
      break;
    case 'full_state':
      // Replace entire state
      replaceState(msg.state);
      break;
  }
};

// Version vector operations
let vv: VersionVector = { clocks: {} };
vv = VersionVectorUtils.increment(vv, deviceId);

const concurrent = VersionVectorUtils.concurrent(vv1, vv2);
const merged = VersionVectorUtils.merge(vv1, vv2);
```

### Swift

The native Swift implementation in `apps/tarminal-native-macos` can be migrated to use this protocol via JSON bridging:

```swift
// Encode to JSON
let workspace = SyncableWorkspace(/* ... */)
let encoder = JSONEncoder()
let data = try encoder.encode(workspace)

// Decode from JSON
let decoder = JSONDecoder()
let workspace = try decoder.decode(SyncableWorkspace.self, from: data)
```

## Transport Implementation

Implement the `TransportLayer` trait for custom transports:

```rust
use lib_tarminal_sync::{
    TransportLayer, TransportDelegate, TransportResult,
    SyncMessage, PeerInfo, DeviceId,
};

struct MyTransport {
    device_id: DeviceId,
    delegate: Option<Box<dyn TransportDelegate>>,
}

impl TransportLayer for MyTransport {
    fn device_id(&self) -> DeviceId {
        self.device_id
    }

    fn send(&mut self, message: SyncMessage, to: DeviceId) -> TransportResult<()> {
        // Implement sending logic
        Ok(())
    }

    // ... implement other methods
}
```

## Conflict Resolution

The protocol uses **Last-Writer-Wins** based on:
1. Version vector causality (happens-before)
2. Timestamp for concurrent updates
3. Tombstones for deletions

```rust
// When merging concurrent versions
let merged_metadata = meta1.merged(&meta2);
let merged_version = vv1.merged(&vv2);
```

## Architecture

```
┌─────────────────────────────────────────────┐
│         Application Layer                   │
│  (Workspaces, Sessions, CommandBlocks)      │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│      lib-tarminal-sync (Protocol)           │
│  - SyncMessage                              │
│  - VersionVector (CRDT)                     │
│  - SyncMetadata                             │
│  - GridDelta/GridSnapshot                   │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│      Transport Layer (Pluggable)            │
│  - WebSocket (signaling server)             │
│  - Custom implementations                   │
└─────────────────────────────────────────────┘
```

## Examples

See the `/examples` directory for:
- WebSocket client/server
- P2P sync example
- Grid delta compression
- Conflict resolution scenarios

## Related Projects

- **tarminal-signaling-server**: WebSocket relay for device pairing
- **TarminalSync** (Swift): Native iOS/macOS implementation
- **TarminalWebSync** (Swift): WebSocket transport for web sync

## License

BSL-1.0
