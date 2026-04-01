# Session Command Porting Map: Isolate -> Hardline

## 1. Isolate Session Source Files

### 1a. CLI Command Layer (`~/src/isolate/crates/isolate/src/commands/`)

| File | Purpose |
|------|---------|
| `session_command.rs` | Main dispatcher: routes `isolate session <action>` to handlers. 10 subcommands: list, add, remove, pause, resume, clone, rename, spawn, sync, init |
| `session_mgmt.rs` | Pause/Resume/Clone implementations with `PauseOptions`, `ResumeOptions`, `CloneOptions`, and result types |

### 1b. Session Data Model (`~/src/isolate/crates/isolate/src/session.rs`)

Key types:
- `SessionStatus` enum: `Creating | Active | Paused | Completed | Failed` (with `Display`, `FromStr`)
- `Session` struct: `{ id: Option<i64>, name, status, state: WorkspaceState, workspace_path, branch: Option<String>, created_at: u64, updated_at: u64, last_synced: Option<u64>, metadata: Option<serde_json::Value> }`
- `SessionUpdate` struct: `{ status, state, branch, last_synced, metadata }` (all `Option<T>`)
- `validate_session_name(name: &str) -> Result<()>` -- ASCII-only, starts with letter, 1-64 chars, no reserved keywords
- `validate_status_transition(from, to) -> Result<()>` -- state machine enforcement

### 1c. Session Business Logic (`~/src/isolate/crates/isolate/src/commands/session_command.rs`)

Two-layer architecture:
- **`SessionManager`** (business logic layer): Wraps `SessionDb` + `LockManager`. Methods:
  - `create_session(name, workspace_path, agent_id) -> Result<Session>`
  - `get_session(name) -> Result<Option<Session>>`
  - `list_sessions(status_filter, include_closed) -> Result<Vec<Session>>`
  - `remove_session(name, force, agent_id) -> Result<()>`
  - `focus_session(name) -> Result<Session>`
  - `pause_session(name, agent_id) -> Result<Session>`
  - `resume_session(name, agent_id) -> Result<Session>`
  - `rename_session(old_name, new_name) -> Result<Session>`
  - `get_current_session() -> Result<Option<Session>>`
  - `count_active_sessions() -> Result<usize>`
- **`SessionCommand`** (shell/CLI layer): Wraps `SessionManager` + `SessionCommandOptions`. Methods:
  - `run_list`, `run_add`, `run_remove`, `run_focus`, `run_pause`, `run_resume`, `run_rename`, `run_status`
  - Emits structured JSONL output via `emit_stdout`, `emit_action`, `emit_result_success`

### 1d. Persistence (`~/src/isolate/crates/isolate/src/db.rs`)

- `SessionDb` struct: SQLx `SqlitePool`-backed
  - `create(name, workspace_path) -> Result<Session>`
  - `get(name) -> Result<Option<Session>>`
  - `update(name, SessionUpdate) -> Result<()>`
  - `delete(name) -> Result<()>`
  - `list(state_filter) -> Result<Vec<Session>>`
  - `create_or_open(path) -> Result<SessionDb>`
  - `pool() -> &SqlitePool`

### 1e. Management Commands (`session_mgmt.rs`)

- `run_pause(PauseOptions) -> Result<()>` -- idempotent, uses `SchemaEnvelope` for JSON output
- `run_resume(ResumeOptions) -> Result<()>` -- validates Paused state
- `run_clone(CloneOptions) -> Result<()>` -- creates JJ workspace, validates name, checks target uniqueness
- Result types: `PauseResult`, `ResumeResult`, `CloneResult` (all serializable)

### 1f. Isolate Core Domain (`~/src/isolate/crates/isolate-core/src/domain/`)

- `session.rs`: `BranchState` enum (`Detached | OnBranch { name }`)
- `session_create.rs`: Session creation with validation (preconditions P1-P7)
- `session_focus.rs`: Focus domain logic with preconditions
- `session_remove.rs`: Remove domain logic with preconditions

---

## 2. Hardline Existing Session Infrastructure

### 2a. Session Crate (`~/src/hardline/crates/session/`)

