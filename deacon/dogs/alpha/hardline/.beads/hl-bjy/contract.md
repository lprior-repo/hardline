bead_id: hl-bjy
bead_title: Port Session Locks: TTL/Heartbeat Implementation
phase: state-1-contract
updated_at: 2026-03-26T14:30:00Z

---

# Contract Specification: Session Lock Manager

## Context

- **Feature**: TTL/Heartbeat implementation for session-based lock management
- **Domain terms**:
  - `Session`: A named entity in the `sessions` table representing a user/client
  - `Lock`: A mutex-like guard preventing concurrent operations on a session
  - `TTL`: Time-to-live duration after which a lock expires automatically
  - `Heartbeat`: Mechanism to extend a lock's TTL while active
  - `Audit Log`: Immutable record of all lock operations for forensics
  - `Lock Manager`: Domain service coordinating lock lifecycle
- **Assumptions**:
  - SQLite database with `sessions` table (managed externally)
  - `session_locks` and `session_lock_audit` tables created by `init()`
  - All timestamps use UTC `DateTime<Utc>` from `chrono`
  - Lock IDs are deterministic: `lock-{session}-{timestamp_nanos}`
- **Open questions**:
  - Should `verify_session_exists()` be a public API or internal helper? **Answer: Public API for external validation before locking**
  - What is the maximum session name length? **Answer: 255 chars (SQLite TEXT limit)**
  - What TTL range is valid? **Answer: 0 (default 300s) to 86400s (24 hours)**

---

## Preconditions

- [ ] `LockManager::new()` / `LockManager::with_ttl()`: `SqlitePool` must be valid and connectionable
- [ ] `LockManager::init()`: Database must be writable; no schema migrations running concurrently
- [ ] `lock_with_ttl(session, agent, ttl)`: 
  - `session` must be non-empty string (1-255 chars)
  - `agent_id` must be non-empty string
  - `ttl_seconds` must be in range `[0, 86400]` (0 uses default 300s)
  - `session.len() <= 255` (SQLite TEXT limit)
  - `ttl_seconds != u64::MAX` (overflow check)
- [ ] `lock(session, agent)`: Same preconditions as `lock_with_ttl` with `ttl=0` (uses default)
- [ ] `unlock(session, agent)`: `session` and `agent_id` must be non-empty strings
- [ ] `heartbeat(session, agent)`: Same preconditions as `unlock`
- [ ] `get_all_locks()`: No preconditions (safe to call on any database state)
- [ ] `get_lock_audit_log(session)`: `session` must be non-empty string
- [ ] `get_lock_state(session)`: Same precondition as `get_lock_audit_log`
- [ ] `verify_session_exists(session)`: `session` must be non-empty string

---

## Postconditions

- [ ] `LockManager::new()`: Returns `LockManager` with `ttl = Duration::seconds(300)`, `db` set to input pool
- [ ] `LockManager::with_ttl(db, ttl)`: Returns `LockManager` with `ttl` set to input duration, `db` set to input pool
- [ ] `LockManager::init()`: 
  - Creates `session_locks` table if not exists with schema: `lock_id PRIMARY KEY, session, agent_id, acquired_at, expires_at`
  - Creates `session_lock_audit` table if not exists with schema: `id PRIMARY KEY AUTOINCREMENT, session, agent_id, operation, timestamp`
  - Returns `Ok(())` on success
  - Idempotent: calling multiple times succeeds without error
- [ ] `LockManager::pool()`: Returns `&SqlitePool` reference to internal pool
- [ ] `lock_with_ttl(session, agent, ttl)`:
  - On success: Creates lock record, inserts audit entry, returns `LockResponse` with generated `lock_id`
  - On SessionLocked: Returns `SessionLocked { session, holder }` with actual holder's agent_id
  - On SessionNotFound: Returns `SessionNotFound { session }`
  - On TtlOutOfRange: Returns `TtlOutOfRange("TTL must be in range [0, 86400]")` when `ttl_seconds > 86400`
  - On EmptySessionName: Returns `EmptySessionName("Session name cannot be empty")` when `session == ""`
  - On EmptyAgentId: Returns `EmptyAgentId("Agent ID cannot be empty")` when `agent_id == ""`
  - On TtlOverflow: Returns `TtlOverflow("TTL overflow detected")` when `ttl_seconds == u64::MAX`
  - On SessionNameTooLong: Returns `SessionNameTooLong("Session name cannot exceed 255 characters")` when `session.len() > 255`
  - Re-acquire by same agent: Returns existing `LockResponse` without regeneration
  - Expired lock cleanup: Deletes expired lock before creating new one
