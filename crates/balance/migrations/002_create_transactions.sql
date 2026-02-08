-- Transaction type enum
DO $$ BEGIN
    CREATE TYPE transaction_type AS ENUM (
        'deposit',
        'debit',
        'adjustment',
        'transfer_in',
        'transfer_out'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Transaction status enum
DO $$ BEGIN
    CREATE TYPE transaction_status AS ENUM (
        'pending',
        'completed',
        'failed',
        'reversed'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Transactions table (audit trail)
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    balance_id UUID NOT NULL REFERENCES balances(id),
    transaction_type transaction_type NOT NULL,
    status transaction_status NOT NULL DEFAULT 'pending',
    amount BIGINT NOT NULL,
    balance_before BIGINT NOT NULL,
    balance_after BIGINT NOT NULL,
    description TEXT,
    reference_type VARCHAR(50),
    reference_id VARCHAR(255),
    metadata JSONB NOT NULL DEFAULT '{}',
    idempotency_key VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_transactions_user_id ON transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_transactions_balance_id ON transactions(balance_id);
CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_transactions_reference ON transactions(reference_type, reference_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_transactions_idempotency
    ON transactions(user_id, idempotency_key) WHERE idempotency_key IS NOT NULL;
