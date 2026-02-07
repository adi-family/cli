# Browser Debug via MCP - Implementation Plan

## Overview

Enable MCP clients to access browser debugging data (network, console) for pages served through ADI-proxied cocoons. The Hive proxy injects a debug token header, the browser extension captures it and collects debug data, and MCP tools query this data via the signaling server.

## Key Decisions

| Decision | Choice |
|----------|--------|
| Proxy location | Hive middleware |
| Token injection | Only `text/html` (configurable) |
| Token validation | Signature-only (stateless HMAC verification) |
| Data delivery | WebSocket streaming (real-time push) |
| Response body limit | Configurable |
| Event buffering | None - request/response model |
| Multi-browser ID | Extension generates unique `browser_id` on install |

## Architecture

```
+-----------------------------------------------------------------------------+
|                                   Hive                                       |
|                                                                              |
|   Middleware (per response):                                                |
|   1. Check Content-Type matches config (default: text/html)                 |
|   2. Generate: token = base64(hmac_sha256(cocoon_id|path|nonce, secret))    |
|   3. Inject header: X-ADI-Debug-Token: <token>                              |
|   No registration needed - signaling validates signature                    |
+--------------------------------------------------------------------------+--+
                                                                           |
                                                                           v
+-----------------------------------------------------------------------------+
|                           Chrome Extension                                   |
|                                                                              |
|   On install: generate browser_id = uuid(), store in chrome.storage         |
|                                                                              |
|   Flow:                                                                      |
|   1. Detect X-ADI-Debug-Token on main_frame text/html responses            |
|   2. Attach debugger to tab                                                 |
|   3. Connect to signaling (cookie auth) if not connected                    |
|   4. Send: browser_debug_tab_available { token, browser_id, url, title }    |
|   5. Stream events as they occur:                                           |
|      - browser_debug_network_event                                          |
|      - browser_debug_console_event                                          |
|   6. On MCP request -> respond with current data                            |
|   7. On tab close -> browser_debug_tab_closed                               |
+--------------------------------------------------------------------------+--+
                                                                           |
                                                                           v
+-----------------------------------------------------------------------------+
|                         Signaling Server                                     |
|                                                                              |
|   Token validation (stateless):                                             |
|   - Verify HMAC signature using shared hive_secret                          |
|   - Extract cocoon_id from token payload                                    |
|   - Check user owns that cocoon                                             |
|                                                                              |
|   State (minimal):                                                           |
|   - Active tabs: token -> { browser_conn_id, tab_info }                     |
|   - Connection cleanup: on disconnect, remove that connection's tabs        |
|                                                                              |
|   Routing:                                                                   |
|   - browser_debug_list_tabs -> return tabs for user's cocoons               |
|   - browser_debug_get_* -> route request to extension, return response      |
+--------------------------------------------------------------------------+--+
                                                                           |
                                                                           v
+-----------------------------------------------------------------------------+
|                      adi-browser-debug-plugin                                |
|                                                                              |
|   MCP Tools:                                                                 |
|   - browser_debug_list_tabs()                                               |
|   - browser_debug_get_network(token, filters?)                              |
|   - browser_debug_get_console(token, filters?)                              |
|                                                                              |
|   CLI:                                                                       |
|   - adi browser-debug list-tabs                                             |
|   - adi browser-debug network <token> [--filter...]                         |
|   - adi browser-debug console <token> [--filter...]                         |
+-----------------------------------------------------------------------------+
```

## Token Format

```
Token = base64url({
  "c": "<cocoon_id>",
  "p": "<request_path>",
  "n": "<random_nonce>",
  "t": <timestamp>,
  "s": "<hmac_signature>"
})

Signature = hmac_sha256(cocoon_id + path + nonce + timestamp, hive_secret)
```

Signaling server can verify without any database lookup - just recompute HMAC and compare.

## Configuration

### Hive (per-cocoon or global)

```yaml
browser_debug:
  enabled: true
  content_types: ["text/html"]
  response_body_max_size: 102400  # 100KB default, configurable
  exclude_paths: ["/health", "/metrics"]
```

### Extension (chrome.storage)

