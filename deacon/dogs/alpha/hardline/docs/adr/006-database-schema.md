# ADR-006: Database Schema - SQLite with WAL Durability

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline needs persistent storage for:

1. **Workspace state** - Track all workspaces, their state, paths, metadata
2. **Operation journal** - Step-by-step records for durable execution
3. **Queue entries** - Merge queue with priority ordering
4. **Agent registry** - Track active agents, heartbeats, capabilities
5. **Configuration** - User preferences, repository settings

**Requirements:**
- **ACID durability** - No data loss on crash
- **Concurrent access** - 600+ agents accessing simultaneously
- **SQLite with WAL** - Battle-tested, performant, portable
- **No schema migrations during runtime** - Schema versioning for upgrades

This ADR defines the complete database schema.

---

## Decision

### Database Configuration

```rust
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub wal_mode: bool,           // Always true
    pub foreign_keys: bool,       // Always true
    pub synchronous: Synchronous, // FULL for durability
    pub busy_timeout: Duration,   // 5 seconds
    pub max_connections: u32,    // 1 for SQLite
}

pub enum Synchronous {
    OFF,   // Risky - no durability
    NORMAL, // Balanced (default)
    FULL,  // Maximum durability, slower
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from(".hardline/state.db"),
            wal_mode: true,
            foreign_keys: true,
            synchronous: Synchronous::FULL,  // CRITICAL for durability
            busy_timeout: Duration::from_secs(5),
            max_connections: 1,  // SQLite limitation
        }
    }
}
```

### Schema Version Table (Migration Tracking)

```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL,
    description TEXT NOT NULL
);

-- Initial schema
INSERT INTO schema_version (version, applied_at, description)
VALUES (1, datetime('now'), 'Initial schema');
```

### Workspace Table

```sql
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    backend TEXT NOT NULL CHECK (backend IN ('git')),
    state TEXT NOT NULL CHECK (state IN (
        'created', 'active', 'syncing', 'paused', 'completed', 'failed'
    )),
    agent_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed_at TEXT NOT NULL,
    
    -- Indexes for common queries
    UNIQUE(name),  -- redundant with UNIQUE but explicit
    INDEX idx_workspaces_state (state),
    INDEX idx_workspaces_agent (agent_id)
);

CREATE TRIGGER workspaces_updated_at
AFTER UPDATE ON workspaces
BEGIN
    UPDATE workspaces SET updated_at = datetime('now') WHERE id = NEW.id;
END;
```

### Operation Journal Table (Durable Execution)

```sql
CREATE TABLE operations (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'started', 'in_progress', 'completed', 'failed'
    )),
    current_step INTEGER NOT NULL DEFAULT 0,
    total_steps INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    final_revision INTEGER,
    error_message TEXT,
    author_id TEXT NOT NULL,
    description TEXT NOT NULL,
    
    INDEX idx_operations_workspace (workspace_id),
    INDEX idx_operations_state (state),
    INDEX idx_operations_started (started_at)
);

CREATE TABLE operation_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL REFERENCES operations(id),
    step_index INTEGER NOT NULL,
    step_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'pending', 'running', 'completed', 'failed', 'skipped'
    )),
    event_revision INTEGER,
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    
    UNIQUE(operation_id, step_index),
    INDEX idx_steps_operation (operation_id)
);
```

### Queue Entry Table (Merge Queue)

```sql
CREATE TABLE queue_entries (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    priority INTEGER NOT NULL CHECK (priority >= 0 AND priority <= 255),
    status TEXT NOT NULL CHECK (status IN (
        'pending', 'claimed', 'rebase', 'testing', 'ready_to_merge',
        'merging', 'merged', 'failed_retryable', 'failed_terminal', 'cancelled'
    )),
    enqueued_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    claimed_by TEXT,
    claimed_at TEXT,
    position INTEGER NOT NULL,
    
    -- Priority queue ordering: smallest priority first, then oldest first
    INDEX idx_queue_priority (priority ASC, enqueued_at ASC),
    INDEX idx_queue_status (status),
    INDEX idx_queue_claimed (claimed_by)
);

CREATE TRIGGER queue_updated_at
AFTER UPDATE ON queue_entries
BEGIN
    UPDATE queue_entries SET updated_at = datetime('now') WHERE id = NEW.id;
END;
```

