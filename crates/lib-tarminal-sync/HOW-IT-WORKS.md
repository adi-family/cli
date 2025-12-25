# How Tarminal Sync Works

how-it-works, architecture, flow, deployment

## 🎯 Simple Answer

**The signaling server runs on ONE cloud machine (like Fly.io), and ALL your devices connect to it.**

```
Your MacBook (home) ──┐
Your Linux VPS ────────┼──→ Signaling Server (Fly.io)
Your iPad ─────────────┘
```

The server **relays messages** between paired devices. It's like a WhatsApp server - it doesn't store your data, just passes messages between devices.

---

## 📡 Complete Flow (Step-by-Step)

### Step 1: Deploy Signaling Server (Once)

```bash
cd crates/tarminal-signaling-server
fly launch
fly deploy
# Server now running at: wss://your-app.fly.dev/ws
```

**This runs 24/7 on Fly.io** (costs ~$5/month)

### Step 2: Connect Device A (Your MacBook)

```
MacBook → Connects to wss://your-app.fly.dev/ws
Server  → "Connected! Register your device"
MacBook → {"type": "register", "device_id": "abc-123"}
Server  → "Registered ✅"
```

### Step 3: Create Pairing Code

```
MacBook → {"type": "create_pairing_code"}
Server  → {"type": "pairing_code", "code": "X7K9M2"}
MacBook → Shows "X7K9M2" on screen
```

### Step 4: Connect Device B (Your Linux Server)

```
Linux → Connects to wss://your-app.fly.dev/ws
Server → "Connected! Register your device"
Linux → {"type": "register", "device_id": "def-456"}
Server → "Registered ✅"
```

### Step 5: Pair Devices

```
Linux → User enters code "X7K9M2"
Linux → {"type": "use_pairing_code", "code": "X7K9M2"}
Server → Looks up code → "abc-123" created it
Server → Creates bidirectional pairing:
         abc-123 ↔ def-456
Server → {"type": "paired", "peer_id": "abc-123"} → Linux
Server → {"type": "paired", "peer_id": "def-456"} → MacBook
```

**Now devices are paired!** 🎉

### Step 6: Sync Data

```
MacBook → Creates workspace "My Project"
MacBook → {
  "type": "workspace_update",
  "workspace": {
    "id": "workspace-1",
    "name": "My Project",
    "sync_metadata": {...}
  }
}
        ↓
Server  → "This is from abc-123, who's paired with def-456"
Server  → Forwards message to Linux
        ↓
Linux   → Receives workspace update
Linux   → Merges using CRDT logic
Linux   → Now has workspace "My Project" ✅
```

**Data flow:**
```
Device A → Server → Device B
(JSON)     (relay)  (JSON)
```

---

## 🔄 Real-World Example

Let's say you're working on your MacBook at home, then switch to your Linux server:

**On MacBook (evening):**
```bash
$ cd ~/my-project
$ ls
README.md  src/  tests/

# Tarminal automatically syncs this session
# Including: current directory, command history, output
```

**On Linux Server (next morning):**
```bash
# Open Tarminal
# All workspaces from MacBook appear!
# Click on "my-project" workspace
# You're now in the same state:

$ pwd
/home/user/my-project

$ history
  1  cd ~/my-project
  2  ls

# Everything synced! 🎉
```

**What got synced:**
1. Workspace exists ("my-project")
2. Session exists (terminal session)
3. Current directory (`/home/user/my-project`)
4. Command history
5. Terminal output (stored as blocks)

---

## 📊 Data That Syncs

### Workspace Level
- Workspace name & icon
- List of sessions
- Active session

### Session Level
- Session title
- Current directory
- Command history
- Terminal type (block-based vs interactive)

### Command Block Level
- Command text
- Output text
- Exit code
- Timestamps

### Terminal Grid (for interactive sessions)
- Terminal state (cursor position, colors)
- Grid deltas (only changes, not full screen)
- Scrollback buffer

---

## 🔐 Security & Privacy

**Current:**
- ✅ Device pairing with expiring codes
- ✅ Bidirectional pairing (can't intercept)
- ✅ WSS (encrypted WebSocket)
- ⚠️ Data passes through server (visible to server)

**Future improvements:**
- 🔜 End-to-end encryption (server can't read data)
- 🔜 Device verification
- 🔜 Authentication (JWT/OAuth)

---

## 💰 Cost

- **Fly.io**: ~$5/month (shared-cpu-1x, 256MB RAM)
- **Railway**: ~$5/month (starter plan)
- **Self-hosted**: Free (if you have a VPS)

---

## 🚀 Quick Test (5 Minutes)

### Terminal 1: Start Server
```bash
cd crates/tarminal-signaling-server
cargo run --release
```

### Terminal 2: Device 1
```bash
cd crates/lib-tarminal-sync
cargo run --example simple_client
# Choose: 1 (Create pairing code)
# Note the code: X7K9M2
```

### Browser: Device 2
```bash
open crates/lib-tarminal-sync/examples/web_client.html
# 1. Click "Connect"
# 2. Enter code: X7K9M2
# 3. Devices paired! ✅
```

---

## 🤔 FAQ

**Q: Does the server store my data?**
A: No, it only relays messages. Data is stored on your devices.

**Q: What if the server goes down?**
A: Devices can't sync until it's back up. Data is stored locally on each device.

**Q: Can I self-host the server?**
A: Yes! It's just a Rust binary. Run it anywhere.

**Q: How much bandwidth does it use?**
A: Minimal. Only changed data syncs (deltas), not full state.

**Q: Can I sync 10+ devices?**
A: Currently designed for 2-device pairing. Multi-device sync coming soon.

**Q: What about conflicts?**
A: Uses CRDT (Version Vectors) for automatic merging. Last-Writer-Wins for content.

---

## 🎓 Learn More

- **Architecture**: See `README.md`
- **Deployment**: See `DEPLOYMENT.md`
- **Integration**: See `INTEGRATION.md`
- **Examples**: See `examples/` directory

---

## 🔧 Troubleshooting

**Device won't connect:**
- Check server is running: `curl http://localhost:8080/health`
- Check WebSocket URL is correct
- Check firewall allows WebSocket connections

**Pairing fails:**
- Codes expire after 5 minutes
- Codes are case-insensitive
- Check both devices are connected to server

**Sync not working:**
- Check devices are paired: look for "paired" message
- Check both devices are online
- Check server logs for errors

**Performance issues:**
- Enable delta compression
- Reduce sync frequency
- Use local network mode if on same WiFi
