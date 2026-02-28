CREATE TABLE IF NOT EXISTS balances (
    user_id              UUID PRIMARY KEY,
    subscription_credits BIGINT       NOT NULL DEFAULT 0,
    extra_credits        BIGINT       NOT NULL DEFAULT 0,
    updated_at           TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS balance_transactions (
    id               UUID PRIMARY KEY,
    user_id          UUID         NOT NULL REFERENCES balances(user_id),
    payment_id       UUID         REFERENCES payments(id),
    transaction_type VARCHAR(50)  NOT NULL,
    pool             VARCHAR(20)  NOT NULL,
    amount           BIGINT       NOT NULL,
    balance_before   BIGINT       NOT NULL,
    balance_after    BIGINT       NOT NULL,
    conversion_rate  NUMERIC      NOT NULL DEFAULT 0,
    description      TEXT,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_balance_transactions_user_id ON balance_transactions (user_id);
CREATE INDEX IF NOT EXISTS idx_balance_transactions_payment_id ON balance_transactions (payment_id) WHERE payment_id IS NOT NULL;
