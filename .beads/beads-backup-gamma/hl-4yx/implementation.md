# Implementation Summary: Critical Race Condition Fix (hl-4yx)

## Findings
The Red Queen discovered non-deterministic behavior during concurrent lock acquisitions. Investigation revealed:
1. **Database Contention**: High concurrency (100+ agents) caused frequent `database is locked` (SQLITE_BUSY) errors, which were mapped to exit code 63 (DatabaseError) instead of being handled as contention.
2. **Error Delegation Failure**: The root `Error::exit_code` implementation was hardcoded to return 90 for all `Error::Lock` variants, masking specific error codes like 16 (SessionLocked).
3. **Atomicity Risk**: While protected by a UNIQUE constraint, the "check-then-insert" logic was performed in multiple transactions, increasing the risk of state drift if non-DB operations (like logging) failed.

## Fixes
1. **Busy Timeout**: Added `busy_timeout(5000)` to `SqliteDatabaseService` configuration in `crates/core/src/infrastructure/database.rs`. This allows SQLite to wait up to 5 seconds for a lock to clear instead of failing immediately.
2. **Error Delegation**: Updated `crates/core/src/error.rs` to correctly delegate `exit_code()` to the inner `LockError`.
3. **Atomic Transactions**: Refactored `LockManager` (`lock_with_ttl`, `unlock`, `heartbeat`) to use explicit transactions, ensuring that session verification, expired lock cleanup, and state transitions are all atomic.

## Verification
- **Race Test**: Simulated 100 concurrent acquisition attempts. 
    - **Result**: Exactly 1 success (0) and 99 `SessionLocked` (16) results. Zero other errors.
- **Unit/Integration Tests**: All 19 existing lock tests pass.
- **Manual QA**: Verified global `--database` flag functionality under load.
