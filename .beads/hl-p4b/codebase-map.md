# Codebase Map: Hardline `done` Command Port

## 1. Hardline `done` Command File Inventory

**Directory:** `~/src/hardline/crates/hardline/src/commands/done/`

| File | Purpose |
|------|---------|
| `mod.rs` (800 lines) | Main entry point and orchestration. Public API: `run_with_options`, `execute_done`. Implements the full done pipeline: validate location, commit uncommitted changes, detect conflicts, merge to main, log undo history, update bead/session status, cleanup workspace. |
| `types.rs` (394 lines) | All public types: `DoneArgs`, `DoneOptions`, `DoneOutput`, `DoneError`, `UndoEntry`, `DonePreview`, `CommitInfo`, `DonePhase`. Error conversions from sub-module error types. Inline tests. |
| `conflict.rs` (1026 lines) | Pre-merge conflict detection. `ConflictDetector` trait, `JjConflictDetector` implementation, `ConflictDetectionResult`, `ConflictError`, `ConflictExitCode`. JSONL output conversion for AI consumers. Resolution option generation per conflict type. |
| `executor.rs` (140 lines) | JJ command executor abstraction. `JjExecutor` trait, `RealJjExecutor`, `WorkspaceExecutor` (wraps executor with `-R` flag for workspace-scoped commands). `ExecutorError` enum. |
| `bead.rs` (90 lines) | Bead repository abstraction. `BeadRepository` trait, `RealBeadRepository` wrapper. `BeadError` enum. |
| `filesystem.rs` (104 lines) | Filesystem abstraction for testability. `FileSystem` trait, `RealFileSystem` using tokio::fs. `FsError` enum. |
| `newtypes.rs` (182 lines) | Validated wrappers: `RepoRoot`, `CommitId`, `JjOutput`, `ValidationError`. Re-exports `BeadId` and `WorkspaceName` from hardline_core::domain. |

### Done-related files outside `done/` directory

| File | Relationship |
|------|-------------|
| `commands/abort.rs` | Opposite of done: removes workspace instead of merging it. |
| `commands/whatif/tests.rs` | Multiple tests for done command simulation ("what would done do"). |
| `commands/work.rs` | References `hardline done` in user-facing messages. |
| `commands/context/types.rs` | Defines `Location` enum used by done's `validate_location()`. |
| `commands/context/mod.rs` | Defines `detect_location()` function used by done. |
| `commands/mod.rs` | Declares `pub mod done` and provides `get_session_db()` helper. |

---

## 2. Public Types and Fields

### `DoneArgs` (CLI argument struct)
```
workspace: Option<String>        // Target workspace name
message: Option<String>          // Commit message
keep_workspace: bool             // --keep-workspace flag
no_keep: bool                    // --no-keep flag
squash: bool                     // --squash flag
dry_run: bool                    // --dry-run flag
detect_conflicts: bool           // --detect-conflicts flag
no_bead_update: bool             // --no-bead-update flag
format: OutputFormat             // json or text
```

### `DoneOptions` (Internal options, same fields as DoneArgs)
Identical fields to DoneArgs. Converted via `DoneArgs::to_options()`.

### `DoneOutput` (Command result)
```
workspace_name: String
bead_id: Option<String>
files_committed: usize
commits_merged: usize
merged: bool
cleaned: bool
bead_closed: bool
session_updated: bool
new_status: Option<String>       // "completed" if session was updated
pushed_to_remote: bool
dry_run: bool
preview: Option<DonePreview>
error: Option<String>
```

### `DonePreview` (Dry-run output)
```
uncommitted_files: Vec<String>
commits_to_merge: Vec<CommitInfo>
potential_conflicts: Vec<String>
bead_to_close: Option<String>
workspace_path: String
conflict_detection: Option<ConflictDetectionResult>
```

### `CommitInfo`
```
change_id: String
commit_id: String
description: String
timestamp: String
```

### `UndoEntry` (Logged to .hardline/undo.log)
```
session_name: String
commit_id: String
pre_merge_commit_id: String
timestamp: u64
pushed_to_remote: bool
status: String
```

### `DoneError` (Error enum)
```
NotInWorkspace { current_location: String }
NotAJjRepo
WorkspaceNotFound { workspace_name: String }
CommitFailed { reason: String }
MergeConflict { conflicts: Vec<String> }
MergeFailed { reason: String }
CleanupFailed { reason: String }
BeadUpdateFailed { reason: String }
JjCommandFailed { command: String, reason: String }
InvalidState { reason: String }
```
Methods: `error_code() -> &'static str`, `is_recoverable() -> bool`, `phase() -> DonePhase`