- [ ] `lock(session, agent)`: Same postconditions as `lock_with_ttl` with `ttl=0` (uses default 300s)
- [ ] `unlock(session, agent)`:
  - On success: Deletes lock record, inserts audit entry, returns `Ok(())`
  - On NotLockHolder: Returns `NotLockHolder { session, agent_id }` without modification
  - Double-unlock: Returns `Ok(())` with `double_unlock_warning` audit entry
- [ ] `heartbeat(session, agent)`:
  - On success: Updates `expires_at = current_time + default_ttl`, inserts audit entry, returns `LockResponse`
  - On NotLockHolder: Returns `NotLockHolder { session, agent_id }`
  - On NotFound (no lock or expired): Returns `NotFound("No active lock for session '{session}'")`
- [ ] `get_all_locks()`: Returns `Vec<LockInfo>` with only active locks (expires_at > now), sorted by expires_at ASC
- [ ] `get_lock_audit_log(session)`: Returns `Vec<LockAuditEntry>` ordered by timestamp ASC, empty Vec if no history
- [ ] `get_lock_state(session)`: Returns `LockState` with `holder` and `expires_at` as `Option` (None if no active lock)
- [ ] `verify_session_exists(session)`:
  - On success: Returns `Ok(())` if session exists in `sessions` table
  - On SessionNotFound: Returns `SessionNotFound { session }` if session missing
  - Graceful degradation: Returns `Ok(())` if `sessions` table does not exist (legacy DB compatibility)

---

## Invariants

- [ ] **Lock uniqueness per session**: At most one active lock exists per session at any time
- [ ] **TTL consistency**: For any lock, `expires_at > acquired_at` and `expires_at - acquired_at == TTL` (or default TTL)
- [ ] **Audit completeness**: Every successful lock/unlock/heartbeat operation creates exactly one audit entry
- [ ] **Ownership enforcement**: Only lock holder can call `heartbeat()` or `unlock()` on their lock
- [ ] **Lock ID uniqueness**: Generated lock_ids are unique within test run; format: `lock-{session}-{nanos}`
- [ ] **Session validation**: Lock cannot be acquired for non-existent session (prevents orphaned locks)
- [ ] **Init idempotency**: `CREATE TABLE IF NOT EXISTS` ensures `init()` is safe to call multiple times
- [ ] **No orphaned locks**: Transaction rollback on audit insert failure prevents lock records without audit entries
- [ ] **Expired lock exclusion**: `get_all_locks()` never returns expired locks; locks are not auto-deleted but filtered

---

## Error Taxonomy

All fallible functions return `Result<T, Error>` where `Error` is defined as:

```rust
#[derive(Debug)]
pub enum Error {
    /// Session does not exist in sessions table
    /// Used when lock/heartbeat/unlock is attempted on non-existent session
    SessionNotFound { session: String },
    
    /// Session is already locked by another agent
    /// Contains the holder's agent_id for client-side retry logic
    SessionLocked { session: String, holder: String },
    
    /// Agent attempted operation on lock held by different agent
    /// Used for unlock() and heartbeat() when agent_id != holder
    NotLockHolder { session: String, agent_id: String },
    
    /// No active lock exists for session
    /// Used for heartbeat() when lock is missing or expired
    NotFound(String),
    
    /// Database operation failed (connection, query, transaction)
    /// Wraps sqlx::Error with context
    DatabaseError(String),
    
    /// Failed to parse timestamp or other format
    /// Used for RFC3339 timestamp parsing failures
    ParseError(String),
    
    /// Unknown/unexpected error with context
    /// Catch-all for errors not covered by specific variants
    Unknown(String),
    
    /// TTL value outside valid range [0, 86400]
    /// Used when ttl_seconds parameter is < 0 or > 86400 (24 hours)
    TtlOutOfRange(String),
    
    /// Session name is empty string
    /// Used when session parameter is ""
    EmptySessionName(String),
    
    /// Agent ID is empty string
    /// Used when agent_id parameter is ""
    EmptyAgentId(String),
    
    /// TTL value would overflow u64::MAX
    /// Used when ttl_seconds = u64::MAX or would overflow on arithmetic
    TtlOverflow(String),
    
    /// Session name exceeds 255 character limit
    /// Used when session.len() > 255 (SQLite TEXT limit)
    SessionNameTooLong(String),
}
```

### Error Code Mapping (for telemetry/monitoring)

