# Test Plan: Port CLI Sync Command (hl-6u5)

## Summary
- **Behaviors identified**: 30
- **Trophy allocation**: 30% Unit / 60% Integration / 5% E2E / 5% Static
- **Proptest invariants**: 4
- **Fuzz targets**: 1
- **Kani harnesses**: 2
- **Mutation threshold**: 90%
- **Total Planned Tests**: 35 (28 BDD + 7 Unit/Boundary)

## 1. Behavior Inventory
1. `sync_named_session` rebases the session's revision onto the default target branch (`main`).
2. `sync_named_session` rebases the session's revision onto a custom `target_branch`.
3. `sync_all_sessions` identifies and syncs all eligible sessions (`Active` or `Failed`) in the database.
4. `sync_current_workspace` identifies the session associated with the current JJ workspace and syncs it.
5. Sync operations acquire a cross-process lock (`.jj/hardline/sync.lock`) before executing JJ commands.
6. Sync operations release the lock immediately upon completion, regardless of success or failure.
7. Sync operations update the `last_synced` timestamp in the database ONLY on successful rebase.
8. Sync operations transition session state to `Syncing` during the operation and `Synced` on success.
9. Sync operations emit JSONL `Action` records for lock acquisition and rebase start.
10. Sync operations emit JSONL `Issue` records for conflicts or JJ failures.
11. Sync operations retry transient JJ failures according to `RetryConfig`.
12. Sync operations fail with specific `SyncError` variants for all 17 identified failure modes.
13. Sync operations handle boundary conditions for timeouts, retry counts, and delays (e.g., 0 values).
14. Sync operations ensure zero panics by using `Result` for all IO and JJ interactions.

## 2. Trophy Allocation

| Layer | Weight | Rationale |
|-------|--------|-----------|
| **Static Analysis** | 5% | Enforce `deny.toml`, Clippy (pedantic), and `#![deny(clippy::unwrap_used)]`. |
| **Unit (Calc)** | 30% | Pure logic for retry delays, state transition validation, and JSONL formatting. |
| **Integration** | 60% | **Primary Layer.** Real JJ CLI calls, real filesystem locks, and real JSONL database I/O. |
| **E2E** | 5% | Smoke tests of the CLI entry point using `assert_cmd`. |

## 3. BDD Scenarios

### Behavior: Sync Named Session with Default Branch
**Scenario**: `fn sync_named_session_rebases_on_default_branch()`
- **Given**: A JJ repository with session "feat-x" at revision `R1`.
- **Given**: Session "feat-x" is `Active` in the database.
- **When**: `sync_named_session("feat-x", options_with_no_target_branch)` is called.
- **Then**: `jj rebase -r R1 -d main` is executed.
- **And**: Returns `Ok(SyncSummary { sessions_synced: ["feat-x"], total_operations: 1, had_conflicts: false })`.
- **And**: JSONL `Action` records for "Lock acquisition" and "Rebase start" are emitted.
- **And**: Database record for "feat-x" has `last_synced` updated to current time and state set to `Synced`.
- **And**: Sync lock file `.jj/hardline/sync.lock` is removed.

### Behavior: Sync Named Session with Custom Target Branch
**Scenario**: `fn sync_named_session_rebases_on_custom_target_branch()`
- **Given**: A JJ repository with session "feat-y" at revision `R2`.
- **Given**: Session "feat-y" is `Active` in the database.
- **When**: `sync_named_session("feat-y", SyncOptions { target_branch: Some("develop"), .. })` is called.
- **Then**: `jj rebase -r R2 -d develop` is executed.
- **And**: Returns `Ok(SyncSummary { sessions_synced: ["feat-y"], total_operations: 1, had_conflicts: false })`.
- **And**: JSONL `Action` records for "Lock acquisition" and "Rebase start" are emitted.
- **And**: Database record for "feat-y" has `last_synced` updated and state set to `Synced`.
- **And**: Sync lock file `.jj/hardline/sync.lock` is removed.

