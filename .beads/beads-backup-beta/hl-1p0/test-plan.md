---
bead_id: hl-1p0
bead_title: Port Session Lock Manager (TTL/Heartbeat)
phase: test-plan
updated_at: 2026-03-30T12:00:00Z
---

# Test Plan: hl-1p0 Session Lock Manager

## Scope
Fix 4 failing tests that use flat error matching instead of hardline's layered `Error::Lock(LockError(LockErrorKind::...))` pattern.

## Testing Trophy Allocation
- **Unit (70%)**: Error matching pattern fixes in existing tests
- **Integration (20%)**: Concurrent lock tests (already exist, just fixing patterns)
- **Property (10%)**: N/A for this fix — proptest not needed for error pattern alignment

## BDD Scenarios

### Scenario 1: Concurrent lock contention returns LockError
**Given** 10 agents try to lock the same session simultaneously
**When** 9 fail and 1 succeeds
**Then** all failures match `Error::Lock(LockError(LockErrorKind::SessionLocked { .. }))`
**Files**: `tests_concurrent.rs:regression_concurrent_lock_mutual_exclusion`

### Scenario 2: Lock_with_ttl contention maps to SessionLocked
**Given** 10 agents try lock_with_ttl on same session
**When** contention detected
**Then** all failures are `Error::Lock(LockErrorKind::SessionLocked)` not `Error::Io(_)` or raw database errors
**Files**: `tests_ttl_regression.rs:regression_lock_with_ttl_maps_contention_race_to_session_locked`

### Scenario 3: Non-existent session returns LockError::SessionNotFound
**Given** sessions table exists but no session "ghost-session"
**When** `lock("ghost-session", "agent-1")` called
**Then** error matches `Error::Lock(LockError(LockErrorKind::SessionNotFound { session }))`
**Files**: `tests_session_validation.rs:lock_nonexistent_session_returns_not_found_error`

### Scenario 4: Deleted session returns LockError::SessionNotFound
**Given** session was created then deleted
**When** `lock("ephemeral-session", "agent-1")` called
**Then** error matches `Error::Lock(LockError(LockErrorKind::SessionNotFound { session }))`
**Files**: `tests_session_validation.rs:lock_deleted_session_fails_with_not_found`

## Pattern Fix

Old: `matches!(r, Err(Error::SessionLocked { .. }))`
New (hardline): `matches!(r, Err(Error::Lock(LockError(LockErrorKind::SessionLocked { .. }))))`

Old: `matches!(result, Err(Error::SessionNotFound { .. }))`
New: `matches!(result, Err(Error::Lock(LockError(LockErrorKind::SessionNotFound { .. }))))`

Old: `matches!(r, Err(Error::DatabaseError(_)))`
New: `matches!(r, Err(Error::Lock(LockError(LockErrorKind::DatabaseError(_))))`

## No Kani/Proptest Needed
This is a pattern alignment fix, not algorithmic logic. Kani and proptest add no value here.

## Verification Gate
```bash
cargo test -p scp-core --lib coordination::locks
```
Expected: 22 passed, 0 failed
