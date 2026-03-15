-- Initialize databases for TimescaleDB services
-- This script runs on first timescaledb container startup

-- Create logging database
SELECT 'CREATE DATABASE adi_logging'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'adi_logging')\gexec

-- Grant privileges
GRANT ALL PRIVILEGES ON DATABASE adi_logging TO adi;

-- Connect to logging database and create schema
\c adi_logging

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Logs table (centralized log storage)
CREATE TABLE IF NOT EXISTS logs (
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
    trace_id UUID NOT NULL,
    span_id UUID NOT NULL,
    parent_span_id UUID,
    
    -- Structured data
    fields JSONB DEFAULT '{}',
    
    -- Error details (for error/fatal logs)
    error_kind VARCHAR(255),
    error_message TEXT,
    error_stack_trace TEXT,
    
    -- Source location
    source VARCHAR(255),
    target VARCHAR(255),
    
    PRIMARY KEY (timestamp, id)
);

-- Convert to hypertable (time-series partitioning by day)
SELECT create_hypertable('logs', 'timestamp', chunk_time_interval => INTERVAL '1 day', if_not_exists => TRUE);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_logs_trace_id ON logs(trace_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_logs_span_id ON logs(span_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_logs_service ON logs(service, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_logs_level ON logs(level, timestamp DESC) WHERE level >= 4;
CREATE INDEX IF NOT EXISTS idx_logs_service_level ON logs(service, level, timestamp DESC);

-- Enable compression after 7 days
ALTER TABLE logs SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'service, level',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Auto-compress chunks older than 7 days (ignore if policy exists)
DO $$
BEGIN
    PERFORM add_compression_policy('logs', INTERVAL '7 days', if_not_exists => TRUE);
EXCEPTION WHEN OTHERS THEN
    NULL;
END $$;

-- Data retention: keep logs for 30 days
DO $$
BEGIN
    PERFORM add_retention_policy('logs', INTERVAL '30 days', if_not_exists => TRUE);
EXCEPTION WHEN OTHERS THEN
    NULL;
END $$;