### Behavior: Sync All Sessions
**Scenario**: `fn sync_all_sessions_processes_multiple_eligible_sessions()`
- **Given**: A database with sessions "s1" (Active), "s2" (Failed), and "s3" (Synced).
- **When**: `sync_all_sessions(options)` is called.
- **Then**: It attempts to sync "s1" and "s2".
- **And**: Returns `Ok(SyncSummary { sessions_synced: ["s1", "s2"], total_operations: 2, had_conflicts: false })`.
- **And**: JSONL `Action` records for each session rebase are emitted.
- **And**: Both "s1" and "s2" in database have `last_synced` updated and state `Synced`.
- **And**: Sync lock file is removed.

### Behavior: Sync Current Workspace
**Scenario**: `fn sync_current_workspace_identifies_and_syncs_session()`
- **Given**: A JJ workspace associated with session "active-task".
- **Given**: Session "active-task" is `Active` in the database.
- **When**: `sync_current_workspace(options)` is called from within that workspace.
- **Then**: It correctly resolves the workspace to "active-task".
- **And**: `jj rebase` is executed for "active-task".
- **And**: Returns `Ok(SyncSummary { sessions_synced: ["active-task"], total_operations: 1, had_conflicts: false })`.
- **And**: JSONL `Action` records for "Lock acquisition" and "Rebase start" are emitted.
- **And**: Database record for "active-task" has `last_synced` updated and state set to `Synced`.

### Behavior: Lock Release on Failure (MANDATE: Ghost Test Resolved)
**Scenario**: `fn sync_lock_released_on_failure()`
- **Given**: A valid session "feat-fail".
- **Given**: `jj rebase` command is mocked to fail with exit code 1.
- **When**: `sync_named_session("feat-fail", ..)` is called.
- **Then**: Returns `Err(SyncError::JjCommandFailed(_))`.
- **And**: JSONL `Issue` record for the failure is emitted.
- **And**: Sync lock file `.jj/hardline/sync.lock` is ABSENT from the filesystem.
- **And**: Session "feat-fail" state in database remains/becomes `Failed` (not `Syncing`).

### Behavior: Transient Failure Retry (Sharpened)
**Scenario**: `fn sync_retries_on_transient_jj_error_and_succeeds()`
- **Given**: A JJ command that fails once with a transient error but succeeds on the second attempt.
- **When**: `sync_named_session` is called with `max_attempts: 3`.
- **Then**: Returns `Ok(SyncSummary { sessions_synced: ["feat-x"], total_operations: 2, had_conflicts: false })`.
- **And**: JSONL `Action` records for both attempts are emitted.
- **And**: JSONL `Issue` record for the first transient failure is emitted.
- **And**: The database record shows "feat-x" state as `Synced` and `last_synced` updated.

### Behavior: Sync Error Taxonomy (All 17 Variants)

#### 1. WorkspaceNotFound
**Scenario**: `fn sync_fails_with_workspace_not_found_when_outside_repo()`
- **Given**: Current directory is `/tmp` (not a JJ repo).
- **When**: `sync_current_workspace` is called.
- **Then**: Returns `Err(SyncError::WorkspaceNotFound(path))` where path is `/tmp`.
- **And**: JSONL `Issue` record is emitted.

#### 2. WorkspacePathNotAccessible
**Scenario**: `fn sync_fails_with_workspace_path_not_accessible_when_permissions_denied()`
- **Given**: A JJ workspace directory that is not readable (chmod 000).
- **When**: `sync_current_workspace` is called.
- **Then**: Returns `Err(SyncError::WorkspacePathNotAccessible(path))`.
- **And**: JSONL `Issue` record is emitted.

