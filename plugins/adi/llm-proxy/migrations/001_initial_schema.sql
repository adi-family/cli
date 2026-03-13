-- ADI API Proxy - Initial Schema
-- Migration: 001_initial_schema

-- Platform-managed provider keys (our keys for reselling)
CREATE TABLE platform_provider_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_type VARCHAR(50) NOT NULL UNIQUE,  -- 'openrouter', 'openai', 'anthropic'
    api_key_encrypted TEXT NOT NULL,            -- ChaCha20-Poly1305 encrypted
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
    api_key_encrypted TEXT NOT NULL,            -- ChaCha20-Poly1305 encrypted
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
