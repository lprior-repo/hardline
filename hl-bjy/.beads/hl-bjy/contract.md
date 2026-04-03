bead_id: hl-bjy
bead_title: Port Session Locks: TTL/Heartbeat Implementation
phase: state-1-contract
updated_at: 2026-03-25T23:30:00Z

# Contract Specification: Session Lock Manager

## Context

- **Feature**: Port session lock manager with TTL and heartbeat from legacy codebase to hardline
- **Domain Terms**:
  - **SessionLock**: An exclusive lock on a session held by an agent
  - **LockId**: Unique identifier for a lock instance
  - **Session**: A named session resource that can be locked
  - **AgentId**: Identifier of the agent holding a lock
  - **TTL**: Time-to-live in seconds determining lock expiration
  - **LockManager**: Core service managing lock lifecycle
  - **Heartbeat**: Mechanism to extend lock TTL while agent is active
  - **LockAuditEntry**: Audit trail record for lock operations

- **Assumptions**:
  - SQLite database with `session_locks` and `session_lock_audit` tables
  - Sessions table exists for validation (backward compatible if missing)
  - Locks have a default TTL of 300 seconds (5 minutes)
  - Lock IDs are generated using format: `lock-{session}-{nanos}`
  - Timestamps use RFC3339 format for SQLite compatibility
  - Multiple agents can contend for locks; only one holds at a time

- **Open Questions**:
  1. Should `heartbeat()` return `LockResponse` or `Result<(), Error>`? (Source returns `LockResponse`)
  2. Should `lock_with_ttl()` accept `u64` seconds or `Duration`? (Source uses `u64`)
  3. What is the exact error code for `SessionLocked`? (Source uses `"SESSION_LOCKED"`)
  4. Should `unlock()` silently succeed on double-unlock or return a specific error? (Source logs warning and succeeds)

## Domain Types

```rust
/// Unique identifier for a lock instance
pub struct LockId(String);

/// Session name (validated identifier)
pub struct Session(String);

/// Agent identifier
pub struct AgentId(String);

/// Time-to-live in seconds
pub struct Ttl(u64);

/// Lock information returned to caller
#[derive(Debug, Clone)]
pub struct LockResponse {
    pub lock_id: LockId,
    pub session: Session,
    pub agent_id: AgentId,
    pub expires_at: DateTime<Utc>,
}

/// Lock info for querying all active locks
#[derive(Debug, Clone)]
pub struct LockInfo {
    pub session: Session,
    pub agent_id: AgentId,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Current lock state for a session
#[derive(Debug, Clone)]
pub struct LockState {
    pub session: Session,
    pub holder: Option<AgentId>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Audit log entry for lock operations
#[derive(Debug, Clone)]
pub struct LockAuditEntry {
    pub session: Session,
    pub agent_id: AgentId,
    pub operation: LockOperation,
    pub timestamp: DateTime<Utc>,
}

/// Types of operations logged in audit trail
#[derive(Debug, Clone, PartialEq)]
pub enum LockOperation {
    Lock,
    Unlock,
    Heartbeat,
    DoubleUnlockWarning,
}
```

## Invariants

1. **Uniqueness**: At most one active (non-expired) lock exists per session at any time
2. **Expiration**: A lock is inactive if `expires_at < now()`
3. **Ownership**: Only the agent that acquired a lock can release it (unless expired)
4. **Session Existence**: A lock can only be acquired for a session that exists in the `sessions` table (when table exists)
5. **Audit Completeness**: Every lock operation (lock, unlock, heartbeat, double_unlock) is logged to `session_lock_audit`
6. **Lock ID Uniqueness**: Each lock_id is globally unique (collision handled via nanosecond timestamp)
7. **TTL Consistency**: All locks created via `LockManager` use either default TTL or explicitly provided TTL
8. **No Orphaned Locks**: Locks for non-existent sessions are never created

## Preconditions

### `lock_with_ttl(session, agent_id, ttl_seconds)`

- **Session Format**: `session` must be a non-empty string matching session naming conventions
- **Agent Format**: `agent_id` must be a non-empty string
- **TTL Validity**: `ttl_seconds` must be >= 0 (0 means use default TTL)
- **Database Connection**: Database pool must be valid and accessible
- **Session Existence**: Session must exist in `sessions` table (if table exists)

### `lock(session, agent_id)`

- **Session Format**: `session` must be a non-empty string
- **Agent Format**: `agent_id` must be a non-empty string
- **Database Connection**: Database pool must be valid and accessible
- **Session Existence**: Session must exist in `sessions` table (if table exists)

### `unlock(session, agent_id)`

