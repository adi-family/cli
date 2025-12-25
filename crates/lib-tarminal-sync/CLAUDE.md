tarminal-sync-protocol, rust, crdt, version-vectors, json-serialization

## Overview
- Client-agnostic synchronization protocol for Tarminal terminal emulator
- Uses CRDT (Version Vectors) for conflict resolution
- JSON-based for cross-platform compatibility (Rust, Swift, TypeScript, etc.)
- Transport-agnostic (WebSocket, P2P, local network)

## Architecture
- **VersionVector**: Logical clocks for causality tracking
- **SyncMetadata**: Per-entity metadata with version vectors and timestamps
- **SyncMessage**: Protocol messages (hello, fullState, updates, deletes, acks)
- **GridDelta/GridSnapshot**: Terminal grid synchronization
- **TransportLayer**: Abstract interface for transport implementations

## Key Design Decisions
- **JSON serialization**: Works across Rust, Swift, JavaScript, Python, etc.
- **Version vectors over Lamport clocks**: Enables concurrent change detection
- **Last-writer-wins + tombstones**: Simple conflict resolution
- **Delta operations**: Efficient terminal state sync
- **Transport abstraction**: Any transport can be plugged in

## Usage
- Rust clients: Use directly via `lib-tarminal-sync`
- Swift clients: Bridge via JSON encoding/decoding
- JavaScript/TypeScript: Use `types.d.ts` definitions
- Python/Go/etc: Generate types from JSON schema

## Related Components
- `tarminal-signaling-server`: WebSocket relay for device pairing
- `apps/tarminal-native-macos/Packages/TarminalSync`: Swift implementation (can migrate to JSON bridge)
- Transport implementations in consuming applications