```json
{
  "browser_id": "uuid-generated-on-install",
  "signaling_url": "wss://adi.the-ihor.com/api/signaling/ws",
  "response_body_max_size": 102400
}
```

## Protocol Messages

### Extension -> Signaling

```typescript
// Tab available (on detecting debug header)
{
  type: "browser_debug_tab_available",
  token: string,
  browser_id: string,
  url: string,
  title: string,
  favicon?: string
}

// Tab closed
{ type: "browser_debug_tab_closed", token: string }

// Tab updated (SPA navigation)
{ type: "browser_debug_tab_updated", token: string, url: string, title: string }

// Network event (streamed)
{
  type: "browser_debug_network_event",
  token: string,
  event: "request" | "response" | "finished" | "failed",
  data: {
    request_id: string,
    timestamp: number,
    // request: method, url, headers, body
    // response: status, status_text, headers, mime_type
    // finished: body, body_truncated, duration_ms
    // failed: error
    ...
  }
}

// Console event (streamed)
{
  type: "browser_debug_console_event",
  token: string,
  entry: {
    timestamp: number,
    level: "log" | "debug" | "info" | "warn" | "error",
    message: string,
    args: any[],
    source?: string,
    line?: number,
    column?: number,
    stack_trace?: string
  }
}
```

### MCP Plugin <-> Signaling

```typescript
// Request: List tabs
{ type: "browser_debug_list_tabs" }

// Response: Tabs list
{
  type: "browser_debug_tabs",
  tabs: [{
    token: string,
    browser_id: string,
    url: string,
    title: string,
    cocoon_id: string,
    cocoon_name?: string
  }]
}

// Request: Get network (routed to extension)
{
  type: "browser_debug_get_network",
  request_id: string,  // For correlating response
  token: string,
  filters?: {
    url_pattern?: string,
    method?: string[],
    status_min?: number,
    status_max?: number,
    since?: number,
    limit?: number
  }
}

// Response: Network data (from extension via signaling)
{
  type: "browser_debug_network_data",
  request_id: string,
  requests: NetworkRequest[]
}

// Request: Get console (routed to extension)
{
  type: "browser_debug_get_console",
  request_id: string,
  token: string,
  filters?: {
    level?: string[],
    message_pattern?: string,
    since?: number,
    limit?: number
  }
}

// Response: Console data
{
  type: "browser_debug_console_data",
  request_id: string,
  entries: ConsoleEntry[]
}
```

## Data Types

```typescript
interface NetworkRequest {
  request_id: string;
  timestamp: number;
  
  // Request
  method: string;
  url: string;
  request_headers?: Record<string, string>;
  request_body?: string;
  
  // Response (filled in later events)
  status?: number;
  status_text?: string;
  response_headers?: Record<string, string>;
  response_body?: string;        // May be truncated for large responses
  response_body_truncated?: boolean;
  mime_type?: string;
  
  // Timing
  duration_ms?: number;
  
  // Error (if failed)
  error?: string;
}

interface ConsoleEntry {
  timestamp: number;
  level: "log" | "debug" | "info" | "warn" | "error";
  args: any[];                   // Serialized console arguments
  message: string;               // Formatted message string
  source?: string;               // Source file URL
  line?: number;
  column?: number;
  stack_trace?: string;          // For errors
}
```

## Components to Create/Modify

| Component | Type | Description |
|-----------|------|-------------|
| `lib-tarminal-sync` | Modify | Add `browser_debug_*` message types |
| `signaling-server` | Modify | Handle messages, token validation, request routing |
| `apps/chrome-extension-debugger` | Modify | Token detection, WebSocket client, event streaming |
| `crates/adi-browser-debug/core` | New | Types, protocol definitions |
| `crates/adi-browser-debug/plugin` | New | MCP tools, CLI, signaling client |
| `crates/hive` | Modify | HTTP middleware for token injection |

## File Structure

