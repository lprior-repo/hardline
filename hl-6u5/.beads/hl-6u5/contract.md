# Contract Specification: Port CLI Sync Command (hl-6u5)

## Context
- **Feature**: Port the `sync` command from `isolate` to `hardline`.
- **Domain terms**:
    - **Workspace**: A JJ repository or a specific JJ workspace.
    - **Session**: A named logical unit of work tracked in the session database.
    - **Sync**: The process of rebasing a session's revision onto the main branch.
    - **Sync Lock**: A cross-process file-based lock (`.jj/hardline/sync.lock`) to serialize JJ operations.
    - **JSONL**: Line-delimited JSON output format for AI-readability.
- **Assumptions**:
    - `jj` is installed and available in the system PATH.
    - The session database is a JSONL file managed by `scp_core`.
    - `scp_core` provides the necessary VCS abstraction (`VcsBackend`) and output types.
- **Open questions**:
    - Should we support auto-resolution of simple conflicts? (Assumed: No, report as `Conflict` error).

## Preconditions
- [ ] The command must be executed within a directory managed by JJ (main repo or workspace).
- [ ] If a specific session is named, it must exist in the session database and be in an `Active` or `Failed` state.
- [ ] The cross-process sync lock must be acquirable (waiting up to a configured timeout).
- [ ] The workspace must be "clean" (no uncommitted changes) unless `--allow-dirty` is specified.

## Postconditions
- [ ] The session's revision is rebased onto the target branch (e.g., `main`).
- [ ] The `last_synced` timestamp in the session database is updated to the current time on success.
- [ ] The sync lock is released, even if the operation fails.
- [ ] All significant steps (lock acquisition, rebase start, rebase result) are emitted as JSONL `Action` or `Issue` records.
- [ ] A final `ResultOutput` and `Summary` are emitted via JSONL.

## Invariants
- [ ] No more than one `sync` operation can modify the same JJ repository concurrently (enforced by `Sync Lock`).
- [ ] A session's state in the database never transitions from `Syncing` to `Active` without a successful rebase or an explicit error recovery.
- [ ] Output is always valid JSONL according to `scp_core::output_jsonl` schemas.
- [ ] **Zero unwrap/panic** - The implementation must use `Result` for all fallible operations and avoid any panic-inducing calls.

## Error Taxonomy
- `SyncError::WorkspaceNotFound(PathBuf)` - Path is not a JJ repository.
- `SyncError::WorkspacePathNotAccessible(PathBuf)` - Workspace directory exists but is not readable.
- `SyncError::SessionNotFound(String)` - Named session does not exist in the database.
- `SyncError::SessionAlreadySyncing(String)` - Session is currently being synced by another process (state is `Syncing`).
- `SyncError::SessionTerminalState(String)` - Session is already `Synced`, `Completed`, or `Failed`.
- `SyncError::LockAcquisitionFailed(String)` - Could not create or write the lock file.
- `SyncError::LockHeldByOther { pid, holder }` - Sync lock already held by another process.
- `SyncError::LockTimeout(u64)` - Timed out waiting for the sync lock.
- `SyncError::DirtyWorkspace(String)` - Workspace has uncommitted changes and `allow_dirty` is false.
- `SyncError::JjCommandFailed(String)` - The `jj` command exited with a non-zero status (e.g., repository corruption).
- `SyncError::Conflict { workspace, files }` - Rebase resulted in merge conflicts in the specified files.
- `SyncError::RetryLimitExceeded(u32)` - Exponential backoff reached maximum attempts for transient JJ failures.
- `SyncError::SessionDatabaseNotFound(PathBuf)` - Could not locate the session database.
- `SyncError::SessionDatabaseReadFailed(String)` - IO error or corruption when reading the database.
- `SyncError::SessionDatabaseWriteFailed(String)` - IO error when updating the database.
- `SyncError::ConfigurationError(String)` - Invalid retry or timeout parameters.
- `SyncError::IoError(std::io::Error)` - General IO failure.

## Contract Signatures
- `pub async fn sync_named_session(session_name: SessionName, options: SyncOptions) -> Result<SyncSummary, SyncError>`
- `pub async fn sync_all_sessions(options: SyncOptions) -> Result<SyncSummary, SyncError>`
- `pub async fn sync_current_workspace(options: SyncOptions) -> Result<SyncSummary, SyncError>`

### Supporting Types
```rust
pub struct SyncOptions {
    pub allow_dirty: bool,
    pub target_branch: Option<String>,
    pub lock_timeout_secs: u64,
    pub retry_config: RetryConfig,
}

pub struct RetryConfig {
    pub max_attempts: u32, // Default: 3
    pub initial_delay_ms: u64,
}

pub struct SyncSummary {
    pub sessions_synced: Vec<SessionName>,
    pub total_operations: u32,
    pub had_conflicts: bool,
}
```

## Non-goals
- [ ] Support for non-JJ VCS (Git-only repositories).
- [ ] Automatic conflict resolution.
- [ ] Syncing multiple repositories in a single command.
