CREATE TABLE IF NOT EXISTS payments (
    id              UUID PRIMARY KEY,
    provider        VARCHAR(50)  NOT NULL,
    provider_payment_id VARCHAR(255),
    user_id         UUID         NOT NULL,
    amount_cents    BIGINT       NOT NULL,
    currency        VARCHAR(10)  NOT NULL DEFAULT 'USD',
    status          VARCHAR(50)  NOT NULL DEFAULT 'pending',
    checkout_url    TEXT,
    metadata        JSONB,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_payments_user_id ON payments (user_id);
CREATE INDEX IF NOT EXISTS idx_payments_provider_payment_id ON payments (provider, provider_payment_id);
CREATE INDEX IF NOT EXISTS idx_payments_status ON payments (status);
