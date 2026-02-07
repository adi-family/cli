-- Credential types enum
DO $$ BEGIN
    CREATE TYPE credential_type AS ENUM (
        'github_token',
        'gitlab_token',
        'api_key',
        'oauth2',
        'ssh_key',
        'password',
        'certificate',
        'custom'
    );
EXCEPTION WHEN duplicate_object THEN null;
END $$;

-- Main credentials table
CREATE TABLE IF NOT EXISTS credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    credential_type credential_type NOT NULL,
    
    -- Encrypted credential data (ChaCha20-Poly1305)
    -- Format: ENC:{base64(nonce + ciphertext)}
    encrypted_data TEXT NOT NULL,
    
    -- Optional metadata (not encrypted, for filtering/display)
    metadata JSONB NOT NULL DEFAULT '{}',
    
    -- Provider info (e.g., 'github.com', 'gitlab.com', 'openai')
    provider VARCHAR(255),
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT true,
    
    -- Optional expiration
    expires_at TIMESTAMPTZ,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    
    -- Ensure unique name per user
    UNIQUE(user_id, name)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_credentials_user_id ON credentials(user_id);
CREATE INDEX IF NOT EXISTS idx_credentials_type ON credentials(credential_type);
CREATE INDEX IF NOT EXISTS idx_credentials_provider ON credentials(provider);
CREATE INDEX IF NOT EXISTS idx_credentials_active ON credentials(is_active) WHERE is_active = true;

-- Access log for auditing
CREATE TABLE IF NOT EXISTS credential_access_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    credential_id UUID NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    action VARCHAR(50) NOT NULL, -- 'read', 'update', 'delete', 'use'
    ip_address INET,
    user_agent TEXT,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_credential_access_log_credential_id ON credential_access_log(credential_id);
CREATE INDEX IF NOT EXISTS idx_credential_access_log_user_id ON credential_access_log(user_id);
CREATE INDEX IF NOT EXISTS idx_credential_access_log_created_at ON credential_access_log(created_at);