- `Error::SessionNotFound` → `"SESSION_NOT_FOUND"`
- `Error::SessionLocked` → `"SESSION_LOCKED"`
- `Error::NotLockHolder` → `"NOT_LOCK_HOLDER"`
- `Error::NotFound` → `"NOT_FOUND"`
- `Error::DatabaseError` → `"DATABASE_ERROR"`
- `Error::ParseError` → `"PARSE_ERROR"`
- `Error::Unknown` → `"UNKNOWN"`
- `Error::TtlOutOfRange` → `"TTL_OUT_OF_RANGE"`
- `Error::EmptySessionName` → `"EMPTY_SESSION_NAME"`
- `Error::EmptyAgentId` → `"EMPTY_AGENT_ID"`
- `Error::TtlOverflow` → `"TTL_OVERFLOW"`
- `Error::SessionNameTooLong` → `"SESSION_NAME_TOO_LONG"`

### Error Construction Rules

- `SessionNotFound`: Constructed when `SELECT session FROM sessions WHERE session = ?` returns no rows
- `SessionLocked`: Constructed when constraint conflict detected (UNIQUE violation on `session` column) OR when `SELECT lock_id FROM session_locks WHERE session = ? AND expires_at > now()` returns different agent_id
- `NotLockHolder`: Constructed when `agent_id` parameter does not match `lock.agent_id` from database
- `NotFound`: Constructed when `SELECT lock_id FROM session_locks WHERE session = ? AND expires_at > now()` returns no rows
- `DatabaseError`: Constructed by wrapping sqlx::Error with context message
- `ParseError`: Constructed when `DateTime::parse_from_rfc3339()` fails
- `Unknown`: Constructed for unexpected error paths (constraint conflict without lock record, etc.)

---

## Contract Signatures

### Domain Operations (Core API)

```rust
/// Acquire a lock with explicit TTL
/// 
/// Preconditions:
/// - session: non-empty string, 1-255 characters
/// - agent_id: non-empty string
/// - ttl_seconds: 0 to 86400 (0 uses default 300s, max 24 hours)
/// - session.len() <= 255 (SQLite TEXT limit)
/// - ttl_seconds != u64::MAX (overflow prevention)
/// 
/// Returns:
/// - Ok(LockResponse) on success with generated lock_id
/// - Err(SessionLocked) if another agent holds valid lock
/// - Err(SessionNotFound) if session does not exist
/// - Err(TtlOutOfRange) if ttl_seconds > 86400
/// - Err(EmptySessionName) if session == ""
/// - Err(EmptyAgentId) if agent_id == ""
/// - Err(TtlOverflow) if ttl_seconds == u64::MAX
/// - Err(SessionNameTooLong) if session.len() > 255
/// - Err(DatabaseError) if database operation fails
pub fn lock_with_ttl(
    &self, 
    session: &str, 
    agent_id: &str, 
    ttl_seconds: u64
) -> Result<LockResponse, Error>;

/// Acquire a lock with default TTL (300s)
/// 
/// Wrapper around lock_with_ttl with ttl_seconds=0
pub fn lock(
    &self, 
    session: &str, 
    agent_id: &str
) -> Result<LockResponse, Error>;

/// Release a lock held by the caller
/// 
/// Preconditions:
/// - session: non-empty string
/// - agent_id: non-empty string
/// 
/// Returns:
/// - Ok(()) on success (idempotent: double-unlock returns Ok with warning)
/// - Err(NotLockHolder) if agent does not hold the lock
/// - Err(DatabaseError) if database operation fails
pub fn unlock(
    &self, 
    session: &str, 
    agent_id: &str
) -> Result<(), Error>;

/// Extend lock TTL via heartbeat (must be lock holder)
/// 
/// Preconditions:
/// - session: non-empty string
/// - agent_id: non-empty string
/// 
/// Returns:
/// - Ok(LockResponse) with extended expires_at
/// - Err(NotLockHolder) if agent does not hold the lock
/// - Err(NotFound) if no active lock exists
/// - Err(DatabaseError) if database operation fails
pub fn heartbeat(
    &self, 
    session: &str, 
    agent_id: &str
) -> Result<LockResponse, Error>;
```

### Query Operations (Read-Only API)

