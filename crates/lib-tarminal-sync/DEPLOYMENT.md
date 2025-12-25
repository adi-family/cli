# Deployment Guide

deployment, architecture, signaling-server

## Deployment Architecture

Tarminal sync uses a centralized signaling server architecture:

### **Signaling Server Mode**

```
Device A (Home)  ←──→  Signaling Server  ←──→  Device B (Office)
   macOS                  (Cloud)                  Linux
```

**Deploy:**
```bash
# 1. Deploy signaling server to cloud (Fly.io, Railway, etc.)
cd crates/tarminal-signaling-server
docker build -t tarminal-signaling .
docker run -p 8080:8080 tarminal-signaling

# 2. Or deploy directly
cargo run --release
# Server runs on http://0.0.0.0:8080

# 3. Configure clients with server URL
# Swift: Update WebSyncService.swift
# Rust: Connect to ws://your-server.com/ws
# Web: Set WebSocket URL in JavaScript
```

**How it works:**
1. Server runs 24/7 on cloud infrastructure
2. Devices connect via WebSocket: `ws://server:8080/ws`
3. Device A creates pairing code → Server generates "ABC123"
4. Device B enters "ABC123" → Server pairs devices
5. All sync data flows through server (devices don't connect directly)

**Server responsibilities:**
- Device registration
- Pairing code generation (6 chars, 5-minute expiry)
- Message relay between paired devices
- Connection state tracking

**Features:**
- ✅ Works across NATs and firewalls
- ✅ No port forwarding needed
- ✅ Simple pairing with 6-digit codes
- ✅ Sync across the Internet

---

## Quick Start: Test Locally

### Terminal 1: Start Signaling Server
```bash
cd crates/tarminal-signaling-server
cargo run --release
# Server running on http://0.0.0.0:8080
```

### Terminal 2: Device 1 (Rust client)
```bash
cd crates/lib-tarminal-sync
cargo run --example basic_sync
```

### Browser: Device 2 (Web client)
```bash
# Open examples/web_client.html in browser
open crates/lib-tarminal-sync/examples/web_client.html

# 1. Click "Connect to ws://localhost:8080/ws"
# 2. Click "Create Pairing Code" → Get code "ABC123"
# 3. Enter code on Device 1
# 4. Devices are now paired and syncing!
```

---

## Production Deployment

### Option 1: Fly.io (Recommended)
```bash
cd crates/tarminal-signaling-server

# Install flyctl
curl -L https://fly.io/install.sh | sh

# Deploy
fly launch
fly deploy

# Get URL: wss://tarminal-signaling.fly.dev/ws
```

### Option 2: Railway
```bash
# 1. Create account at railway.app
# 2. Connect GitHub repo
# 3. Deploy from crates/tarminal-signaling-server
# 4. Railway auto-detects Dockerfile
```

### Option 3: Docker Compose (Self-hosted)
```yaml
# docker-compose.yml
version: '3.8'
services:
  signaling:
    build: ./crates/tarminal-signaling-server
    ports:
      - "8080:8080"
    environment:
      - PORT=8080
    restart: unless-stopped
```

---

## Client Configuration

### Rust Client
```rust
use tokio_tungstenite::connect_async;

let server_url = "ws://your-server.com:8080/ws";
let (ws, _) = connect_async(server_url).await?;

// Register device
let msg = SignalingMessage::Register {
    device_id: device_id.to_string(),
};
ws.send(Message::Text(serde_json::to_string(&msg)?)).await?;
```

### Swift Client (macOS/iOS)
```swift
// Update WebSyncService.swift
let serverURL = URL(string: "wss://your-server.com/ws")!
let manager = WebSyncManager(serverURL: serverURL, ...)
manager.startSync()
```

### Web Client (JavaScript)
```javascript
const ws = new WebSocket('wss://your-server.com/ws');
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'register',
    device_id: deviceId
  }));
};
```

---

## Cost

- **Fly.io**: ~$5/month (shared-cpu-1x, 256MB RAM)
- **Railway**: ~$5/month (starter plan)
- **Self-hosted**: Free (if you have a VPS)

---

## Monitoring

The signaling server logs all events:
```rust
// Check logs
docker logs -f <container-id>

// Example output:
[INFO] Device registered: abc-123
[INFO] Pairing code created: X7K9M2
[INFO] Devices paired: abc-123 <-> def-456
[INFO] Device disconnected: abc-123
```

Add your own metrics/monitoring:
- Prometheus endpoint
- Sentry error tracking
- CloudWatch/Datadog metrics

---

## Security Considerations

1. **Use WSS (WebSocket Secure)** in production
2. **Pairing codes expire** after 5 minutes
3. **Device pairing is bidirectional** (can't intercept)
4. **Consider E2E encryption** for sync data payload
5. **Rate limit** pairing attempts

Future improvements:
- Add authentication (JWT, OAuth)
- End-to-end encryption
- Device verification
- Audit logs