### Agent Registry Table

```sql
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    capabilities TEXT NOT NULL,  -- JSON array of capabilities
    status TEXT NOT NULL CHECK (status IN ('active', 'idle', 'disconnected')),
    last_heartbeat_at TEXT NOT NULL,
    registered_at TEXT NOT NULL,
    metadata TEXT,  -- JSON for agent-specific data
    
    INDEX idx_agents_status (status),
    INDEX idx_agents_heartbeat (last_heartbeat_at)
);
```

### Session Table

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'created', 'active', 'syncing', 'synced', 'paused', 'completed', 'failed'
    )),
    bead_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    
    UNIQUE(workspace_id, name),
    INDEX idx_sessions_workspace (workspace_id),
    INDEX idx_sessions_bead (bead_id),
    INDEX idx_sessions_state (state)
);
```

### Configuration Table

```sql
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    description TEXT
);

CREATE TRIGGER config_updated_at
AFTER UPDATE ON config
BEGIN
    UPDATE config SET updated_at = datetime('now') WHERE key = NEW.key;
END;
```

---

## Variants

### Variant A: Single Database File (CHOSEN)

```sql
-- All tables in one .db file
-- .db-wal and .db-shm for WAL mode
```

**Pros:**
- Simple backup (single file)
- ACID across all data
- WAL provides concurrency

**Cons:**
- Single point of failure (mitigated by WAL + backups)
- Schema changes affect entire DB

### Variant B: Multiple Database Files

```sql
-- workspaces.db, queue.db, agents.db, sessions.db
```

**Pros:**
- Isolation between domains
- Parallel access

**Cons:**
- Cross-domain transactions not possible
- More complex backup management
- SQLite doesn't handle cross-file JOINs well

**Rejected because:** Single file is simpler and cross-domain consistency matters for hardline.

### Variant C: Embedded KV Store (Rejected)

```rust
// Use rocksdb or sled instead of SQLite
```

**Rejected because:**
- SQLite is battle-tested and simpler
- SQL is familiar for queries
- Built-in WAL durability
- No external dependencies

---

## Invariants

### Schema Version Invariants

```rust
/// INVARIANT: Schema version exactly one row
assert!(query("SELECT COUNT(*) FROM schema_version") == 1);

/// INVARIANT: Schema version is monotonically increasing
let versions = query("SELECT version FROM schema_version ORDER BY version");
assert!(versions.is_sorted());
```

### Workspace Table Invariants

```rust
/// INVARIANT: No duplicate workspace names
assert!(query(
    "SELECT name, COUNT(*) as cnt FROM workspaces GROUP BY name HAVING cnt > 1"
).is_empty());

/// INVARIANT: No circular workspace references
// Workspace has no self-referential FKs, so this is automatically satisfied

/// INVARIANT: State transitions are valid (enforced by CHECK constraint)
let valid_states = ["created", "active", "syncing", "paused", "completed", "failed"];
assert!(workspaces.iter().all(|w| valid_states.contains(&w.state)));
```

### Operation Journal Invariants

```rust
/// INVARIANT: Steps are contiguous for each operation
for operation in operations {
    let steps = operation.steps().sorted_by_key(|s| s.step_index);
    for (i, step) in steps.enumerate() {
        assert_eq!(step.step_index, i as i64);
    }
}

/// INVARIANT: Completed operations have completed_at set
for op in operations.filter(|op| op.state == "completed") {
    assert!(op.completed_at.is_some());
}

