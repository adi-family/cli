-- Enable TimescaleDB extension (idempotent)
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Analytics events table (raw events storage)
CREATE TABLE analytics_events (
    id BIGSERIAL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type VARCHAR(100) NOT NULL,
    service VARCHAR(100) NOT NULL DEFAULT 'unknown',

    -- Actor information (indexed for fast user queries)
    user_id UUID,

    -- Full event data stored as JSONB for flexibility
    -- This contains all event-specific fields
    data JSONB NOT NULL,

    PRIMARY KEY (timestamp, id)
);

-- Convert to hypertable (time-series partitioning by day)
-- This enables automatic partitioning and better query performance
SELECT create_hypertable(
    'analytics_events',
    'timestamp',
    chunk_time_interval => INTERVAL '1 day'
);

-- Indexes for common query patterns
CREATE INDEX idx_analytics_events_user_id
    ON analytics_events(user_id, timestamp DESC)
    WHERE user_id IS NOT NULL;

CREATE INDEX idx_analytics_events_event_type
    ON analytics_events(event_type, timestamp DESC);

CREATE INDEX idx_analytics_events_service
    ON analytics_events(service, timestamp DESC);

-- GIN index for JSONB queries (search within event data)
CREATE INDEX idx_analytics_events_data_gin
    ON analytics_events USING GIN (data);

-- Add comment
COMMENT ON TABLE analytics_events IS
    'TimescaleDB hypertable storing all analytics events with automatic time-series partitioning';

-- Enable compression after 7 days (saves ~90% disk space)
-- Compressed chunks are still queryable but not updatable
ALTER TABLE analytics_events SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'event_type, service',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Auto-compress chunks older than 7 days
SELECT add_compression_policy('analytics_events', INTERVAL '7 days');

-- Add data retention policy (delete raw events older than 90 days)
-- Aggregated data will be kept longer in materialized views
SELECT add_retention_policy('analytics_events', INTERVAL '90 days');
