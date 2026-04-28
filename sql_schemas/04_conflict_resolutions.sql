-- Isolate Conflict Resolutions Table Schema
--
-- This table provides an append-only audit trail for tracking conflict
-- resolution decisions in isolate workspace management.
--
-- Design Principles:
-- 1. Append-Only: No UPDATE or DELETE operations allowed
-- 2. AI vs Human Tracking: Every resolution records the decider type
-- 3. Transparency: Full audit trail for debugging and accountability
-- 4. Performance: Optimized for inserts and queries with indexes

CREATE TABLE IF NOT EXISTS conflict_resolutions (
    -- Primary key (auto-increment)
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- ISO 8601 timestamp of resolution
    timestamp TEXT NOT NULL,

    -- Session name where conflict occurred
    session TEXT NOT NULL,

    -- File path with conflict
    file TEXT NOT NULL,

    -- Resolution strategy used
    -- Examples: "accept_theirs", "accept_ours", "manual_merge", "skip"
    strategy TEXT NOT NULL,

    -- Human-readable reason for resolution (optional)
    reason TEXT,

    -- Confidence score for AI decisions (optional)
    -- Examples: "high", "medium", "low", "0.95"
    confidence TEXT,

    -- Decider type: 'ai' or 'human'
    decider TEXT NOT NULL CHECK(decider IN ('ai', 'human'))
);

-- Index for session-based queries
-- Used by: get_conflict_resolutions()
CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session
ON conflict_resolutions(session);

-- Index for time-based queries
-- Used by: get_resolutions_by_time_range()
CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_timestamp
ON conflict_resolutions(timestamp);

-- Index for decider-based queries
-- Used by: get_resolutions_by_decider()
CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_decider
ON conflict_resolutions(decider);

-- Composite index for session+time queries (common pattern)
CREATE INDEX IF NOT EXISTS idx_conflict_resolutions_session_timestamp
ON conflict_resolutions(session, timestamp);