- **Session Format**: `session` must be a non-empty string
- **Agent Format**: `agent_id` must be a non-empty string
- **Lock Existence**: Either a valid lock exists with this agent as holder, OR no lock exists (double-unlock case)

### `heartbeat(session, agent_id)`

- **Session Format**: `session` must be a non-empty string
- **Agent Format**: `agent_id` must be a non-empty string
- **Lock Holder**: Agent must currently hold the lock (valid, non-expired)

### `get_all_locks()`

- **Database Connection**: Database pool must be valid and accessible

### `get_lock_audit_log(session)`

- **Session Format**: `session` must be a non-empty string
- **Database Connection**: Database pool must be valid and accessible

### `get_lock_state(session)`

- **Session Format**: `session` must be a non-empty string
- **Database Connection**: Database pool must be valid and accessible

## Postconditions

### `lock_with_ttl(session, agent_id, ttl_seconds) -> Result<LockResponse, Error>`

**Success**:
- A new lock entry is inserted into `session_locks` with:
  - `lock_id` = generated unique ID
  - `session` = provided session
  - `agent_id` = provided agent_id
  - `acquired_at` = current UTC timestamp
  - `expires_at` = current time + TTL
- Audit entry logged with operation="lock"
- Returns `LockResponse` with all lock details

**Failure modes**:
- `SessionLocked`: Another agent holds a valid lock for this session
- `SessionNotFound`: Session does not exist in `sessions` table
- `DatabaseError`: Database operation failed

### `lock(session, agent_id) -> Result<LockResponse, Error>`

**Success**:
- A new lock entry is inserted into `session_locks` with default TTL
- Audit entry logged with operation="lock"
- Returns `LockResponse` with all lock details

**Failure modes**:
- `SessionLocked`: Another agent holds a valid lock for this session
- `SessionNotFound`: Session does not exist in `sessions` table
- `DatabaseError`: Database operation failed

### `unlock(session, agent_id) -> Result<(), Error>`

**Success cases**:
- Lock was held by agent: Lock is deleted, audit entry logged with operation="unlock"
- No lock exists: No-op, audit entry logged with operation="double_unlock_warning"

**Failure modes**:
- `NotLockHolder`: Lock exists but is held by a different agent

### `heartbeat(session, agent_id) -> Result<LockResponse, Error>`

**Success**:
- `expires_at` is extended to current time + TTL
- Returns `LockResponse` with updated expiration

**Failure modes**:
- `NotLockHolder`: Lock exists but is held by a different agent
- `NotFound`: No active lock exists for this session

### `get_all_locks() -> Result<Vec<LockInfo>, Error>`

**Success**:
- Returns all non-expired locks from `session_locks`
- Each `LockInfo` includes session, agent_id, acquired_at, expires_at

### `get_lock_audit_log(session) -> Result<Vec<LockAuditEntry>, Error>`

**Success**:
- Returns all audit entries for session, ordered by id ASC
- Each entry includes session, agent_id, operation, timestamp

### `get_lock_state(session) -> Result<LockState, Error>`

**Success**:
- Returns current lock state for session
- If locked: `holder = Some(agent_id)`, `expires_at = Some(timestamp)`
- If unlocked: `holder = None`, `expires_at = None`

## Error Taxonomy

```rust
#[derive(Debug)]
pub enum Error {
    /// Session does not exist in sessions table
    /// Occurs when attempting to lock a non-existent session
    SessionNotFound {
        session: String,
    },

    /// Session is already locked by another agent
    /// Occurs when attempting to lock an already-held session
    SessionLocked {
        session: String,
        holder: String,
    },

    /// Agent attempted to unlock a lock they do not hold
    /// Occurs when non-holder calls unlock()
    NotLockHolder {
        session: String,
        agent_id: String,
    },

    /// No active lock exists for the session
    /// Occurs when heartbeat() is called but no valid lock exists
    NotFound(String),

    /// Database operation failed
    DatabaseError(String),

    /// Failed to parse timestamp or other format conversion
    ParseError(String),

    /// Unknown/unexpected error
    Unknown(String),
}

impl Error {
    /// Returns error code for external identification
    pub fn code(&self) -> &'static str {
        match self {
            Error::SessionNotFound { .. } => "SESSION_NOT_FOUND",
            Error::SessionLocked { .. } => "SESSION_LOCKED",
            Error::NotLockHolder { .. } => "NOT_LOCK_HOLDER",
            Error::NotFound(_) => "NOT_FOUND",
            Error::DatabaseError(_) => "DATABASE_ERROR",
            Error::ParseError(_) => "PARSE_ERROR",
            Error::Unknown(_) => "UNKNOWN",
        }
    }
}
```

