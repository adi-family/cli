ALTER TABLE payments ADD COLUMN IF NOT EXISTS subscription_id UUID REFERENCES subscriptions(id);

CREATE INDEX IF NOT EXISTS idx_payments_subscription_id ON payments (subscription_id) WHERE subscription_id IS NOT NULL;