#### 3. SessionNotFound
**Scenario**: `fn sync_fails_with_session_not_found_when_name_is_missing()`
- **Given**: Database does NOT contain "ghost-session".
- **When**: `sync_named_session("ghost-session", ..)` is called.
- **Then**: Returns `Err(SyncError::SessionNotFound("ghost-session"))`.
- **And**: JSONL `Issue` record is emitted.

#### 4. SessionAlreadySyncing
**Scenario**: `fn sync_fails_with_session_already_syncing_when_state_is_syncing()`
- **Given**: Session "busy-bee" has state `Syncing` in the database.
- **When**: `sync_named_session("busy-bee", ..)` is called.
- **Then**: Returns `Err(SyncError::SessionAlreadySyncing("busy-bee"))`.
- **And**: JSONL `Issue` record is emitted.

#### 5. SessionTerminalState
**Scenario**: `fn sync_fails_with_session_terminal_state_when_already_synced()`
- **Given**: Session "done-deal" has state `Synced`.
- **When**: `sync_named_session("done-deal", ..)` is called.
- **Then**: Returns `Err(SyncError::SessionTerminalState("done-deal"))`.
- **And**: JSONL `Issue` record is emitted.

#### 6. LockAcquisitionFailed
**Scenario**: `fn sync_fails_with_lock_acquisition_failed_when_io_error_occurs()`
- **Given**: The `.jj/hardline` directory is read-only.
- **When**: `sync_named_session` is called.
- **Then**: Returns `Err(SyncError::LockAcquisitionFailed(msg))` where msg contains IO details.
- **And**: JSONL `Issue` record is emitted.

#### 7. LockHeldByOther
**Scenario**: `fn sync_fails_with_lock_held_by_other_when_another_process_has_lock()`
- **Given**: Lock file `.jj/hardline/sync.lock` exists with content `{"pid": 9999, "holder": "other-agent"}`.
- **When**: `sync_named_session` is called with `lock_timeout_secs: 0`.
- **Then**: Returns `Err(SyncError::LockHeldByOther { pid: 9999, holder: "other-agent" })`.
- **And**: JSONL `Issue` record is emitted.

#### 8. LockTimeout
**Scenario**: `fn sync_fails_with_lock_timeout_when_timeout_reached()`
- **Given**: Lock held by another process.
- **When**: `sync_named_session` is called with `lock_timeout_secs: 1`.
- **Then**: Returns `Err(SyncError::LockTimeout(1))`.
- **And**: JSONL `Issue` record is emitted.

#### 9. DirtyWorkspace
**Scenario**: `fn sync_fails_with_dirty_workspace_when_allow_dirty_is_false()`
- **Given**: JJ workspace has uncommitted changes.
- **When**: `sync_named_session("feat", SyncOptions { allow_dirty: false, .. })` is called.
- **Then**: Returns `Err(SyncError::DirtyWorkspace(path))`.
- **And**: JSONL `Issue` record is emitted.

#### 10. JjCommandFailed
**Scenario**: `fn sync_fails_with_jj_command_failed_when_jj_exits_with_error()`
- **Given**: `jj rebase` fails with "fatal: repository corruption".
- **When**: `sync_named_session` is called.
- **Then**: Returns `Err(SyncError::JjCommandFailed(msg))` containing the JJ stderr.
- **And**: JSONL `Issue` record is emitted.
- **And**: Sync lock is released.

#### 11. Conflict
**Scenario**: `fn sync_fails_with_conflict_when_rebase_has_conflicts()`
- **Given**: `jj rebase` results in merge conflicts in `src/main.rs`.
- **When**: `sync_named_session` is called.
- **Then**: Returns `Err(SyncError::Conflict { workspace: path, files: ["src/main.rs"] })`.
- **And**: JSONL `Issue` record is emitted.
- **And**: Sync lock is released.