## Contract Signatures

```rust
pub struct LockManager {
    db: SqlitePool,
    ttl: Duration,
}

impl LockManager {
    /// Create a new LockManager with default TTL (300s)
    pub const fn new(db: SqlitePool) -> Self;

    /// Create a new LockManager with custom TTL
    pub const fn with_ttl(db: SqlitePool, ttl: Duration) -> Self;

    /// Get the database pool reference
    pub const fn pool(&self) -> &SqlitePool;

    /// Initialize lock tables
    pub async fn init(&self) -> Result<()>;

    /// Acquire an exclusive lock on a session with custom TTL
    ///
    /// Returns SessionLocked if another agent holds a valid lock
    /// Returns SessionNotFound if session doesn't exist
    pub async fn lock_with_ttl(
        &self,
        session: &str,
        agent_id: &str,
        ttl_seconds: u64,
    ) -> Result<LockResponse, Error>;

    /// Acquire an exclusive lock on a session (default TTL)
    ///
    /// Returns SessionLocked if another agent holds a valid lock
    /// Returns SessionNotFound if session doesn't exist
    pub async fn lock(
        &self,
        session: &str,
        agent_id: &str,
    ) -> Result<LockResponse, Error>;

    /// Release a lock (only holder can release)
    ///
    /// Logs double-unlock warning if called when no lock exists
    pub async fn unlock(
        &self,
        session: &str,
        agent_id: &str,
    ) -> Result<(), Error>;

    /// Extend a lock's TTL (heartbeat)
    ///
    /// Only the lock holder can heartbeat
    pub async fn heartbeat(
        &self,
        session: &str,
        agent_id: &str,
    ) -> Result<LockResponse, Error>;

    /// Get all active (non-expired) locks
    pub async fn get_all_locks(&self) -> Result<Vec<LockInfo>, Error>;

    /// Get audit log for a session
    pub async fn get_lock_audit_log(&self, session: &str) -> Result<Vec<LockAuditEntry>, Error>;

    /// Get current lock state for a session
    pub async fn get_lock_state(&self, session: &str) -> Result<LockState, Error>;
}

/// Helper function to detect constraint conflict errors in SQLite
fn is_constraint_conflict_error(error: &sqlx::Error) -> bool;
```

## Schema Contract

### `session_locks` table

```sql
CREATE TABLE IF NOT EXISTS session_locks (
    lock_id TEXT PRIMARY KEY,
    session TEXT NOT NULL UNIQUE,
    agent_id TEXT NOT NULL,
    acquired_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
```

**Indexes**:
- `lock_id` - PRIMARY KEY
- `session` - UNIQUE (enforces one lock per session)
- `expires_at` - Implicit index for expiration queries

### `session_lock_audit` table

```sql
CREATE TABLE IF NOT EXISTS session_lock_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    timestamp TEXT NOT NULL
);
```

**Indexes**:
- `id` - PRIMARY KEY
- `session` - Implicit index for audit log queries

## Non-goals

- **Lock ownership transfer**: A lock cannot be transferred from one agent to another
- **Lock priority**: All locks are equal; first-come-first-served
- **Lock queuing**: No waiting queue; contention results in immediate `SessionLocked` error
- **Session lifecycle management**: Lock manager does not create/delete sessions
- **Lock metrics/monitoring**: No built-in metrics collection (audit log is for debugging)
- **Distributed coordination**: Lock manager is SQLite-backed; not suitable for multi-node coordination without additional infrastructure

## Implementation Notes

1. **Fail-fast contention**: Check for existing lock BEFORE session validation to exit quickly on contention
2. **Idempotent re-lock**: Same agent re-locking returns existing lock info without creating duplicate
3. **Constraint race handling**: Convert SQLite constraint violations to `SessionLocked` for stable API behavior
4. **Backward compatibility**: If `sessions` table doesn't exist, allow lock acquisition (legacy databases)
5. **Cleanup on failure**: If audit log insertion fails after successful lock, delete the lock to maintain consistency
6. **Double-unlock detection**: Log warning instead of error to allow graceful cleanup

## Architecture Principles

- **Railway-Oriented Programming**: All fallible operations use `Result<T, Error>`
- **Type Safety**: Domain types (`Session`, `AgentId`, `LockId`) prevent primitive obsession
- **Immutability**: Domain types are immutable; only `LockManager` state changes
- **Audit Trail**: All operations logged for debugging and forensics
- **Session Validation**: Prevent orphaned locks by validating session existence first
