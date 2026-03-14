-- ADI Embed Proxy - Initial Schema
-- Migration: 001_initial_schema

-- Platform-managed provider keys (our keys for reselling)
CREATE TABLE embed_platform_provider_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_type VARCHAR(50) NOT NULL UNIQUE,
    api_key_encrypted TEXT NOT NULL,
    base_url TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Platform allowed models (what we expose in platform mode)
CREATE TABLE embed_platform_allowed_models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_type VARCHAR(50) NOT NULL,
    model_id VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(provider_type, model_id)
);

-- User's upstream API keys (BYOK)
CREATE TABLE embed_upstream_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    provider_type VARCHAR(50) NOT NULL,
    api_key_encrypted TEXT NOT NULL,
    base_url TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id, name)
);

CREATE INDEX idx_embed_upstream_keys_user ON embed_upstream_api_keys(user_id);

-- Proxy tokens (issued to users for API access)
CREATE TABLE embed_proxy_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,

    -- Token identification
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    token_prefix VARCHAR(20) NOT NULL,

    -- Key source configuration
    key_mode VARCHAR(20) NOT NULL,
    upstream_key_id UUID REFERENCES embed_upstream_api_keys(id) ON DELETE SET NULL,
    platform_provider VARCHAR(50),

    -- Access control
    allowed_models TEXT[],
    blocked_models TEXT[],

    -- Logging options
    log_requests BOOLEAN NOT NULL DEFAULT false,
    log_responses BOOLEAN NOT NULL DEFAULT false,

    -- Status
    is_active BOOLEAN NOT NULL DEFAULT true,
    expires_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT embed_valid_key_source CHECK (
        (key_mode = 'byok' AND upstream_key_id IS NOT NULL AND platform_provider IS NULL) OR
        (key_mode = 'platform' AND upstream_key_id IS NULL AND platform_provider IS NOT NULL)
    ),
    UNIQUE(user_id, name)
);

CREATE INDEX idx_embed_proxy_tokens_user ON embed_proxy_tokens(user_id);
CREATE INDEX idx_embed_proxy_tokens_hash ON embed_proxy_tokens(token_hash);

-- Usage logs
CREATE TABLE embed_proxy_usage_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proxy_token_id UUID NOT NULL,
    user_id UUID NOT NULL,

    -- Correlation
    request_id VARCHAR(255) NOT NULL,
    upstream_request_id VARCHAR(255),

    -- Model info
    requested_model VARCHAR(255),
    actual_model VARCHAR(255),
    provider_type VARCHAR(50) NOT NULL,
    key_mode VARCHAR(20) NOT NULL,

    -- Token usage
    input_tokens INTEGER,
    total_tokens INTEGER,

    -- Embedding metadata
    dimensions INTEGER,
    input_count INTEGER,

    -- Cost
    reported_cost_usd DECIMAL(12,8),

    -- Request metadata
    endpoint VARCHAR(100) NOT NULL,

    -- Performance
    latency_ms INTEGER,

    -- Status
    status VARCHAR(20) NOT NULL,
    status_code SMALLINT,
    error_type VARCHAR(100),
    error_message TEXT,

    -- Optional request/response logging
    request_body JSONB,
    response_body JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_embed_usage_user_time ON embed_proxy_usage_log(user_id, created_at DESC);
CREATE INDEX idx_embed_usage_token_time ON embed_proxy_usage_log(proxy_token_id, created_at DESC);
CREATE INDEX idx_embed_usage_created ON embed_proxy_usage_log(created_at DESC);
