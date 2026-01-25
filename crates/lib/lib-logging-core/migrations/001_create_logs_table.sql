-- Enable TimescaleDB extension (idempotent)
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Logs table (centralized log storage)
CREATE TABLE logs (
    id BIGSERIAL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Service identification
    service VARCHAR(100) NOT NULL,
    hostname VARCHAR(255),
    environment VARCHAR(50),
    version VARCHAR(50),
    
    -- Log level (trace=0, debug=1, info=2, notice=3, warn=4, error=5, fatal=6)
    level SMALLINT NOT NULL,
    level_name VARCHAR(10) NOT NULL,
    
    -- Message
    message TEXT NOT NULL,
    
    -- Distributed tracing (hierarchical correlation)
    trace_id UUID NOT NULL,       -- Request chain identifier (propagated across services)
    span_id UUID NOT NULL,        -- Current operation identifier
    parent_span_id UUID,          -- Parent span (for call hierarchy)
    
    -- Structured data
    fields JSONB DEFAULT '{}',
    
    -- Error details (for error/fatal logs)
    error_kind VARCHAR(255),
    error_message TEXT,
    error_stack_trace TEXT,
    
    -- Source location
    source VARCHAR(255),          -- file:line
    target VARCHAR(255),          -- module path
    
    PRIMARY KEY (timestamp, id)
);

-- Convert to hypertable (time-series partitioning by day)
SELECT create_hypertable(
    'logs',
    'timestamp',
    chunk_time_interval => INTERVAL '1 day'
);

-- Index for trace-based queries (find all logs in a request chain)
CREATE INDEX idx_logs_trace_id
    ON logs(trace_id, timestamp DESC);

-- Index for span-based queries (find logs for specific operation)
CREATE INDEX idx_logs_span_id
    ON logs(span_id, timestamp DESC);

-- Index for service-based queries
CREATE INDEX idx_logs_service
    ON logs(service, timestamp DESC);

-- Index for level-based queries (find errors, warnings, etc.)
CREATE INDEX idx_logs_level
    ON logs(level, timestamp DESC)
    WHERE level >= 4;  -- Only index warn and above for efficiency

-- Index for service + level combination (common query pattern)
CREATE INDEX idx_logs_service_level
    ON logs(service, level, timestamp DESC);

-- GIN index for JSONB field queries
CREATE INDEX idx_logs_fields_gin
    ON logs USING GIN (fields);

-- Full-text search on message
CREATE INDEX idx_logs_message_search
    ON logs USING GIN (to_tsvector('english', message));

-- Add comment
COMMENT ON TABLE logs IS
    'TimescaleDB hypertable storing centralized application logs with distributed tracing support';

-- Enable compression after 7 days
ALTER TABLE logs SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'service, level',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Auto-compress chunks older than 7 days
SELECT add_compression_policy('logs', INTERVAL '7 days');

-- Data retention: keep logs for 30 days (can be adjusted per environment)
SELECT add_retention_policy('logs', INTERVAL '30 days');
