-- Migration: 001_queue_entries
-- Description: Create queue_entries table for merge queue
-- Target: SQLite
-- Bead: scp-09f

-- Create queue_entries table with all required columns, constraints, and indexes
CREATE TABLE IF NOT EXISTS queue_entries (
    -- Primary key
    id TEXT PRIMARY KEY,
    
    -- Required fields
    session_id TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 128,
    position INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL,
    enqueued_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    
    -- Optional fields
    bead_id TEXT,
    error_message TEXT,
    
    -- Constraints
    CHECK (status IN (
        'Pending',
        'Claimed',
        'Rebasing',
        'Testing',
        'ReadyToMerge',
        'Merging',
        'Merged',
        'FailedRetryable',
        'FailedTerminal',
        'Cancelled'
    )),
    CHECK (priority >= 0 AND priority <= 255),
    CHECK (retry_count >= 0)
);

-- Index for dequeue operations (status + priority + position)
CREATE INDEX IF NOT EXISTS idx_queue_entries_status_priority_position
ON queue_entries(status, priority, position);

-- Index for session lookups
CREATE INDEX IF NOT EXISTS idx_queue_entries_session_id
ON queue_entries(session_id);