#### 12. RetryLimitExceeded
**Scenario**: `fn sync_fails_with_retry_limit_exceeded_when_transient_errors_persist()`
- **Given**: `jj` command fails with transient error 3 times in a row.
- **When**: `sync_named_session` is called with `max_attempts: 3`.
- **Then**: Returns `Err(SyncError::RetryLimitExceeded(3))`.
- **And**: Multiple JSONL `Issue` records (one per attempt) are emitted.
- **And**: Sync lock is released.

#### 13. SessionDatabaseNotFound
**Scenario**: `fn sync_fails_with_session_database_not_found_when_file_missing()`
- **Given**: The session database file path does not exist.
- **When**: `sync_named_session` is called.
- **Then**: Returns `Err(SyncError::SessionDatabaseNotFound(path))`.
- **And**: JSONL `Issue` record is emitted.

#### 14. SessionDatabaseReadFailed
**Scenario**: `fn sync_fails_with_session_database_read_failed_when_corrupt_jsonl()`
- **Given**: The session database contains invalid non-JSON data.
- **When**: `sync_named_session` is called.
- **Then**: Returns `Err(SyncError::SessionDatabaseReadFailed(msg))`.
- **And**: JSONL `Issue` record is emitted.

#### 15. SessionDatabaseWriteFailed
**Scenario**: `fn sync_fails_with_session_database_write_failed_when_disk_full()`
- **Given**: Disk is full when attempting to update session state.
- **When**: `sync_named_session` is called.
- **Then**: Returns `Err(SyncError::SessionDatabaseWriteFailed(msg))`.
- **And**: JSONL `Issue` record is emitted.

#### 16. ConfigurationError
**Scenario**: `fn sync_fails_with_configuration_error_when_invalid_params_provided()`
- **Given**: `SyncOptions` with an invalid `target_branch` name (e.g., empty string).
- **When**: `sync_named_session` is called.
- **Then**: Returns `Err(SyncError::ConfigurationError(msg))`.
- **And**: JSONL `Issue` record is emitted.

#### 17. IoError
**Scenario**: `fn sync_fails_with_io_error_when_generic_io_failure_occurs()`
- **Given**: An unexpected IO error occurs during workspace traversal.
- **When**: `sync_current_workspace` is called.
- **Then**: Returns `Err(SyncError::IoError(..))`.
- **And**: JSONL `Issue` record is emitted.

### Behavior: Boundary Conditions
**Scenario**: `fn sync_fails_immediately_on_lock_contention_when_timeout_is_0()`
- **Given**: Lock is held by PID 1234.
- **When**: `sync_named_session` is called with `lock_timeout_secs: 0`.
- **Then**: Returns `Err(SyncError::LockHeldByOther { pid: 1234, .. })` instantly.
- **And**: JSONL `Issue` record is emitted.

**Scenario**: `fn sync_performs_no_retries_when_max_attempts_is_0()`
- **Given**: `jj` command fails once.
- **When**: `sync_named_session` is called with `max_attempts: 0`.
- **Then**: Returns `Err(SyncError::RetryLimitExceeded(0))` or the underlying `JjCommandFailed`.
- **And**: Sync lock is released.

**Scenario**: `fn sync_performs_exactly_one_attempt_when_max_attempts_is_1()`
- **Given**: `jj` command fails.
- **When**: `sync_named_session` is called with `max_attempts: 1`.
- **Then**: Returns `Err(SyncError::RetryLimitExceeded(1))` after exactly one call.
- **And**: JSONL `Issue` record for the single attempt is emitted.

**Scenario**: `fn sync_succeeds_with_zero_initial_delay()`
- **Given**: A valid session "fast-track".
- **When**: `sync_named_session` is called with `RetryConfig { initial_delay_ms: 0, .. }`.
- **Then**: It executes without artificial delay on the first attempt.
- **And**: Returns `Ok(SyncSummary { sessions_synced: ["fast-track"], .. })`.
- **And**: JSONL `Action` records and DB updates are verified.

## 4. Proptest Invariants

