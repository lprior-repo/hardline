-- Isolate Sessions Table Schema
--
-- This table stores all session information for isolate workspace management.
-- Each session represents a JJ workspace with associated Zellij session.

CREATE TABLE IF NOT EXISTS sessions (
    -- Primary key
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Session identifier (unique human-readable name)
    name TEXT UNIQUE NOT NULL,

    -- Session lifecycle status
    -- Valid values: 'creating', 'active', 'paused', 'completed', 'failed'
    status TEXT NOT NULL CHECK(status IN ('creating', 'active', 'paused', 'completed', 'failed')),

    -- Workspace lifecycle state (tracks work progress)
    -- Valid values: 'created', 'working', 'ready', 'merged', 'abandoned', 'conflict'
    state TEXT NOT NULL DEFAULT 'created'
        CHECK(state IN ('created', 'working', 'ready', 'merged', 'abandoned', 'conflict')),

    -- File system path to the JJ workspace
    workspace_path TEXT NOT NULL,

    -- JJ branch name (optional)
    branch TEXT,

    -- Timestamps (Unix epoch seconds)
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_synced INTEGER,

    -- JSON metadata for extensible session properties
    metadata TEXT
);

-- Workspace state transition history
CREATE TABLE IF NOT EXISTS state_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    reason TEXT NOT NULL,
    agent_id TEXT,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Index for status-based filtering
-- Used by: list command, dashboard, clean operations
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

-- Index for state-based filtering
CREATE INDEX IF NOT EXISTS idx_sessions_state ON sessions(state);

-- Index for name-based lookups
-- Used by: get, update, delete operations
CREATE INDEX IF NOT EXISTS idx_sessions_name ON sessions(name);

-- Index for created_at ordering
-- Used by: list command (default ordering)
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);

-- Indexes for transition history
CREATE INDEX IF NOT EXISTS idx_state_transitions_session ON state_transitions(session_id);
CREATE INDEX IF NOT EXISTS idx_state_transitions_timestamp ON state_transitions(timestamp);
