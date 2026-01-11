# ADI API Proxy - Implementation Plan

> LLM API Token Proxy with BYOK/Platform modes, Rhai scripting, and comprehensive analytics

## Overview

| Item | Value |
|------|-------|
| **Service** | adi-api-proxy |
| **Port** | 8024 |
| **Purpose** | LLM API proxy with BYOK/Platform modes, Rhai scripting, full analytics |
| **Streaming** | Token counting via final `usage` object |
| **Request logging** | Opt-in per token (with cost implications) |
| **Model listing** | Platform-configurable allowlist |

## Key Features

1. **BYOK Mode**: Users provide their own upstream API keys, we proxy + log analytics
2. **Platform Mode**: Users use our platform keys (reselling), we charge based on analytics
3. **Rhai Scripting**: Full request/response transformation with sandboxed Rhai engine
4. **Rich Analytics**: All data logged for post-facto cost calculation (per-token billing)
5. **Streaming Support**: SSE streaming with token counting from final usage object
6. **Multi-Provider**: OpenAI-compatible, Anthropic, OpenRouter, Custom endpoints

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          adi-api-proxy (:8024)                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │                      Request Pipeline                              │ │
│  │  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────┐ │ │
│  │  │ Proxy   │→ │ Rhai     │→ │ Provider │→ │ Rhai     │→ │ Log  │ │ │
│  │  │ Auth    │  │ Request  │  │ Forward  │  │ Response │  │ +    │ │ │
│  │  │         │  │ Transform│  │          │  │ Transform│  │ Send │ │ │
│  │  └─────────┘  └──────────┘  └──────────┘  └──────────┘  └──────┘ │ │
│  └───────────────────────────────────────────────────────────────────┘ │
│                                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐  │
│  │ Provider     │  │ Management   │  │ Analytics Client             │  │
│  │ Adapters     │  │ API          │  │ (lib-analytics-core)         │  │
│  │ - OpenAI     │  │ /api/proxy/* │  │                              │  │
│  │ - Anthropic  │  │              │  │  → adi-analytics-ingestion   │  │
│  │ - OpenRouter │  │              │  │                              │  │
│  │ - Custom     │  │              │  │                              │  │
│  └──────────────┘  └──────────────┘  └──────────────────────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Crate Structure

```
crates/adi-api-proxy/
├── core/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── types.rs                 # All domain types
│   │   ├── error.rs                 # Error types
│   │   ├── crypto.rs                # AES-256-GCM encryption for API keys
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── keys.rs              # upstream_api_keys CRUD
│   │   │   ├── tokens.rs            # proxy_tokens CRUD
│   │   │   ├── platform_keys.rs     # platform_provider_keys CRUD
│   │   │   ├── usage.rs             # Usage logging
│   │   │   └── models.rs            # platform_allowed_models CRUD
│   │   ├── transform/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs            # Rhai engine setup with sandboxing
│   │   │   └── context.rs           # Request/Response context for scripts
│   │   └── providers/
│   │       ├── mod.rs
│   │       ├── traits.rs            # Provider trait
│   │       ├── openai.rs            # OpenAI-compatible (includes OpenRouter)
│   │       ├── anthropic.rs         # Anthropic Messages API
│   │       └── custom.rs            # User-defined base URL
│   └── migrations/
│       ├── 001_initial_schema.up.sql
│       └── 001_initial_schema.down.sql
│
├── http/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── state.rs
│   │   ├── routes/
│   │   │   ├── mod.rs
│   │   │   ├── proxy.rs             # /v1/* proxy endpoints
│   │   │   ├── keys.rs              # Upstream key management
│   │   │   ├── tokens.rs            # Proxy token management
│   │   │   ├── providers.rs         # List available platform providers
│   │   │   └── usage.rs             # Query usage logs
│   │   ├── middleware/
│   │   │   ├── mod.rs
│   │   │   └── proxy_auth.rs        # Proxy token authentication
│   │   └── handlers/
│   │       ├── mod.rs
│   │       ├── chat.rs              # /v1/chat/completions (streaming)
│   │       ├── completions.rs       # /v1/completions
│   │       ├── embeddings.rs        # /v1/embeddings
│   │       ├── models.rs            # /v1/models
│   │       └── messages.rs          # /v1/messages (Anthropic)
│   └── Cargo.toml
│
└── plugin/                          # Optional CLI plugin
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

---

## Database Schema

### Migration: 001_initial_schema.up.sql

```sql
-- Platform-managed provider keys (our keys for reselling)
CREATE TABLE platform_provider_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_type VARCHAR(50) NOT NULL UNIQUE,  -- 'openrouter', 'openai', 'anthropic'
    api_key_encrypted TEXT NOT NULL,            -- AES-256-GCM encrypted
    base_url TEXT,                              -- Custom endpoint (optional)
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Platform allowed models (what we expose in platform mode)
CREATE TABLE platform_allowed_models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_type VARCHAR(50) NOT NULL,
    model_id VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(provider_type, model_id)
);

-- User's upstream API keys (BYOK)
CREATE TABLE upstream_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    provider_type VARCHAR(50) NOT NULL,         -- 'openrouter', 'openai', 'anthropic', 'custom'
    api_key_encrypted TEXT NOT NULL,            -- AES-256-GCM encrypted
    base_url TEXT,                              -- For custom providers or overrides
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(user_id, name)
);

CREATE INDEX idx_upstream_keys_user ON upstream_api_keys(user_id);

-- Proxy tokens (issued to users for API access)
CREATE TABLE proxy_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    
    -- Token identification
    token_hash VARCHAR(255) NOT NULL UNIQUE,    -- SHA-256 hash for lookup
    token_prefix VARCHAR(20) NOT NULL,          -- 'adi_pk_xxx...' for display
    
    -- Key source configuration
    key_mode VARCHAR(20) NOT NULL,              -- 'byok', 'platform'
    upstream_key_id UUID REFERENCES upstream_api_keys(id) ON DELETE SET NULL,
    platform_provider VARCHAR(50),              -- 'openrouter', 'openai', 'anthropic'
    
    -- Rhai transformation scripts (nullable = no transform)
    request_script TEXT,                        -- Rhai script for request modification
    response_script TEXT,                       -- Rhai script for response modification
    
    -- Access control
    allowed_models TEXT[],                      -- NULL = all allowed
    blocked_models TEXT[],                      -- NULL = none blocked
    
    -- Logging options (opt-in, costs apply)
    log_requests BOOLEAN NOT NULL DEFAULT false,
    log_responses BOOLEAN NOT NULL DEFAULT false,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT true,
    expires_at TIMESTAMPTZ,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT valid_key_source CHECK (
        (key_mode = 'byok' AND upstream_key_id IS NOT NULL AND platform_provider IS NULL) OR
        (key_mode = 'platform' AND upstream_key_id IS NULL AND platform_provider IS NOT NULL)
    ),
    UNIQUE(user_id, name)
);

CREATE INDEX idx_proxy_tokens_user ON proxy_tokens(user_id);
CREATE INDEX idx_proxy_tokens_hash ON proxy_tokens(token_hash);

-- Usage logs (also sent to analytics service)
CREATE TABLE proxy_usage_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proxy_token_id UUID NOT NULL,
    user_id UUID NOT NULL,
    
    -- Correlation
    request_id VARCHAR(255) NOT NULL,           -- Our ID for tracing
    upstream_request_id VARCHAR(255),           -- Provider's ID (e.g., OpenRouter generation ID)
    
    -- Model info
    requested_model VARCHAR(255),               -- What client requested
    actual_model VARCHAR(255),                  -- What provider returned
    provider_type VARCHAR(50) NOT NULL,
    key_mode VARCHAR(20) NOT NULL,
    
    -- Token usage (from response)
    input_tokens INTEGER,
    output_tokens INTEGER,
    total_tokens INTEGER,
    
    -- Provider-reported cost (when available, e.g., OpenRouter)
    reported_cost_usd DECIMAL(12,8),
    
    -- Request metadata
    endpoint VARCHAR(100) NOT NULL,             -- '/v1/chat/completions', '/v1/embeddings', etc.
    is_streaming BOOLEAN NOT NULL DEFAULT false,
    
    -- Performance
    latency_ms INTEGER,
    ttft_ms INTEGER,                            -- Time to first token (streaming)
    
    -- Status
    status VARCHAR(20) NOT NULL,                -- 'success', 'error', 'upstream_error'
    status_code SMALLINT,
    error_type VARCHAR(100),
    error_message TEXT,
    
    -- Optional request/response logging (when enabled)
    request_body JSONB,
    response_body JSONB,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for analytics queries
CREATE INDEX idx_proxy_usage_user_time ON proxy_usage_log(user_id, created_at DESC);
CREATE INDEX idx_proxy_usage_token_time ON proxy_usage_log(proxy_token_id, created_at DESC);
CREATE INDEX idx_proxy_usage_created ON proxy_usage_log(created_at DESC);
```

### Migration: 001_initial_schema.down.sql

```sql
DROP TABLE IF EXISTS proxy_usage_log;
DROP TABLE IF EXISTS proxy_tokens;
DROP TABLE IF EXISTS upstream_api_keys;
DROP TABLE IF EXISTS platform_allowed_models;
DROP TABLE IF EXISTS platform_provider_keys;
```

---

## API Endpoints

### Management API (JWT auth via platform token)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/proxy/keys` | Add upstream API key |
| `GET` | `/api/proxy/keys` | List user's upstream keys |
| `GET` | `/api/proxy/keys/:id` | Get key details |
| `PATCH` | `/api/proxy/keys/:id` | Update key |
| `DELETE` | `/api/proxy/keys/:id` | Delete key |
| `POST` | `/api/proxy/keys/:id/verify` | Test key connectivity |
| `GET` | `/api/proxy/providers` | List available platform providers + allowed models |
| `POST` | `/api/proxy/tokens` | Create proxy token (**returns secret once**) |
| `GET` | `/api/proxy/tokens` | List user's proxy tokens |
| `GET` | `/api/proxy/tokens/:id` | Get token config (not the secret) |
| `PATCH` | `/api/proxy/tokens/:id` | Update token config |
| `DELETE` | `/api/proxy/tokens/:id` | Revoke token |
| `POST` | `/api/proxy/tokens/:id/rotate` | Regenerate token secret |
| `GET` | `/api/proxy/usage` | Query usage logs |
| `GET` | `/api/proxy/usage/export` | Export usage as CSV/JSON |

### Proxy API (Proxy token auth via `Authorization: Bearer adi_pk_xxx`)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/chat/completions` | Chat completion (streaming supported) |
| `POST` | `/v1/completions` | Legacy text completion |
| `POST` | `/v1/embeddings` | Embeddings |
| `GET` | `/v1/models` | List allowed models |
| `POST` | `/v1/messages` | Anthropic Messages API |

---

## Provider Trait

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider identifier
    fn provider_type(&self) -> &'static str;
    
    /// Forward a request to the upstream provider
    async fn forward(
        &self,
        api_key: &str,
        endpoint: &str,
        request: ProxyRequest,
    ) -> Result<ProxyResponse, ProviderError>;
    
    /// Forward a streaming request
    async fn forward_stream(
        &self,
        api_key: &str,
        endpoint: &str,
        request: ProxyRequest,
    ) -> Result<impl Stream<Item = Result<Bytes, ProviderError>>, ProviderError>;
    
    /// Extract usage info from response
    fn extract_usage(&self, response: &ProxyResponse) -> Option<UsageInfo>;
    
    /// Extract cost from response (if provider reports it, e.g., OpenRouter)
    fn extract_cost(&self, response: &ProxyResponse) -> Option<Decimal>;
    
    /// List available models
    async fn list_models(&self, api_key: &str) -> Result<Vec<ModelInfo>, ProviderError>;
}
```

---

## Rhai Transform Engine

### Security Configuration

```rust
fn create_rhai_engine() -> Engine {
    let mut engine = Engine::new();
    
    // Safety limits
    engine.set_max_operations(10_000);
    engine.set_max_call_levels(16);
    engine.set_max_expr_depths(64, 64);
    engine.set_max_string_size(100_000);  // 100KB strings
    engine.set_max_array_size(1_000);
    engine.set_max_map_size(100);
    engine.set_max_variables(50);
    engine.set_max_functions(20);
    engine.set_max_modules(0);  // No module imports
    
    // Disable dangerous features
    engine.disable_symbol("eval");
    
    engine
}
```

### Transform Context Types

```rust
// Passed to request transform script
struct RequestContext {
    method: String,
    path: String,
    headers: Map<String, String>,
    body: Dynamic,  // Parsed JSON body
    model: Option<String>,
}

// Passed to response transform script  
struct ResponseContext {
    status_code: i64,
    headers: Map<String, String>,
    body: Dynamic,  // Parsed JSON body
    model: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}
```

### Example Scripts

**Request Transform** (inject system prompt):
```rhai
// Inject system prompt if not present
if body.messages.len() == 0 || body.messages[0].role != "system" {
    body.messages.insert(0, #{
        role: "system",
        content: "You are a helpful assistant."
    });
}

// Override temperature
body.temperature = 0.7;
```

**Response Transform** (strip metadata):
```rhai
// Remove system fingerprint
body.remove("system_fingerprint");

// Add custom field
body.proxy_metadata = #{
    processed_at: timestamp(),
    proxy_version: "1.0"
};
```

---

## Analytics Integration

### New Event Type

Add to `lib-analytics-core/src/events.rs`:

```rust
// ===== API Proxy Events =====
/// LLM API proxy request completed
ProxyRequest {
    proxy_token_id: Uuid,
    user_id: Uuid,
    request_id: String,
    upstream_request_id: Option<String>,
    requested_model: Option<String>,
    actual_model: Option<String>,
    provider_type: String,
    key_mode: String,              // 'byok', 'platform'
    endpoint: String,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    total_tokens: Option<i32>,
    reported_cost_usd: Option<f64>,
    latency_ms: i32,
    ttft_ms: Option<i32>,
    is_streaming: bool,
    status: String,
    status_code: Option<i32>,
    error_type: Option<String>,
    error_message: Option<String>,
},
```

### Integration

```rust
// In handler after request completes
state.analytics.track(AnalyticsEvent::ProxyRequest {
    proxy_token_id: token.id,
    user_id: token.user_id,
    request_id: request_id.clone(),
    upstream_request_id,
    requested_model: Some(requested_model),
    actual_model,
    provider_type: provider.provider_type().to_string(),
    key_mode: token.key_mode.to_string(),
    endpoint: "/v1/chat/completions".to_string(),
    input_tokens: usage.as_ref().map(|u| u.input_tokens as i32),
    output_tokens: usage.as_ref().map(|u| u.output_tokens as i32),
    total_tokens: usage.as_ref().map(|u| u.total_tokens as i32),
    reported_cost_usd: cost.map(|c| c.to_f64().unwrap_or(0.0)),
    latency_ms: latency.as_millis() as i32,
    ttft_ms: ttft.map(|t| t.as_millis() as i32),
    is_streaming,
    status: "success".to_string(),
    status_code: Some(200),
    error_type: None,
    error_message: None,
});
```

---

## Request Flow

```
1. Receive request at /v1/chat/completions
   │
2. Extract proxy token from Authorization header
   │
3. Lookup proxy_token by hash
   │   ├─ Not found → 401 Unauthorized
   │   ├─ Inactive/expired → 403 Forbidden  
   │   └─ Found → continue
   │
4. Check model against allowed_models/blocked_models
   │   └─ Blocked → 403 Forbidden
   │
5. Get upstream API key
   │   ├─ BYOK → decrypt user's upstream_api_key
   │   └─ Platform → decrypt platform_provider_key
   │
6. Parse request body as JSON
   │
7. Run request_script (Rhai) if configured
   │   └─ Script error → 400 Bad Request
   │
8. Forward to upstream provider
   │   ├─ Streaming → stream response back
   │   └─ Non-streaming → wait for response
   │
9. Run response_script (Rhai) if configured
   │
10. Extract usage info (tokens, cost from provider if available)
   │
11. Log to proxy_usage_log table (with request/response body if enabled)
   │
12. Send analytics event to adi-analytics-ingestion
   │
13. Return response to client
```

---

## Environment Variables

```bash
# Required
DATABASE_URL=postgres://postgres:postgres@localhost/adi_api_proxy
ENCRYPTION_KEY=<64-char-hex>  # 32-byte key for AES-256-GCM
JWT_SECRET=<same-as-platform>

# Optional
PORT=8024
ANALYTICS_URL=http://localhost:8094
RUST_LOG=info,adi_api_proxy=debug
```

---

## Dependencies

```toml
[dependencies]
# Web framework
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# HTTP client (for upstream requests)
reqwest = { version = "0.12", features = ["json", "stream"] }

# Scripting
rhai = { version = "1.19", features = ["serde"] }

# Crypto
aes-gcm = "0.10"
sha2 = "0.10"
rand = "0.8"
hex = "0.4"

# Auth
jsonwebtoken = "9"

# Analytics
lib-analytics-core = { path = "../../lib/lib-analytics-core" }

# Streaming
futures = "0.3"
async-stream = "0.3"
tokio-stream = "0.1"

# Utils
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

## Implementation Tasks

### Phase 1: Core Infrastructure
- [ ] Create crate structure (adi-api-proxy/core, http)
- [ ] Add database migrations
- [ ] Implement crypto module (AES-256-GCM)
- [ ] Define types (ProviderType, KeyMode, ProxyToken, UsageInfo, etc.)
- [ ] Define error types
- [ ] Implement DB operations for upstream_api_keys CRUD
- [ ] Implement DB operations for proxy_tokens CRUD
- [ ] Implement DB operations for platform_provider_keys CRUD

### Phase 2: Provider Adapters
- [ ] Define LlmProvider trait
- [ ] Implement OpenAI-compatible provider adapter
- [ ] Implement Anthropic provider adapter
- [ ] Implement Custom provider adapter (user-defined base URL)

### Phase 3: Transform Engine
- [ ] Setup Rhai engine with security sandboxing
- [ ] Define RequestContext and ResponseContext types for Rhai
- [ ] Implement transform execution logic (request + response)

### Phase 4: HTTP Server
- [ ] Create HTTP server main.rs with config and state
- [ ] Implement proxy token authentication middleware
- [ ] Implement management API - keys routes (CRUD + verify)
- [ ] Implement management API - tokens routes (CRUD + rotate)
- [ ] Implement management API - providers route (list platform providers)
- [ ] Implement management API - usage routes (query + export)
- [ ] Implement proxy route - POST /v1/chat/completions (non-streaming)
- [ ] Implement proxy route - POST /v1/completions
- [ ] Implement proxy route - POST /v1/embeddings
- [ ] Implement proxy route - GET /v1/models
- [ ] Implement proxy route - POST /v1/messages (Anthropic)

### Phase 5: Streaming Support
- [ ] Implement SSE streaming for /v1/chat/completions
- [ ] Extract tokens from final usage object in stream
- [ ] Track time-to-first-token for streaming requests

### Phase 6: Analytics Integration
- [ ] Add ProxyRequest event to lib-analytics-core
- [ ] Integrate AnalyticsClient into adi-api-proxy
- [ ] Implement local usage logging to proxy_usage_log table

### Phase 7: Polish
- [ ] Add platform_allowed_models table for configurable model allowlist
- [ ] Add adi-api-proxy to docker-compose.yml
- [ ] Update dev.sh script with new service
- [ ] Add release configuration (Dockerfile, docker-compose for production)
- [ ] Update workspace Cargo.toml with new crate

---

## Estimated Effort

| Phase | Tasks | Estimate |
|-------|-------|----------|
| Phase 1: Core | 8 tasks | ~4 hours |
| Phase 2: Providers | 4 tasks | ~3 hours |
| Phase 3: Transforms | 3 tasks | ~2 hours |
| Phase 4: HTTP Server | 11 tasks | ~6 hours |
| Phase 5: Streaming | 3 tasks | ~3 hours |
| Phase 6: Analytics | 3 tasks | ~1 hour |
| Phase 7: Polish | 5 tasks | ~2 hours |
| **Total** | **37 tasks** | **~21 hours** |

---

## Files to Create

```
crates/adi-api-proxy/
├── core/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── types.rs
│   │   ├── error.rs
│   │   ├── crypto.rs
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── keys.rs
│   │   │   ├── tokens.rs
│   │   │   ├── platform_keys.rs
│   │   │   ├── usage.rs
│   │   │   └── models.rs
│   │   ├── transform/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs
│   │   │   └── context.rs
│   │   └── providers/
│   │       ├── mod.rs
│   │       ├── traits.rs
│   │       ├── openai.rs
│   │       ├── anthropic.rs
│   │       └── custom.rs
│   └── migrations/
│       ├── 001_initial_schema.up.sql
│       └── 001_initial_schema.down.sql
│
├── http/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── state.rs
│       ├── routes/
│       │   ├── mod.rs
│       │   ├── proxy.rs
│       │   ├── keys.rs
│       │   ├── tokens.rs
│       │   ├── providers.rs
│       │   └── usage.rs
│       ├── middleware/
│       │   ├── mod.rs
│       │   └── proxy_auth.rs
│       └── handlers/
│           ├── mod.rs
│           ├── chat.rs
│           ├── completions.rs
│           ├── embeddings.rs
│           ├── models.rs
│           └── messages.rs
│
└── plugin/
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

## Files to Modify

- `crates/lib/lib-analytics-core/src/events.rs` - Add `ProxyRequest` event
- `Cargo.toml` (workspace) - Add new crate members
- `docker-compose.yml` - Add service
- `scripts/dev.sh` - Add service support

---

## Quick Start (After Implementation)

```bash
# 1. Run migrations
cd crates/adi-api-proxy/core
sqlx database create
sqlx migrate run

# 2. Set environment
export DATABASE_URL=postgres://postgres:postgres@localhost/adi_api_proxy
export ENCRYPTION_KEY=$(openssl rand -hex 32)
export JWT_SECRET=<your-jwt-secret>
export ANALYTICS_URL=http://localhost:8094

# 3. Run service
cargo run -p adi-api-proxy-http

# 4. Test
curl http://localhost:8024/health
```

---

## Notes

- Cost calculation happens post-facto via analytics data, not real-time
- Token-based billing calculated from analytics aggregates
- All provider costs are logged when available (OpenRouter reports cost in response)
- Rhai scripts are sandboxed with strict limits to prevent abuse
- Request/response body logging is opt-in per token due to storage costs
