---
bead_id: hl-1p0
bead_title: Port Session Lock Manager (TTL/Heartbeat)
phase: contract
updated_at: 2026-03-30T12:00:00Z
---

# Contract: Session Lock Manager (TTL/Heartbeat) — hl-1p0

## Status: PARTIAL PORT — Implementation exists, tests broken

## 1. Gap Analysis

### Already Ported (implementation code in `crates/core/src/coordination/locks/`)
- `LockManager` struct with `db: SqlitePool`, `ttl: Duration`
- `lock()` — acquire with default TTL
- `lock_with_ttl()` — acquire with custom TTL, input validation, session existence check
- `unlock()` — holder-only release with double-unlock detection
- `heartbeat()` — TTL extension for lock holder
- `get_all_locks()` — all active locks query
- `get_lock_state()` — per-session lock state
- `get_lock_audit_log()` — audit trail query
- `verify_session_exists()` — session existence guard
- `log_operation()` — audit trail logging
- `is_constraint_conflict_error()` — SQLite race detection
- Full error taxonomy: `LockError(LockErrorKind::...)` with 12 variants
- Type-safe `Ttl` value object with validation
- `LockOperation` enum (Lock, Unlock, Heartbeat, DoubleUnlockWarning)

### Missing/Broken
- **4 failing tests** — ported from isolate using flat error matching (`Error::SessionLocked { ... }`) instead of hardline's layered `Error::Lock(LockError(LockErrorKind::SessionLocked { ... }))`

## 2. Error Mapping (Isolate → Hardline)

| Isolate Error | Hardline Error |
|---|---|
| `Error::SessionLocked { session, holder }` | `Error::Lock(LockError(LockErrorKind::SessionLocked { session, holder }))` |
| `Error::SessionNotFound { session }` | `Error::Lock(LockError(LockErrorKind::SessionNotFound { session }))` |
| `Error::NotLockHolder { session, agent_id }` | `Error::Lock(LockError(LockErrorKind::NotLockHolder { session, agent_id }))` |
| `Error::NotFound(msg)` | `Error::Lock(LockError(LockErrorKind::NotFound(msg)))` |
| `Error::DatabaseError(msg)` | `Error::Lock(LockError(LockErrorKind::DatabaseError(msg)))` |
| `Error::IoError(msg)` | `Error::Io(IoError(IoErrorKind::...))` |
| `Error::ParseError(msg)` | `Error::Lock(LockError(LockErrorKind::ParseError(msg)))` |

## 3. Invariants

1. **Mutual Exclusion**: At most one agent holds a lock on a session at any time
2. **TTL Enforcement**: Expired locks are invisible to queries (filtered by `expires_at >= now`)
3. **Session Validation**: Lock acquisition fails with `SessionNotFound` if session doesn't exist in `sessions` table (graceful degradation if table missing)
4. **Double-Unlock Detection**: Second unlock by same agent logs `DoubleUnlockWarning` audit entry, returns `Ok(())`
5. **Holder-Only Release**: Only the agent holding the lock can unlock or heartbeat it
6. **Idempotent Lock**: Re-locking by same agent returns existing lock info without error
7. **Audit Completeness**: Every lock, unlock, heartbeat, and double-unlock is logged to `session_lock_audit`

## 4. Public API Surface

```rust
impl LockManager {
    pub fn new(db: SqlitePool) -> Self;
    pub fn with_ttl(db: SqlitePool, ttl: Duration) -> Self;
    pub const fn pool(&self) -> &SqlitePool;
    pub async fn init(&self) -> Result<()>;
    pub async fn lock(&self, session: &str, agent_id: &str) -> Result<LockResponse>;
    pub async fn lock_with_ttl(&self, session: &str, agent_id: &str, ttl_seconds: u64) -> Result<LockResponse>;
    pub async fn unlock(&self, session: &str, agent_id: &str) -> Result<()>;
    pub async fn heartbeat(&self, session: &str, agent_id: &str) -> Result<LockResponse>;
    pub async fn get_all_locks(&self) -> Result<Vec<LockInfo>>;
    pub async fn get_lock_state(&self, session: &str) -> Result<LockState>;
    pub async fn get_lock_audit_log(&self, session: &str) -> Result<Vec<LockAuditEntry>>;
}
```

## 5. Preconditions

- `lock()`, `lock_with_ttl()`: session non-empty, len ≤ 255; agent_id non-empty; ttl ≤ 86400
- `unlock()`, `heartbeat()`: session must exist (not validated, but lock must be held)
- `init()` must be called before any operation

## 6. Postconditions

- `lock()`/`lock_with_ttl()`: Returns `LockResponse` with unique `lock_id`, or `SessionLocked`/`SessionNotFound`/validation error
- `unlock()`: Returns `Ok(())` on success or double-unlock, `NotLockHolder` if wrong agent
- `heartbeat()`: Returns `LockResponse` with extended `expires_at`, or `NotLockHolder`/`NotFound`
- `get_all_locks()`: Only returns non-expired locks, sorted by `expires_at ASC`

## 7. Scope of This Bead

Fix the 4 failing tests to use correct hardline error patterns. No implementation changes needed — the port is functionally complete.

### Failing Tests
1. `tests_concurrent::regression_concurrent_lock_mutual_exclusion` — matches `Error::Session(_)` instead of `Error::Lock(LockError(LockErrorKind::SessionLocked {...}))`
2. `tests_ttl_regression::regression_lock_with_ttl_maps_contention_race_to_session_locked` — same pattern
3. `tests_session_validation::lock_nonexistent_session_returns_not_found_error` — matches `Error::Session(_)` instead of `Error::Lock(LockError(LockErrorKind::SessionNotFound {...}))`
4. `tests_session_validation::lock_deleted_session_fails_with_not_found` — same pattern
