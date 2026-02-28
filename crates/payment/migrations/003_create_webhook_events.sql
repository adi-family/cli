CREATE TABLE IF NOT EXISTS webhook_events (
    id                UUID PRIMARY KEY,
    provider          VARCHAR(50)  NOT NULL,
    provider_event_id VARCHAR(255) NOT NULL,
    event_type        VARCHAR(100) NOT NULL,
    payload           JSONB        NOT NULL,
    processed         BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_webhook_events_provider_event
    ON webhook_events (provider, provider_event_id);
CREATE INDEX IF NOT EXISTS idx_webhook_events_processed ON webhook_events (processed);