### `DonePhase` (Error phase enum)
```
ValidatingLocation     // Initial validation
CommittingChanges      // Committing uncommitted changes
MergingToMain          // Merge, cleanup, bead update
```
Method: `name() -> &'static str`

### `ConflictDetectionResult`
```
has_existing_conflicts: bool
existing_conflicts: Vec<String>
overlapping_files: Vec<String>
workspace_only: Vec<String>
main_only: Vec<String>
merge_likely_safe: bool
summary: String
merge_base: Option<String>
files_analyzed: usize
detection_time_ms: u64
```
Methods: `has_conflicts()`, `conflict_count()`, `exit_code()`, `to_text_output()`, `to_conflict_analysis(session)`, `to_output_line(session)`, `emit_jsonl(session)`

### `ConflictError`
```
StatusFailed(String)
MergeBaseFailed(String)
DiffFailed(String)
JjFailed(String)
InvalidOutput(String)
InvalidState(String)
```

### `ConflictExitCode` (repr i32)
```
Safe = 0
Conflicts = 1
Error = 3
```

### `Location` (from context module)
```
Main
Workspace { name: String, path: String }
```

---

## 3. Public Function Signatures

### `mod.rs`
```rust
pub async fn run_with_options(options: &DoneOptions) -> Result<()>
pub async fn execute_done(
    options: &DoneOptions,
    executor: &dyn executor::JjExecutor,
    bead_repo: &mut dyn bead::BeadRepository,
    filesystem: &dyn filesystem::FileSystem,
) -> Result<DoneOutput, DoneError>
```

### `conflict.rs`
```rust
pub async fn run_conflict_detection<E: JjExecutor + ?Sized>(
    executor: &E,
) -> Result<ConflictDetectionResult, ConflictError>

pub async fn has_conflicts<E: JjExecutor + ?Sized>(
    executor: &E,
) -> Result<bool, ConflictError>

pub fn generate_resolution_options(
    conflict_type: ConflictType,
    file: &str,
) -> Vec<ResolutionOption>

pub const fn recommended_strategy(conflict_type: ConflictType) -> ResolutionStrategy
pub fn emit_conflict_details_jsonl(result: &ConflictDetectionResult, session: &str) -> Result<(), ConflictError>
```

### `executor.rs`
```rust
// Trait
pub trait JjExecutor: Send + Sync {
    fn run<'a>(&'a self, args: &'a [&'a str]) -> BoxFuture<'a, Result<JjOutput, ExecutorError>>;
    fn run_with_env<'a>(&'a self, args: &'a [&'a str], env: &'a [(&'a str, &'a str)]) -> BoxFuture<'a, Result<JjOutput, ExecutorError>>;
}
// Concrete: RealJjExecutor, WorkspaceExecutor
pub struct RealJjExecutor { ... }   // new() -> Self
pub struct WorkspaceExecutor<'a> { ... }  // new(inner: &'a dyn JjExecutor, workspace_path: PathBuf) -> Self
```

### `bead.rs`
```rust
pub trait BeadRepository: Send + Sync {
    fn find_by_workspace<'a>(&'a self, workspace: &'a WorkspaceName) -> BoxFuture<'a, Result<Option<BeadId>, BeadError>>;
    fn update_status<'a>(&'a mut self, id: &'a BeadId, status: &'a str) -> BoxFuture<'a, Result<(), BeadError>>;
}
// Concrete: RealBeadRepository::new(root: PathBuf) -> Self
```

### `filesystem.rs`
```rust
pub trait FileSystem: Send + Sync {
    fn read_to_string<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<String, FsError>>;
    fn write<'a>(&'a self, path: &'a Path, contents: &'a str) -> BoxFuture<'a, Result<(), FsError>>;
    fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, bool>;
    fn remove_dir_all<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<(), FsError>>;
}
// Concrete: RealFileSystem::new() -> Self (const)
```

### `newtypes.rs`
```rust
pub struct RepoRoot(PathBuf)   // async fn new(PathBuf) -> Result<Self, ValidationError>
pub struct CommitId(String)    // fn new(String) -> Result<Self, ValidationError>
pub struct JjOutput(String)    // fn new(String) -> Result<Self, ValidationError>
// Re-exports: BeadId, WorkspaceName from hardline_core::domain
```

---

## 4. Error Types Summary

