-- ===== CONTINUOUS AGGREGATES (Materialized Views) =====
-- These auto-update as new data arrives and enable fast analytics queries

-- 1. Daily Active Users (DAU)
CREATE MATERIALIZED VIEW analytics_daily_active_users
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', timestamp) AS day,
    COUNT(DISTINCT user_id) AS active_users,
    COUNT(*) AS total_events
FROM analytics_events
WHERE user_id IS NOT NULL
GROUP BY day
WITH NO DATA;

-- Refresh policy: update every hour for last 3 days
SELECT add_continuous_aggregate_policy(
    'analytics_daily_active_users',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour'
);

-- 2. Task Statistics (daily rollup)
CREATE MATERIALIZED VIEW analytics_task_stats_daily
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', timestamp) AS day,
    COUNT(*) FILTER (WHERE event_type = 'task_created') AS created,
    COUNT(*) FILTER (WHERE event_type = 'task_started') AS started,
    COUNT(*) FILTER (WHERE event_type = 'task_completed') AS completed,
    COUNT(*) FILTER (WHERE event_type = 'task_failed') AS failed,
    COUNT(*) FILTER (WHERE event_type = 'task_cancelled') AS cancelled,
    -- Average duration for completed tasks (stored in data->>'duration_ms')
    AVG((data->>'duration_ms')::BIGINT) FILTER (WHERE event_type = 'task_completed') AS avg_duration_ms,
    -- P95 duration
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY (data->>'duration_ms')::BIGINT)
        FILTER (WHERE event_type = 'task_completed') AS p95_duration_ms
FROM analytics_events
WHERE event_type IN ('task_created', 'task_started', 'task_completed', 'task_failed', 'task_cancelled')
GROUP BY day
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'analytics_task_stats_daily',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour'
);

-- 3. API Endpoint Latency (hourly)
CREATE MATERIALIZED VIEW analytics_api_latency_hourly
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', timestamp) AS hour,
    service,
    data->>'endpoint' AS endpoint,
    data->>'method' AS method,
    COUNT(*) AS request_count,
    AVG((data->>'duration_ms')::BIGINT) AS avg_duration_ms,
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY (data->>'duration_ms')::BIGINT) AS p50_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY (data->>'duration_ms')::BIGINT) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY (data->>'duration_ms')::BIGINT) AS p99_duration_ms,
    COUNT(*) FILTER (WHERE (data->>'status_code')::INTEGER >= 500) AS error_5xx_count,
    COUNT(*) FILTER (WHERE (data->>'status_code')::INTEGER >= 400 AND (data->>'status_code')::INTEGER < 500) AS error_4xx_count
FROM analytics_events
WHERE event_type = 'api_request'
GROUP BY hour, service, endpoint, method
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'analytics_api_latency_hourly',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour'
);

-- 4. Integration Health (daily)
CREATE MATERIALIZED VIEW analytics_integration_health_daily
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', timestamp) AS day,
    data->>'provider' AS provider,
    COUNT(*) FILTER (WHERE event_type = 'integration_connected') AS connections,
    COUNT(*) FILTER (WHERE event_type = 'integration_disconnected') AS disconnections,
    COUNT(*) FILTER (WHERE event_type = 'integration_used') AS uses,
    COUNT(*) FILTER (WHERE event_type = 'integration_error') AS errors,
    COUNT(DISTINCT user_id) AS unique_users
FROM analytics_events
WHERE event_type IN ('integration_connected', 'integration_disconnected', 'integration_used', 'integration_error')
GROUP BY day, provider
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'analytics_integration_health_daily',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour'
);

