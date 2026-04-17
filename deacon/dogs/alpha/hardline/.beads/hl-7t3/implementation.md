# Implementation Summary: Database Corruption Resilience (hl-7t3)

## Findings
The Red Queen discovered that deleting the database file allowed subsequent `release` operations to incorrectly succeed. This was due to:
1. **Lenient Validation**: `verify_session_exists` silently ignored "missing table" errors, assuming graceful degradation was appropriate for uninitialized states.
2. **Auto-Initialization**: Every CLI subcommand (release, status, etc.) called `mgr.init()`, which re-created missing tables in a fresh SQLite file (created by `mode=rwc`).

## Fixes
1. **Strict Verification**: Refactored `verify_session_exists` in `crates/core/src/coordination/locks/manager.rs` to treat missing tables as hard `DatabaseError`s.
2. **Explicit Initialization**: Removed `mgr.init()` from `release`, `heartbeat`, `status`, and `list` handlers in `crates/cli/src/commands/lock.rs`. Database initialization is now strictly limited to the `acquire` command.
3. **Transaction Safety**: (Inherited from hl-4yx) All state-dependent operations are performed within atomic transactions.

## Verification
- **Corruption Test**: Verified that `release` fails with exit code 63 (`no such table`) when the database file is deleted/corrupted.
- **Initialization Test**: Confirmed that `acquire` correctly initializes the schema when the database file is missing or empty (provided the foundational `sessions` table exists).
- **Quality Gates**: All 19 lock-related tests pass.