| Error Type | Location | Used For |
|-----------|----------|----------|
| `DoneError` | `types.rs` | Top-level done command errors. Converts from ExecutorError, BeadError, FsError. |
| `ExecutorError` | `executor.rs` | JJ command execution failures (not found, failed, invalid UTF-8, IO). |
| `ConflictError` | `conflict.rs` | Conflict detection failures. Converts from ExecutorError. |
| `BeadError` | `bead.rs` | Bead database lookup/update failures. |
| `FsError` | `filesystem.rs` | Filesystem operation failures (not found, permission denied, IO). |
| `ValidationError` | `newtypes.rs` | Newtype construction validation failures. |

Error conversion chain: `ExecutorError` -> `ConflictError` / `DoneError`
Error conversion chain: `BeadError` -> `DoneError`
Error conversion chain: `FsError` -> `DoneError`

---

## 5. Dependencies on Other Hardline Modules

| Module | Usage |
|--------|-------|
| `hardline_core::OutputFormat` | Output format (JSON vs text) |
| `hardline_core::json::SchemaEnvelope` | JSON output envelope |
| `hardline_core::WorkspaceState` | Session state enum (Merged, etc.) |
| `hardline_core::output::{ConflictAnalysis, ConflictDetail, ...}` | JSONL streaming output types |
| `crate::cli::jj_root` | Get JJ repository root path |
| `crate::commands::context::{detect_location, Location}` | Detect if in workspace or main |
| `crate::commands::get_session_db` | Open the session database |
| `crate::session::{SessionStatus, SessionUpdate}` | Session status transitions |
| `crate::beads::{BeadRepository as RealBeadRepo, BeadStatus}` | Actual bead database operations |
| `hardline_core::domain::{BeadId, WorkspaceName}` | Validated identifier types |

---

## 6. How Conflicts Are Detected and Resolved

### Detection Strategy (5-step process in `ConflictDetector::detect_conflicts`)

1. **Check existing JJ conflicts** (`check_existing_conflicts`)
   - Runs `jj log -r @ --no-graph -T 'if(conflict, "CONFLICT\n", "")'`
   - If CONFLICT found, runs `jj resolve --list` to enumerate conflicted files
   - Parses file paths from resolve output

2. **Find merge base** (`find_merge_base`)
   - Runs `jj log -r "heads(::@ & ::trunk())" --no-graph -T commit_id --limit 1`
   - Returns the most recent common ancestor of workspace and trunk

3. **Get workspace modified files** (`get_workspace_modified_files`)
   - Runs `jj diff --from trunk() --to @ --summary`
   - Parses output lines like `M path`, `A path`, `D path`, `R old -> new`

4. **Get trunk modified files** (`get_trunk_modified_files`)
   - Runs `jj diff --from <merge_base> --to trunk() --summary`
   - Falls back to `jj diff --from @ --to trunk() --summary` if no merge base

5. **Compute overlap** (in `detect_conflicts`)
   - Uses HashSet intersection to find files modified in both workspace and trunk
   - Determines `merge_likely_safe = !has_existing && overlapping.is_empty()`

### Conflict Resolution

The done command does NOT auto-resolve conflicts. It:
- Aborts with `DoneError::MergeConflict` if conflicts are found
- Provides resolution hints in text output: `jj resolve`, `jj rebase -d trunk()`
- Generates structured `ResolutionOption` lists per conflict type for JSONL consumers:
  - `Existing`: jj_resolve, manual_merge, abort
  - `Overlapping`: jj_resolve, manual_merge, accept_ours, accept_theirs, rebase
  - `DeleteModify`: accept_ours, accept_theirs, manual_merge
  - `RenameModify`: manual_merge, jj_resolve
  - `Binary`: accept_ours, accept_theirs, skip

---

## 7. The Merge Workflow (Phase-by-Phase)

### Phase 1: Validate Location
- `validate_location()` checks we're in a workspace (not main)
- Uses `detect_location()` which checks `.jj/repo` is a file (workspace indicator)
- Accepts explicit `--workspace` name to operate on any workspace

### Phase 2: Dry-Run (optional early exit)
- `build_preview()` gathers: uncommitted files, commits to merge, potential conflicts, bead ID
- Returns `DoneOutput` with `dry_run: true` and `preview` populated

### Phase 3-4: Prepare Workspace
- `prepare_workspace_for_merge()` calls `get_uncommitted_files()` using `jj status --no-pager`
- Parses lines starting with A/M/D/R prefixes
- If files found, calls `commit_changes()` which runs `jj commit -m <message>`
- Default message: "Complete work on {workspace_name}"

### Phase 5: Check Conflicts
- `check_conflicts()` wraps `check_potential_conflicts()`
- Creates `JjConflictDetector` and calls full 5-step detection
- Returns `DoneError::MergeConflict` with file list if conflicts found