### Proptest: `calculate_retry_delay`
- **Invariant**: Delay always increases exponentially (up to a cap) and never exceeds a reasonable max.
- **Strategy**: `any(0..max_attempts)`, `any(initial_delay_ms)`.

### Proptest: `SessionState` transitions
- **Invariant**: Only `Active` or `Failed` sessions can transition to `Syncing`.
- **Strategy**: `any(SessionState)` as input to `can_start_sync()`.

### Proptest: JSONL Serialization
- **Invariant**: `deserialize(serialize(obj)) == obj`.
- **Strategy**: `any(SyncSummary)` or `any(Action)`.

### Proptest: `SyncOptions` Validation
- **Invariant**: `SyncOptions::validate()` returns `Ok` for any positive timeout and attempt count.
- **Strategy**: `any(u64)` for timeout, `any(u32)` for attempts.

## 5. Fuzz Targets

### Fuzz Target: `parse_session_database`
- **Input type**: `&[u8]` (Raw JSONL content).
- **Risk**: Heap exhaustion or panic on malformed line-delimited JSON.
- **Corpus seeds**: Valid database files, empty files, files with extremely long lines, files with partial JSON objects.

## 6. Kani Harnesses

### Kani Harness: `sync_lock_state_machine`
- **Property**: The lock is mathematically guaranteed to be released on every possible exit path of the `sync` function (Panic-free path).
- **Rationale**: Critical for preventing deadlocks.

### Kani Harness: `retry_backoff_overflow`
- **Property**: Exponential backoff calculation never overflows `u64`.
- **Bound**: `max_attempts = 32`.

## 7. Mutation Checkpoints

### Mutation Checkpoints
- **Critical**: Swapping `target_branch` logic (ignoring the `Some(branch)` option) must be caught by `sync_named_session_rebases_on_custom_target_branch`.
- **Critical**: Removing `lock.release()` in the `drop` or `catch` block must be caught by `sync_lock_released_on_failure`.
- **Critical**: Changing `if !options.allow_dirty && is_dirty` to `if options.allow_dirty` must be caught by `sync_fails_with_dirty_workspace_when_allow_dirty_is_false`.
- **Critical**: Deleting JSONL `Action` emission must be caught by `sync_named_session_rebases_on_default_branch`.
- **Critical**: Deleting JSONL `Issue` emission must be caught by `sync_lock_released_on_failure`.
- **Critical**: Deleting database `last_synced` update must be caught by `sync_named_session_rebases_on_default_branch`.

**Threshold**: 90% mutation kill rate.

## 8. Combinatorial Coverage Matrix

| Session State | `allow_dirty` | Workspace Dirty | `target_branch` | Expected Outcome | Side Effects |
|---------------|---------------|-----------------|-----------------|------------------|--------------|
| Active        | false         | No              | None            | `Ok(SyncSummary)`| DB updated, Actions emitted, Lock released |
| Active        | true          | Yes             | Some("dev")     | `Ok(SyncSummary)`| DB updated, Actions emitted, Lock released |
| Syncing       | true          | No              | None            | `Err(AlreadySyncing)`| Issue emitted, Lock released |
| Synced        | true          | No              | None            | `Err(TerminalState)` | Issue emitted, Lock released |
| Failed        | false         | No              | None            | `Ok(SyncSummary)`| DB updated, Actions emitted, Lock released |
| Active        | false         | Yes             | None            | `Err(DirtyWorkspace)`| Issue emitted, Lock released |

## 9. Reliability Standards (NASA/JPL/Holzmann)
1. **Rule 2 (Fixed Bounds)**: All loops (retries) have a hard-coded `max_attempts`.
2. **Rule 5 (Data Integrity)**: Database updated via atomic write-and-rename.
3. **Rule 7 (Check Returns)**: Every `jj` output and IO `Result` handled; zero `.unwrap()`.
4. **Zero-Panic**: Crate decorated with `#![deny(clippy::unwrap_used, clippy::expect_used)]`.
