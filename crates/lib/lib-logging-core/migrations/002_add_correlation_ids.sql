-- Add correlation ID columns for business-level log correlation
-- These enable querying all logs related to a specific cocoon, user, or session

-- Correlation ID columns (nullable - not all logs have all IDs)
ALTER TABLE logs ADD COLUMN IF NOT EXISTS cocoon_id VARCHAR(64);
ALTER TABLE logs ADD COLUMN IF NOT EXISTS user_id VARCHAR(64);
ALTER TABLE logs ADD COLUMN IF NOT EXISTS session_id VARCHAR(64);
ALTER TABLE logs ADD COLUMN IF NOT EXISTS hive_id VARCHAR(64);

-- Index for cocoon-based queries (find all logs for a cocoon)
CREATE INDEX IF NOT EXISTS idx_logs_cocoon_id
    ON logs(cocoon_id, timestamp DESC)
    WHERE cocoon_id IS NOT NULL;

-- Index for user-based queries (find all logs for a user)
CREATE INDEX IF NOT EXISTS idx_logs_user_id
    ON logs(user_id, timestamp DESC)
    WHERE user_id IS NOT NULL;

-- Index for session-based queries (find all logs for a session)
CREATE INDEX IF NOT EXISTS idx_logs_session_id
    ON logs(session_id, timestamp DESC)
    WHERE session_id IS NOT NULL;

-- Index for hive-based queries (find all logs for a hive)
CREATE INDEX IF NOT EXISTS idx_logs_hive_id
    ON logs(hive_id, timestamp DESC)
    WHERE hive_id IS NOT NULL;

-- Composite index for common query patterns (user's cocoons)
CREATE INDEX IF NOT EXISTS idx_logs_user_cocoon
    ON logs(user_id, cocoon_id, timestamp DESC)
    WHERE user_id IS NOT NULL AND cocoon_id IS NOT NULL;

COMMENT ON COLUMN logs.cocoon_id IS 'Cocoon device ID for correlation across all cocoon-related logs';
COMMENT ON COLUMN logs.user_id IS 'User ID for correlation across all user activity logs';
COMMENT ON COLUMN logs.session_id IS 'Session ID for correlation within a WebSocket/WebRTC session';
COMMENT ON COLUMN logs.hive_id IS 'Hive ID for correlation across hive orchestration logs';