### Phase 5.5-6: Gather Merge Metadata
- `get_current_commit_id()`: `jj log -r @ --no-graph -T commit_id` (pre-merge snapshot)
- `is_pushed_to_remote()`: `jj log -r @-` (checks if parent is empty = pushed)
- `get_commits_to_merge()`: `jj log -r @..@-` with template extracting change_id, commit_id, description, timestamp

### Phase 7: Merge to Main
- **Squash mode**: `jj squash --from "ancestors({workspace}@) & ~ancestors(main)" --into main -m <message>`
- **Normal mode**: `jj workspace forget <workspace_name>` (absorbs commits into main)
- Log undo history to `.hardline/undo.log` (JSONL format with UndoEntry)

### Phase 8-9: Finalize
- **Bead update**: Look up bead ID by workspace name, update status to "closed"
- **Session update**: Set status to `Completed`, state to `Merged`
- **Cleanup**: Remove workspace directory unless `--keep-workspace` specified
- Return `DoneOutput` with all outcome flags

---

## 8. Hardline Existing Code Summary

### Handler Files at `~/src/hardline/crates/cli/src/commands/handlers/`

| File | Purpose |
|------|---------|
| `mod.rs` | Module declarations: ai, batch, json_format, sync, task |
| `ai/` | AI command handler with Data/Calc/Actions split |
| `task/` | Task command handler with Data/Calc/Actions split |
| `batch.rs` | Atomic batch execution with checkpoint/rollback |
| `sync.rs` | Session sync handler ported from hardline (rebase with retries, JSONL output) |
| `json_format.rs` | JSON formatting utilities |
| `backup.rs`, `bookmark.rs`, etc. | Stub files (49 bytes each) |

### Hardline Core Domain Types (`~/src/hardline/crates/core/src/`)

**Relevant to done command:**

| Type | File | Description |
|------|------|-------------|
| `Session` | `type_session.rs` | Aggregate with id, name, status, state, workspace_path, branch, timestamps, metadata |
| `SessionName` | `type_session_name.rs` | Validated: 1-63 chars, starts with letter, alphanumeric + dash + underscore |
| `SessionId` | `type_session_id.rs` | Validated session identifier |
| `SessionStatus` | `type_session_status.rs` | State machine: Creating -> Active -> Paused/Completed/Failed |
| `WorkspaceState` | `workspace_state.rs` | Lifecycle: Created -> Working -> Ready -> Merged/Abandoned/Conflict |
| `AbsolutePath` | `type_session_path.rs` | Validated absolute path |
| `BranchState` | `type_branch_state.rs` | Detached or OnBranch(String) |
| `ConflictState` | `conflict.rs` | None/Detected/Resolving/Resolved/Failed with transition validation |
| `Conflict` | `conflict.rs` | Conflict aggregate with branch_id, state, description, base_commit, timestamps |
| `ConflictManager` | `conflict.rs` | HashMap-based conflict tracker with register/resolve/fail operations |
| `Error` | `error.rs` | Unified error type with 12 categories (Workspace/Session/Queue/VCS/Config/Agent/IO/State/Internal/JJ/Task/Wait/Lock) |
| `SessionSyncInput` | `session_sync_data.rs` | session_name, workspace_path, main_branch, allow_dirty |
| `SessionSyncResult` | `session_sync_data.rs` | session_name, new_revision, had_conflicts, synced_at |
| `SyncError` | `session_sync_errors.rs` | SessionNotFound, InvalidSessionStatus, DirtyWorkspace, Conflict, RebaseFailure, JjCommandError, IoError |

### Key Differences: Previous vs Current

| Aspect | Previous | Current |
|--------|---------|----------|
| Error handling | `anyhow::Result` at top level, `DoneError` internally | Unified `Error` enum with typed sub-errors |
| Output | `serde_json` + `SchemaEnvelope` | `output_jsonl` module with typed OutputLine variants |
| Conflict detection | `ConflictDetector` trait with `JjConflictDetector` | `ConflictManager` with `ConflictState` state machine |
| Session DB | SQLite via `SessionDb` from hardline | SQLite via `SessionRepository` in session crate |
| VCS operations | `JjExecutor` trait (command-based) | `VcsBackend` trait (more abstract, supports multiple backends) |
| Architecture | Module-per-command with sub-modules | Data/Calc/Actions split per handler |
| State machine | SessionStatus (hardline_core) | SessionStatus + WorkspaceState with transition validation |
| Beads | `BeadRepository` trait with `RealBeadRepository` | `beads` crate with issue database |
