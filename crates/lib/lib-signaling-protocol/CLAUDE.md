signaling-protocol, websocket, device-pairing, webrtc, cocoon-orchestration, json-serialization

## Overview
- Shared WebSocket message protocol for ADI signaling infrastructure
- Used by: hive (cocoon orchestration), cocoon (worker), signaling-server (relay), platform-api (integration)
- JSON-based for cross-platform compatibility (Rust, JavaScript/TypeScript, Swift)
- Supports: device pairing, cocoon spawning, WebRTC signaling, certificate management, browser debugging

## Key Message Categories
- **Device Registration**: Register, Registered, RegisterWithSetupToken, Deregister
- **Cocoon Lifecycle**: SpawnCocoon, TerminateCocoon, ListHives, RegisterHive
- **WebRTC Signaling**: WebRtcOffer, WebRtcAnswer, WebRtcIceCandidate, WebRtcSessionStarted
- **Certificate Management**: RequestCertificate, CertificateIssued, GetCertificateStatus
- **Browser Debugging**: BrowserDebugTabAvailable, BrowserDebugNetworkEvent, BrowserDebugConsoleEvent

## Architecture Decision
Extracted from `lib-tarminal-sync` to avoid coupling hive/cocoon to terminal CRDT synchronization.
- `lib-tarminal-sync` kept for: CRDT sync (VersionVector, SyncMessage, GridDelta)
- `lib-signaling-protocol` provides: WebSocket message definitions only

## Related Components
- `signaling-server`: WebSocket relay server implementing this protocol
- `hive`: Cocoon orchestration client
- `cocoon`: Worker device implementing protocol
- `adi-platform-api`: Platform integration layer