**Entities** (`domain/entities/session.rs`):
- Typestate-based `Session<S>` generic struct with compile-time state markers: `Created`, `Active`, `Syncing`, `Synced`, `Paused`, `Completed`, `Failed`
- Fields: `{ id: SessionId, name: SessionName, workspace: Option<WorkspaceId>, bead: Option<BeadId>, branch: BranchState, last_synced: Option<DateTime<Utc>>, created_at: DateTime<Utc> }`
- State transitions via consuming methods: `activate()`, `sync()`, `sync_complete()`, `pause()`, `resume()`, `complete()`, `fail()`, `restart()`, `retry()`, `reactivate()`
- `BranchState` enum: `Detached | OnBranch { name: String }`
- `SessionState` enum: `Created | Active | Syncing | Synced | Paused | Completed | Failed`
- `StateInfo` trait + `SealedActive` trait for compile-time safety

**Value Objects** (`domain/value_objects/session.rs`):
- `SessionName` newtype with `parse()` -- max 63 chars, starts with letter, ASCII alphanumeric + dash + underscore
- `IdentifierError` enum: `Empty | TooLong | InvalidCharacters | InvalidStart | NotAscii`

**Repository Trait** (`infrastructure/repository.rs`):
- `SessionRepository` trait: `save`, `find_by_id`, `find_by_name`, `list`, `delete`

**SQLite Implementation** (`infrastructure/sqlite_session_repository.rs`):
- `SqliteSessionRepository` with `SqliteDatabaseService`
- `SessionRow` for DB mapping with `TryFrom<SessionRow> for Session<Created>`
- Schema: `id TEXT PK, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at`

**Application Service** (`application/session_service.rs`):
- `SessionService` -- thin wrapper, mostly stub (`list_sessions` returns empty, `get_session` returns `NotFound`)
- Methods: `create_session`, `activate_session`, `complete_session`, `fail_session`, `list_sessions`, `get_session`

**Errors** (`error.rs`):
- `SessionError` enum: comprehensive -- `NotFound`, `AlreadyActive`, `Expired`, `InvalidTransition`, `InvalidSessionTransition`, `InvalidBranchTransition`, `WorkspaceNotFound`, `WorkspaceExists`, `WorkspaceLocked`, etc.

### 2b. Core Session Types (`~/src/hardline/crates/core/src/`)

**Aggregate** (`domain/session.rs` -- in `types/` module):
- `Session` struct: `{ id: SessionId, name: SessionName, status: SessionStatus, state: WorkspaceState, workspace_path: AbsolutePath, branch: BranchState, created_at: DateTime<Utc>, updated_at: DateTime<Utc>, last_synced: Option<DateTime<Utc>>, metadata: ValidatedMetadata }`
- `validate_pure()`, `validate()`, `name()` methods

**Status** (`type_session_status.rs`):
- `SessionStatus` enum: `Creating | Active | Paused | Completed | Failed` (same as isolate)
- `Operation` enum: `Status | Diff | Focus | Remove`
- `can_transition_to()`, `valid_next_states()`, `is_terminal()`, `allowed_operations()`
- Implements `LifecycleState` trait

**ID/Name/Path** (`type_session_id.rs`, `type_session_name.rs`, `type_session_path.rs`):
- `SessionId`: alphanumeric + hyphens
- `SessionName`: 1-63 chars, starts with letter, ASCII alphanumeric + dash + underscore (matches isolate)
- `AbsolutePath`: validates absolute path

**State Machine** (`session_state.rs`):
- `SessionState` enum: `Created | Active | Syncing | Synced | Paused | Completed | Failed`
- `SessionStateManager<S>` with phantom types and consuming transitions
- `StateTransition` event with validation

**Domain Logic**:
- `session_create.rs`: `SessionCreator`, `SessionCreateInput/Output`, `SessionLimits`, validation functions
- `session_focus.rs`: `SessionFocusInput/Output`, `SessionFocusState`, `validate_focus_preconditions()`, `should_switch_workspace()`, `update_focus_state()`
- `session_remove.rs`: `SessionRemoveInput/Output`, `validate_removal_preconditions()`, `should_delete_workspace()`, `WorkspaceCleanupStrategy`
- `session_sync.rs`: `SessionSyncInput`, `SessionSyncResult`, `SyncError`, `validate_sync_preconditions()`, `parse_rebase_output()`, `determine_workspace_status()`

