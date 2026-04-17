---
bead_id: hl-1p0
bead_title: Port Session Lock Manager (TTL/Heartbeat)
phase: implementation
updated_at: 2026-03-30T12:30:00Z
---

# Implementation Summary: hl-1p0

## Scope
Fix 3 failing test files (4 tests total) that used flat error matching instead of hardline's layered `Error::Lock(LockError(LockErrorKind::...))` pattern.

## Files Changed

### 1. `crates/core/src/coordination/locks/errors.rs`
- Added `kind()` accessor method to `LockError` struct
- Returns `&LockErrorKind` reference to enable pattern matching in tests

### 2. `crates/core/src/coordination/locks/tests_session_validation.rs`
- Added import: `use crate::coordination::locks::errors::LockErrorKind`
- Fixed `lock_nonexistent_session_returns_not_found_error`: `Error::Session(_)` → `Error::Lock(lk) if matches!(lk.kind(), LockErrorKind::SessionNotFound { .. })`
- Fixed `lock_deleted_session_fails_with_not_found`: same pattern fix

### 3. `crates/core/src/coordination/locks/tests_ttl_regression.rs`
- Added import: `use crate::coordination::locks::errors::LockErrorKind`
- Fixed `regression_lock_with_ttl_maps_contention_race_to_session_locked`: `Error::Session(_)` → `Error::Lock(lk)` with `LockErrorKind::SessionLocked`, `Error::Io(_)` → `Error::Lock(lk)` with `LockErrorKind::DatabaseError`

### 4. `crates/core/src/coordination/locks/tests_concurrent.rs`
- Added import: `use crate::coordination::locks::errors::LockErrorKind`
- Fixed `regression_concurrent_lock_mutual_exclusion`: `Error::Session(_)` → `Error::Lock(lk)` with `LockErrorKind::SessionLocked`

## Test Results
- 22 passed, 0 failed (all coordination::locks tests)
- No clippy warnings in changed files
- No implementation code changes — only test fixes and one accessor method

## Clause Mapping
| Contract Clause | Implementation |
|---|---|
| Error Mapping table | All 4 test files updated to use `Error::Lock(LockError(LockErrorKind::...))` |
| Invariant: Mutual Exclusion | Verified by concurrent tests (unchanged) |
| Invariant: Session Validation | Verified by session validation tests (pattern fixed) |
| Invariant: Audit Completeness | Verified by double_unlock test (unchanged) |
