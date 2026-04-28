-- Event Store Lock Schema (Design Constraint 1)
--
-- This table implements distributed locking for the event store,
-- ensuring ordered event processing across multiple agents.
--
-- The composite UNIQUE constraint on (stream_id, stream_seq) ensures
-- that each event has a unique position within its stream, preventing
-- duplicate or out-of-order event processing.
--
-- Lock Lifecycle:
-- 1. Agent calls acquire_stream_lock() -> INSERT with expires_at
-- 2. Event processing checks is_stream_locked() -> SELECT non-expired locks
-- 3. Agent calls release_stream_lock() -> DELETE lock
-- 4. Expired locks auto-cleanup on next query
--
-- Design Rationale:
-- - stream_id: Identifies the event stream (e.g., "session-queue", "workspace-events")
-- - stream_seq: Monotonically increasing sequence number within the stream
-- - The composite UNIQUE prevents duplicate sequence assignments

CREATE TABLE IF NOT EXISTS event_store_locks (
    -- Stream identifier (e.g., session name, workspace ID)
    stream_id TEXT NOT NULL,

    -- Sequence number within the stream (enforces ordering)
    stream_seq INTEGER NOT NULL,

    -- Agent holding the lock
    holder_id TEXT NOT NULL,

    -- Lock acquisition timestamp
    acquired_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- Lock expiration timestamp (TTL-based auto-release)
    expires_at INTEGER NOT NULL,

    -- Composite unique constraint: one lock per stream+seq combination
    -- This ensures exclusive access to process specific events
    UNIQUE(stream_id, stream_seq)
);

-- Index for stream-based lock queries
-- Used by: is_stream_locked(), get_stream_locks()
CREATE INDEX IF NOT EXISTS idx_event_store_locks_stream_id ON event_store_locks(stream_id);

-- Index for expired lock cleanup queries
-- Used by: cleanup_expired_stream_locks()
CREATE INDEX IF NOT EXISTS idx_event_store_locks_expires_at ON event_store_locks(expires_at);

-- Index for holder-based lock queries
-- Used by: locks_by_holder()
CREATE INDEX IF NOT EXISTS idx_event_store_locks_holder_id ON event_store_locks(holder_id);

-- Index for sequence-based ordering (within stream)
-- Used by: get_next_sequence(), event replay
CREATE INDEX IF NOT EXISTS idx_event_store_locks_stream_seq ON event_store_locks(stream_id, stream_seq);