### 2c. CLI Session Layer (`~/src/hardline/crates/cli/src/`)

**Command Dispatch** (`commands/session.rs`):
- `list()`, `status()`, `focus()`, `submit()`, `remove()` -- implemented using VCS backend
- Uses `scp_core::vcs` directly (no SessionDb)
- `focus()` calls `backend.switch_workspace()`
- `submit()` handles dirty working copy with auto-commit
- `remove()` with merge option

**Clap Args** (`cli/session_args.rs`):
- `SessionCommands` enum: `List`, `Status`, `Focus { name }`, `Submit { name, auto_commit, message }`, `Remove { name, force, merge }`

**Object Commands** (`commands/object_commands/session.rs`):
- Full subcommand definitions: `list`, `add`, `remove`, `pause`, `resume`, `clone`, `rename`, `spawn`, `sync`, `init`
- Comprehensive arg definitions matching isolate's CLI surface

**Isolate Commands** (`commands/isolate_commands_session.rs`):
- `cmd_add()`, `cmd_list()`, `cmd_remove()`, `cmd_focus()` -- Clap command definitions with AI contract/hints support

**Handler** (`commands/handlers/session.rs`):
- **STUB** -- empty file: `// Stub - requires isolate-specific dependencies`

---

## 3. Gap Analysis

### 3.1 Already Ported (Exists in Both)

| Feature | Isolate | Hardline |
|---------|---------|----------|
| SessionStatus enum | `Creating/Active/Paused/Completed/Failed` | Same (both in core `type_session_status` and session crate `SessionState`) |
| SessionName validation | 1-64 chars, starts with letter, ASCII | 1-63 chars, same rules (minor length diff: 64 vs 63) |
| BranchState enum | `Detached/OnBranch` | Same in both core and session crate |
| State machine transitions | `validate_status_transition()` | `can_transition_to()` in both `SessionStatus` and `SessionState` |
| Basic CLI list/status/focus/remove | VCS-backend approach | Implemented in `commands/session.rs` |
| Clap arg definitions | 10 subcommands | All defined in `object_commands/session.rs` |
| Session persistence | `SessionDb` (SQLx) | `SqliteSessionRepository` (SQLx via `scp_core::infrastructure`) |
| Session aggregate | `Session` with `Option<i64>` ID | Typestate `Session<S>` with `SessionId` |
| Sync domain logic | In `session_sync.rs` (core) | Fully ported with `SyncError`, `SessionSyncInput`, calculations |

### 3.2 Partially Ported (Exists but Incomplete)

| Feature | Status | Gap |
|---------|--------|-----|
| `SessionManager` (business logic layer) | Not ported | Isolate has a unified manager wrapping DB + locks. Hardline has no equivalent -- CLI commands go directly to VCS backend or would need to go through `SessionService` (which is stubbed) |
| `SessionCommand` (shell layer with JSONL output) | Not ported | Isolate emits structured JSONL. Hardline's `commands/session.rs` uses `println!` |
| `SessionService` application layer | Stub | Only `create_session` works; `list_sessions` returns empty, `get_session` returns `NotFound` |
| `SessionRepository` implementation | Partial | `SqliteSessionRepository` has `init_schema` and mapping, but the `SessionRepository` trait impl is incomplete |
| Pause/Resume commands | CLI args defined, handler is stub | `session_mgmt.rs` logic needs porting to use hardline's typestate `Session<S>` |
| Clone command | CLI args defined, handler is stub | Clone creates JJ workspace + DB session. Needs hardline's workspace infrastructure |
| Rename command | CLI args defined, handler is stub | Rename = create new + copy metadata + delete old |
| Spawn command | CLI args defined, handler is stub | Agent-specific session creation with bead association |
| Add command | CLI args defined, handler is stub | `AddOptions` with bead, no-hooks, no-open, idempotent |
| Submit command | Implemented | Works with VCS backend, handles dirty working copy |

### 3.3 Not Ported (Missing in Hardline)