```
crates/adi-browser-debug/
├── core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types.rs          # NetworkRequest, ConsoleEntry
│       └── protocol.rs       # Message types (shared with signaling)
└── plugin/
    ├── Cargo.toml
    ├── plugin.toml
    └── src/
        ├── lib.rs            # Plugin entry
        ├── tools.rs          # MCP tool implementations
        ├── client.rs         # Signaling WebSocket client
        └── cli.rs            # CLI command handlers

apps/chrome-extension-debugger/
├── manifest.json             # Add permissions
├── background.js             # Add WebSocket, token detection, streaming
├── popup.html                # New: status UI
└── popup.js                  # New: popup logic
```

## MCP Tool Schemas

### browser_debug_list_tabs

```json
{
  "name": "browser_debug_list_tabs",
  "description": "List all browser tabs with debug tokens available for inspection",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

### browser_debug_get_network

```json
{
  "name": "browser_debug_get_network",
  "description": "Get network requests from a browser tab",
  "inputSchema": {
    "type": "object",
    "properties": {
      "token": { "type": "string", "description": "Debug token from browser_debug_list_tabs" },
      "url_pattern": { "type": "string", "description": "Filter by URL regex pattern" },
      "method": { 
        "type": "array", 
        "items": { "type": "string" },
        "description": "Filter by HTTP methods, e.g. ['GET', 'POST']"
      },
      "status_min": { "type": "integer", "description": "Minimum status code (inclusive)" },
      "status_max": { "type": "integer", "description": "Maximum status code (inclusive)" },
      "since": { "type": "integer", "description": "Only requests after this timestamp (ms)" },
      "limit": { "type": "integer", "description": "Max number of requests to return" }
    },
    "required": ["token"]
  }
}
```

### browser_debug_get_console

```json
{
  "name": "browser_debug_get_console",
  "description": "Get console logs from a browser tab",
  "inputSchema": {
    "type": "object",
    "properties": {
      "token": { "type": "string", "description": "Debug token from browser_debug_list_tabs" },
      "level": {
        "type": "array",
        "items": { "type": "string", "enum": ["log", "debug", "info", "warn", "error"] },
        "description": "Filter by log levels"
      },
      "message_pattern": { "type": "string", "description": "Filter by message regex pattern" },
      "since": { "type": "integer", "description": "Only entries after this timestamp (ms)" },
      "limit": { "type": "integer", "description": "Max number of entries to return" }
    },
    "required": ["token"]
  }
}
```

## Implementation Order

| Phase | Component | Tasks | Effort | Status |
|-------|-----------|-------|--------|--------|
| 1 | `lib-tarminal-sync` | Add `browser_debug_*` message types | S | DONE |
| 2 | `signaling-server` | Handle new messages, token validation, routing | M | DONE |
| 3 | `chrome-extension-debugger` | Token detection, WebSocket, streaming | M | DONE |
| 4 | `adi-browser-debug/core` | Shared types | S | DONE |
| 5 | `adi-browser-debug/plugin` | MCP tools, CLI | M | DONE |
| 6 | `hive` | HTTP proxy with token injection | M | DONE |

**Testing strategy**: Use `ENABLE_PROXY=true` to enable the HTTP proxy in Hive, then access cocoon services via `http://hive-host:8081/proxy/{cocoon_id}/path`.

### HTTP Proxy Usage
To enable browser debug token injection in Hive:

```bash
# Required environment variables
export ENABLE_PROXY=true
export HIVE_SECRET=shared-secret-with-signaling-server  # Must match signaling server's HIVE_SECRET
export PROXY_PORT=8081  # Optional, default 8081

# Start Hive
./hive
```

Access cocoon services via the proxy:
- `http://hive-host:8081/proxy/{cocoon_id}/` - Proxies to cocoon container
- Text/HTML responses automatically get `X-ADI-Debug-Token` header injected

The Chrome extension will detect this header and enable debugging for that tab.

## Security Considerations

| Concern | Mitigation |
|---------|------------|
| Token forgery | HMAC signature with shared secret between hive and signaling |
| Token reuse | Token bound to single tab, invalidated on close/navigate |
| Cross-user access | Signaling server validates user owns the cocoon that issued the token |
| Data leakage | Debug data only in-memory, cleared on tab close |
| Extension spoofing | Extension authenticated via cookies (same-origin) |
