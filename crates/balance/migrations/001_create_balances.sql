-- User balance table (one balance per user)
-- Amount stored in microtokens: 1 AdiToken = 1,000,000 microtokens
CREATE TABLE IF NOT EXISTS balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL UNIQUE,
    amount BIGINT NOT NULL DEFAULT 0,
    currency VARCHAR(20) NOT NULL DEFAULT 'ADI_TOKEN',
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for user lookup
CREATE INDEX IF NOT EXISTS idx_balances_user_id ON balances(user_id);
CREATE INDEX IF NOT EXISTS idx_balances_updated_at ON balances(updated_at DESC);