| Feature | Isolate Location | Description |
|---------|-----------------|-------------|
| `SessionManager` unified business logic | `commands/session_command.rs` | Wraps `SessionDb` + `LockManager` into cohesive API |
| `SessionCommand` JSONL shell layer | `commands/session_command.rs` | Structured output with `emit_action`, `emit_session_output`, `emit_result_success` |
| Lock integration for session ops | `session_command.rs` (pause/resume/remove acquire locks) | Hardline has `LockManager` in `coordination/locks/` but it is not wired into session commands |
| Pause idempotency | `session_mgmt.rs` | Pausing already-paused session succeeds silently |
| Clone with JJ workspace creation | `session_mgmt.rs` | `isolate_core::jj::workspace_create()` + DB create + status update |
| Session name reserved keywords check | `session.rs` | `RESERVED_SESSION_NAMES` check (null, undefined, true, false, etc.) |
| `AddOptions` struct with hooks/open control | `add.rs` | `no_hooks`, `no_open`, `idempotent`, `dry_run`, `bead_id` |
| `RemoveOptions` with merge/keep-branch | `remove.rs` | `force`, `merge`, `keep_branch`, `idempotent`, `dry_run` |
| `SpawnOptions` with agent/timeout | `spawn.rs` | `agent_command`, `agent_args`, `no_auto_merge`, `no_auto_cleanup`, `background`, `timeout_secs` |
| `SyncOptions` with push/pull | `sync.rs` | `all`, `dry_run` plus push/pull flags |
| `get_current_session()` by CWD matching | `session_command.rs` | Finds session whose `workspace_path` is a prefix of CWD |
| `count_active_sessions()` | `session_command.rs` | Counts Creating + Active sessions |
| `SchemaEnvelope` JSON output | `session_mgmt.rs` | Envelope wrapper `{"schema": "pause-response", ...}` |
| CLI handler dispatcher | `session_command.rs` `handle_session()` | Routes subcommand args to handler functions |

### 3.4 Architectural Differences

| Aspect | Isolate | Hardline |
|--------|---------|----------|
| Error handling | `anyhow::Result` everywhere | Domain `Error` enum with `thiserror`, typed error kinds |
| Session state | Runtime enum `SessionStatus` | **Both** runtime `SessionStatus` (core) AND typestate `Session<S>` (session crate) |
| Timestamps | `u64` Unix epoch | `DateTime<Utc>` (chrono) |
| Session ID | `Option<i64>` auto-increment | `SessionId` newtype (UUID-based) |
| Workspace path | `String` | `AbsolutePath` newtype with validation |
| Branch | `Option<String>` | `BranchState` enum (`Detached | OnBranch`) |
| Metadata | `Option<serde_json::Value>` | `ValidatedMetadata` |
| Persistence | Direct `SessionDb` with inline SQL | `SessionRepository` trait + `SqliteSessionRepository` via `DatabaseService` |
| Output | `SchemaEnvelope` JSON or `writeln!` human | `Output` helper or `println!` (no JSONL envelope yet) |
| Name length max | 64 | 63 |

---

## 4. Porting Priority

### Phase 1: Wire SessionService to SessionRepository
- Complete `SessionService` stubs (`list_sessions`, `get_session`) by delegating to `SqliteSessionRepository`
- Add `update` method to `SessionRepository` trait (currently missing)

### Phase 2: Port SessionManager Business Logic
- Create `SessionManager` in hardline that wraps `SessionRepository` + `LockManager`
- Port: `create_session`, `remove_session`, `focus_session`, `pause_session`, `resume_session`, `rename_session`, `get_current_session`, `count_active_sessions`
- Use hardline's typestate `Session<S>` for transitions instead of runtime checks

### Phase 3: Port Shell Layer (SessionCommand)
- Create `SessionCommand` handler that emits structured output
- Wire to `SessionManager` instead of direct VCS calls
- Support both human-readable and JSON output formats

### Phase 4: Port Remaining Subcommand Handlers
- `add` with `AddOptions` (bead, hooks, open, idempotent, dry-run)
- `clone` with JJ workspace creation
- `spawn` with agent workflow
- `remove` with merge/keep-branch options
- Complete the stub at `commands/handlers/session.rs`

### Phase 5: Reconcile Dual Session Type Systems
- Hardline has TWO session models: `core::types::Session` (runtime status) and `session::entities::Session<S>` (typestate)
- Decide which is authoritative, or define a clear mapping between them
- The session crate's typestate model is more rigorous; the core model is used by CLI handlers
