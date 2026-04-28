-- Isolate Database Triggers
--
-- Automatic timestamp management for sessions table

-- Trigger: Auto-update updated_at on row modification
--
-- This trigger ensures the updated_at column is automatically refreshed
-- whenever a session row is modified, without requiring manual updates.
CREATE TRIGGER IF NOT EXISTS update_sessions_timestamp
    AFTER UPDATE ON sessions
    FOR EACH ROW
    BEGIN
        UPDATE sessions
        SET updated_at = strftime('%s', 'now')
        WHERE id = NEW.id;
    END;