```rust
/// Get all active locks across all sessions
/// 
/// Returns:
/// - Ok(Vec<LockInfo>) with active locks sorted by expires_at ASC
/// - Empty Vec if no active locks exist
/// - Never returns expired locks
/// 
/// Note: Read-only, no database mutation
pub fn get_all_locks(&self) -> Result<Vec<LockInfo>, Error>;

/// Get audit log entries for a specific session
/// 
/// Preconditions:
/// - session: non-empty string
/// 
/// Returns:
/// - Ok(Vec<LockAuditEntry>) ordered by timestamp ASC
/// - Empty Vec if no audit history for session
pub fn get_lock_audit_log(
    &self, 
    session: &str
) -> Result<Vec<LockAuditEntry>, Error>;

/// Get current lock state for a session
/// 
/// Preconditions:
/// - session: non-empty string
/// 
/// Returns:
/// - Ok(LockState) with holder and expires_at as Options
/// - holder=None and expires_at=None if no active lock
pub fn get_lock_state(&self, session: &str) -> Result<LockState, Error>;

/// Verify session exists in sessions table
/// 
/// Used externally before attempting to acquire lock
/// 
/// Preconditions:
/// - session: non-empty string
/// 
/// Returns:
/// - Ok(()) if session exists in sessions table
/// - Err(SessionNotFound) if session missing
/// - Ok(()) if sessions table does not exist (graceful degradation)
/// 
/// Note: Optional public API for validation before lock_with_ttl()
pub fn verify_session_exists(&self, session: &str) -> Result<(), Error>;
```

### Infrastructure Operations

```rust
/// Initialize database schema (idempotent)
/// 
/// Creates session_locks and session_lock_audit tables if not exists
/// 
/// Returns:
/// - Ok(()) on success
/// - Err(DatabaseError) if table creation fails
pub fn init(&self) -> Result<(), Error>;

/// Get reference to internal SqlitePool
/// 
/// Used for transaction management and custom queries
pub fn pool(&self) -> &SqlitePool;
```

### Constructors

```rust
/// Construct LockManager with default TTL (300s)
/// 
/// Preconditions:
/// - db: valid and connectionable SqlitePool
pub fn new(db: SqlitePool) -> Self;

/// Construct LockManager with custom TTL
/// 
/// Preconditions:
/// - db: valid and connectionable SqlitePool
/// - ttl: Duration for default TTL value
pub fn with_ttl(db: SqlitePool, ttl: Duration) -> Self;
```

---

## Data Types

### LockResponse

```rust
#[derive(Debug, Clone)]
pub struct LockResponse {
    /// Generated unique lock identifier
    /// Format: "lock-{session}-{timestamp_nanos}"
    pub lock_id: String,
    
    /// Session name this lock protects
    pub session: String,
    
    /// Agent ID that holds this lock
    pub agent_id: String,
    
    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,
    
    /// When the lock expires (acquired_at + TTL)
    pub expires_at: DateTime<Utc>,
}
```

### LockInfo

```rust
#[derive(Debug, Clone)]
pub struct LockInfo {
    /// Session name
    pub session: String,
    
    /// Agent ID holding the lock
    pub agent_id: String,
    
    /// Lock identifier
    pub lock_id: String,
    
    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,
    
    /// When the lock expires
    pub expires_at: DateTime<Utc>,
}
```

### LockState

```rust
#[derive(Debug, Clone)]
pub struct LockState {
    /// Session name
    pub session: String,
    
    /// Agent ID holding the lock, None if no active lock
    pub holder: Option<String>,
    
    /// Lock expiration time, None if no active lock
    pub expires_at: Option<DateTime<Utc>>,
}
```

### LockAuditEntry

```rust
#[derive(Debug, Clone)]
pub struct LockAuditEntry {
    /// Session name
    pub session: String,
    
    /// Agent ID that performed the operation
    pub agent_id: String,
    
    /// Type of operation performed
    pub operation: LockOperation,
    
    /// When the operation occurred
    pub timestamp: DateTime<Utc>,
}
```

### LockOperation

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockOperation {
    /// Lock acquired
    Lock,
    
    /// Lock released by holder
    Unlock,
    
    /// Lock extended via heartbeat
    Heartbeat,
    
    /// Double-unlock warning (same agent unlocked twice)
    DoubleUnlockWarning,
}
```

---

## Non-goals

- [ ] Automatic lock cleanup of expired locks (only filtered from queries, not deleted)
- [ ] Distributed locking (SQLite-only, single-process)
- [ ] Lock priority or queuing (first-come-first-served via UNIQUE constraint)
- [ ] Lock inheritance or delegation (strict ownership via agent_id comparison)
- [ ] Transaction isolation beyond SQLite defaults (domain layer doesn't manage DB transactions)
- [ ] Session lifecycle management (sessions table managed externally)
- [ ] Metrics or monitoring instrumentation (domain layer only)
- [ ] Error recovery beyond retry-on-conflict pattern (caller handles SessionLocked)
