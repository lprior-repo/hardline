-- Isolate Session Locks Table Schema (DRQ Round 4: Concurrency Control)
--
-- This table implements distributed locking for multi-agent scenarios.
-- Multiple AI agents can coordinate access to sessions through these locks.
--
-- Lock Lifecycle:
-- 1. Agent calls acquire_lock() -> INSERT with expires_at
-- 2. Operations check is_locked() -> SELECT non-expired locks
-- 3. Agent calls release_lock() -> DELETE lock
-- 4. Expired locks auto-cleanup on next query

CREATE TABLE IF NOT EXISTS session_locks (
    -- Session being locked
    session_name TEXT NOT NULL,

    -- Operation type (e.g., 'sync', 'remove', 'modify', 'spawn')
    operation TEXT NOT NULL,

    -- Optional agent identifier for debugging
    agent_id TEXT,

    -- Lock acquisition timestamp
    acquired_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- Lock expiration timestamp (TTL-based auto-release)
    expires_at INTEGER NOT NULL,

    -- Composite primary key: one lock per session+operation
    PRIMARY KEY (session_name, operation)
);

-- Index for expired lock cleanup queries
-- Used by: is_locked(), get_active_operations(), cleanup_expired_locks()
CREATE INDEX IF NOT EXISTS idx_session_locks_expires_at ON session_locks(expires_at);

-- Index for agent-based lock queries
-- Used by: operations-in-progress query (show locks by agent)
CREATE INDEX IF NOT EXISTS idx_session_locks_agent_id ON session_locks(agent_id);

-- Index for acquisition ordering
-- Used by: get_active_operations() (show oldest locks first)
CREATE INDEX IF NOT EXISTS idx_session_locks_acquired_at ON session_locks(acquired_at);