/// INVARIANT: Failed operations have error_message set
for op in operations.filter(|op| op.state == "failed") {
    assert!(op.error_message.is_some());
}

/// INVARIANT: current_step <= total_steps
for op in operations {
    assert!(op.current_step <= op.total_steps);
}
```

### Queue Entry Invariants

```rust
/// INVARIANT: Priority is within valid range
assert!(queue_entries.iter().all(|e| e.priority >= 0 && e.priority <= 255));

/// INVARIANT: Position is unique per priority level
for (priority, group) in queue_entries.group_by(|e| e.priority) {
    assert!(group.iter().all_unique_by(|e| &e.position));
}

/// INVARIANT: Claimed entries have claimed_by and claimed_at set
for entry in queue_entries.filter(|e| e.status == "claimed") {
    assert!(entry.claimed_by.is_some());
    assert!(entry.claimed_at.is_some());
}

/// INVARIANT: Merged entries have no active claims
for entry in queue_entries.filter(|e| e.status == "merged") {
    assert!(entry.claimed_by.is_none());
}
```

### Agent Registry Invariants

```rust
/// INVARIANT: No duplicate agent IDs
assert!(query(
    "SELECT id, COUNT(*) as cnt FROM agents GROUP BY id HAVING cnt > 1"
).is_empty());

/// INVARIANT: Heartbeat timeout is enforced in application code
// Agents with last_heartbeat_at > now() - HEARTBEAT_TIMEOUT should be marked disconnected
const HEARTBEAT_TIMEOUT_SECONDS: i64 = 300; // 5 minutes
```

### Session Table Invariants

```rust
/// INVARIANT: Session name is unique within workspace
for (workspace_id, group) in sessions.group_by(|s| &s.workspace_id) {
    assert!(group.iter().all_unique_by(|s| &s.name));
}

/// INVARIANT: Completed sessions have completed_at set
for session in sessions.filter(|s| s.state == "completed") {
    assert!(session.completed_at.is_some());
}
```

### Cross-Table Invariants

```rust
/// INVARIANT: All workspace_ids in operations exist in workspaces
for op in operations {
    assert!(workspaces.iter().any(|w| w.id == op.workspace_id));
}

/// INVARIANT: All workspace_ids in queue_entries exist in workspaces
for entry in queue_entries {
    assert!(workspaces.iter().any(|w| w.id == entry.workspace_id));
}

/// INVARIANT: Orphaned sessions are cleaned up
// Sessions with workspace_id not in workspaces should be flagged for cleanup
```

---

## Consequences

### Positive

1. **Durability** - WAL + FULL sync = no data loss on crash
2. **Consistency** - Foreign keys prevent orphaned records
3. **Performance** - Indexes on all lookup columns
4. **Simplicity** - Single file, easy backup
5. **Portability** - SQLite works everywhere

### Negative

1. **Single writer** - SQLite limitation, but OK for hardline's usage
2. **Schema migrations** - Need versioning strategy
3. **No concurrent writes** - But read concurrency is fine

### Migration Strategy

```rust
pub fn migrate(db: &Connection, from_version: u32, to_version: u32) -> Result<()> {
    match (from_version, to_version) {
        (1, 2) => migrate_v1_to_v2(db)?,
        _ => return Err(MigrationError::UnsupportedVersion(from_version, to_version)),
    }
    Ok(())
}
```

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/core/src/infrastructure/database.rs` | Database connection, migrations |
| `crates/core/src/infrastructure/schema.rs` | Schema definitions |
| `crates/workspace/src/infrastructure/workspace_repository.rs` | Workspace persistence |
| `crates/queue/src/infrastructure/queue_repository.rs` | Queue persistence |

---

## Related ADRs

- ADR-002: Durable Workflow Execution (uses operations + steps tables)
- ADR-008: Queue Processing (uses queue_entries table)
- ADR-005: Workspace Isolation Model (uses workspaces table)
