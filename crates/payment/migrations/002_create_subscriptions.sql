CREATE TABLE IF NOT EXISTS subscriptions (
    id                       UUID PRIMARY KEY,
    provider                 VARCHAR(50)  NOT NULL,
    provider_subscription_id VARCHAR(255),
    user_id                  UUID         NOT NULL,
    plan_id                  VARCHAR(255) NOT NULL,
    status                   VARCHAR(50)  NOT NULL DEFAULT 'pending',
    billing_interval         VARCHAR(20),
    amount_cents             BIGINT,
    currency                 VARCHAR(10),
    current_period_start     TIMESTAMPTZ,
    current_period_end       TIMESTAMPTZ,
    metadata                 JSONB,
    created_at               TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions (user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_provider_sub_id ON subscriptions (provider, provider_subscription_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_status ON subscriptions (status);