-- 5. Authentication Events (daily)
CREATE MATERIALIZED VIEW analytics_auth_events_daily
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', timestamp) AS day,
    COUNT(*) FILTER (WHERE event_type = 'auth_login_attempt') AS login_attempts,
    COUNT(*) FILTER (WHERE event_type = 'auth_login_attempt' AND (data->>'success')::BOOLEAN = true) AS successful_logins,
    COUNT(*) FILTER (WHERE event_type = 'auth_login_attempt' AND (data->>'success')::BOOLEAN = false) AS failed_logins,
    COUNT(*) FILTER (WHERE event_type = 'auth_code_verified') AS code_verifications,
    COUNT(*) FILTER (WHERE event_type = 'auth_token_refresh') AS token_refreshes,
    COUNT(DISTINCT user_id) AS unique_users_authenticated
FROM analytics_events
WHERE event_type IN ('auth_login_attempt', 'auth_code_verified', 'auth_token_refresh')
GROUP BY day
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'analytics_auth_events_daily',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour'
);

-- 6. Cocoon Activity (daily)
CREATE MATERIALIZED VIEW analytics_cocoon_activity_daily
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', timestamp) AS day,
    COUNT(*) FILTER (WHERE event_type = 'cocoon_registered') AS registrations,
    COUNT(*) FILTER (WHERE event_type = 'cocoon_connected') AS connections,
    COUNT(*) FILTER (WHERE event_type = 'cocoon_disconnected') AS disconnections,
    COUNT(*) FILTER (WHERE event_type = 'cocoon_claimed') AS claims,
    -- Average connection duration in seconds
    AVG((data->>'duration_seconds')::BIGINT) FILTER (WHERE event_type = 'cocoon_disconnected') AS avg_session_duration_seconds,
    COUNT(DISTINCT data->>'cocoon_id') AS unique_cocoons
FROM analytics_events
WHERE event_type IN ('cocoon_registered', 'cocoon_connected', 'cocoon_disconnected', 'cocoon_claimed')
GROUP BY day
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'analytics_cocoon_activity_daily',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour'
);

-- 7. Error Tracking (hourly)
CREATE MATERIALIZED VIEW analytics_errors_hourly
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', timestamp) AS hour,
    service,
    data->>'error_type' AS error_type,
    COUNT(*) AS error_count,
    COUNT(DISTINCT user_id) AS affected_users,
    -- Sample error message (first occurrence)
    (array_agg(data->>'error_message' ORDER BY timestamp))[1] AS sample_error_message
FROM analytics_events
WHERE event_type = 'application_error'
GROUP BY hour, service, error_type
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'analytics_errors_hourly',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour'
);

-- ===== INDEXES ON AGGREGATES =====
-- For faster dashboard queries

CREATE INDEX idx_dau_day ON analytics_daily_active_users(day DESC);
CREATE INDEX idx_task_stats_day ON analytics_task_stats_daily(day DESC);
CREATE INDEX idx_api_latency_hour ON analytics_api_latency_hourly(hour DESC, service, endpoint);
CREATE INDEX idx_integration_health_day ON analytics_integration_health_daily(day DESC, provider);
CREATE INDEX idx_auth_events_day ON analytics_auth_events_daily(day DESC);
CREATE INDEX idx_cocoon_activity_day ON analytics_cocoon_activity_daily(day DESC);
CREATE INDEX idx_errors_hour ON analytics_errors_hourly(hour DESC, service, error_type);

-- ===== MANUAL REFRESH (Initial Population) =====
-- Run these once to populate aggregates with existing data
-- After this, they auto-update via policies

-- REFRESH MATERIALIZED VIEW analytics_daily_active_users;
-- REFRESH MATERIALIZED VIEW analytics_task_stats_daily;
-- REFRESH MATERIALIZED VIEW analytics_api_latency_hourly;
-- REFRESH MATERIALIZED VIEW analytics_integration_health_daily;
-- REFRESH MATERIALIZED VIEW analytics_auth_events_daily;
-- REFRESH MATERIALIZED VIEW analytics_cocoon_activity_daily;
-- REFRESH MATERIALIZED VIEW analytics_errors_hourly;
