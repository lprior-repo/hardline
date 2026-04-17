# Black Hat Code Review Report

**Date**: 2026-03-30
**Reviewer**: Black Hat Reviewer (automated, merciless)
**Scope**: 3 beads (hl-9nb, hl-c18, hl-d3r)

---

## Bead hl-9nb: ai command

**File**: `crates/cli/src/commands/handlers/ai.rs` (1237 lines)

### PHASE 1: Contract & Bead Parity

- [PASS] Data types are well-defined: `AiStatusOutput`, `WorkflowInfo`, `WorkflowStep`, `AiSubcommand`, `NextActionOutput`, `QuickCommand`, `AiOverview`, `AiEnvelope`.
- [PASS] Calculations are pure functions: `determine_ready_state`, `format_session_count`, `build_workflow`, `build_quick_start`, `build_overview`, `determine_next_action`, `format_status_human`.
- [PASS] Actions handle I/O: `run`, `run_status`, `run_workflow`, `run_quick_start`, `run_next`, `run_default`.
- [FAIL] **AiEnvelope is a local reimplementation** of `scp_core::json::SchemaEnvelope` because "the core json module is not yet public" (line 39). This is a hack. The envelope duplicates structural knowledge instead of making the dependency proper.

### PHASE 2: Farley Engineering Rigor

- [FAIL] **File is 1237 lines. Architectural drift limit is 300.** This is 4x over budget.
- [FAIL] Functions exceeding 25-line limit:
  - `build_workflow`: 42 lines (220-261) -- could be data-driven from a const table
  - `build_quick_start`: 46 lines (267-312) -- same
  - `build_overview`: 28 lines (318-345)
  - `format_status_human`: 37 lines (402-438)
  - `determine_ready_state`: 27 lines (176-202)

### PHASE 3: Functional Rust (Big 6)

- [PASS] No `unwrap()` or `expect()` in non-test code.
- [PASS] `#![cfg_attr(not(test), deny(clippy::unwrap_used))]` lint guard present.
- [WARN] `let mut lines` in `format_status_human` (line 403) -- justified for building a vec.
- [WARN] Primitive `String` used for `location` in `AiStatusOutput` and `determine_ready_state`. A `Location` enum (`Main`, `Workspace(String)`, `NotInRepo`, `Unknown`) would make illegal states unrepresentable.
- [WARN] Primitive `String` for `priority` in `NextActionOutput`. Should be an enum: `Priority { High, Medium, Low }`.
- [WARN] `determine_ready_state(initialized: bool, location: &str)` -- boolean parameter flagged. Should take a typed context.

### PHASE 4: DDD & Simplicity

- [PASS] Clean Data->Calc->Actions separation in the module header.
- [WARN] `build_workflow()` and `build_quick_start()` are 42/46 lines of pure struct construction. These should be `const` or lazy-static data tables, not imperative builders.

### PHASE 5: Bitter Truth

- [WARN] `build_default_status()` and `build_default_next_action()` do filesystem I/O (`std::env::current_dir()`, `.jj/.exists()`) but are called from `run_status()` and `run_next()` which are marked as Tier 3 actions. The I/O is hidden inside private `fn` without the caller being obvious about it. Acceptable but borderline.
- [PASS] No TODO/FIXME/HACK markers.
- [PASS] The AiEnvelope is documented as intentional (line 38-39), but it is still a shortcut.

### VERDICT: **REJECTED**

Reasons:
1. 1237 lines -- 4x over 300-line limit. Must be split.
2. AiEnvelope is a workaround for a visibility problem. Fix the root cause.
3. Primitive strings for `location` and `priority` when enums would make illegal states unrepresentable.
4. Multiple functions over 25-line Farley limit.

---

## Bead hl-c18: session schema reconciliation

**Files**:
- `crates/session/src/infrastructure/migration.rs` (v2 migration added)
- `crates/session/src/domain/entities/session.rs` (Session struct updated)
- `crates/session/src/infrastructure/sqlite_session_repository.rs` (queries updated)

### PHASE 1: Contract & Bead Parity

- [PASS] `BranchState` enum with `Detached`/`OnBranch` properly models the domain.
- [PASS] `Session<S>` typestate pattern is correctly extended with `branch: BranchState` and `last_synced: Option<DateTime<Utc>>`.
- [PASS] `from_parts` constructor includes both new fields.
- [FAIL] **CRITICAL: Schema drift between `migration.rs` and `sqlite_session_repository.rs`.**

  The migration's v1 schema defines:
  ```
  id, name, status, state, workspace_path, created_at, updated_at, metadata, owner
  ```

  The repository's `init_schema()` defines a COMPLETELY DIFFERENT schema:
  ```
  id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at
  ```

  These two schemas share only `id`, `name`, and `created_at`. They are not compatible. The migration v2 adds `branch`/`last_synced` columns to a schema that the repository doesn't use, and the repository uses columns the migration never creates. This is a **schema divergence bug** -- if both `init_schema()` and `migrate_sessions_table()` run on the same database, the second one gets a table with the wrong shape.

### PHASE 2: Farley Engineering Rigor

- [FAIL] `migrate_sessions_table`: 45 lines (200-244) -- over 25-line limit.
- [WARN] `rollback_v2_branch_and_last_synced`: correctly handles SQLite <3.35.0 DROP COLUMN limitation via rename-copy pattern. Good.
- [WARN] `migrate_v2_add_branch_and_last_synced`: 58 lines (271-328) -- well over limit, but the per-column guard pattern justifies some length.

### PHASE 3: Functional Rust (Big 6)

- [PASS] No `unwrap()`/`expect()` in non-test code of migration.rs or session.rs.
- [PASS] `Session<S>` uses typestate pattern -- illegal state transitions are compile-time errors. Excellent.
- [PASS] `BranchState::can_transition_to` enforces state machine rules.
- [FAIL] **`table_exists()` in migration.rs (line 174-176) uses `format!` to interpolate `table_name` into SQL.** This is a SQL injection vector. Although `table_name` is hardcoded in all current call sites, this function is `pub(crate)` and the pattern is dangerous. Should use parameterized queries or at minimum validate the identifier.
- [WARN] `MigrationError` uses `String` for all error context. Fine for infrastructure layer.

### PHASE 4: DDD & Simplicity

- [PASS] Domain entity `Session` is properly separated from infrastructure.
- [PASS] Value objects (`SessionId`, `SessionName`, `WorkspaceId`, `BeadId`) used for domain primitives.
- [PASS] `BranchState` is a proper enum, not a string.
- [WARN] `sqlite_session_repository.rs` still uses `escape_sql_string()` (line 144-146) instead of parameterized queries. This is defense-in-depth failure -- `format!` + manual escaping is fragile. The `save()` method (line 157-178) builds SQL via string formatting. Should use sqlx bind parameters.

### PHASE 5: Bitter Truth

- [FAIL] **`SessionRow` in `sqlite_session_repository.rs` uses `String` for `session_state` (line 19) when `SessionState` is a proper enum.** The `TryFrom` impl converts string->enum, but the row type should carry the validated type earlier.
- [WARN] `let mut sessions` in `SqliteSessionRepository::list()` (line 244) -- could use `try_fold` or `collect` with `Result<Vec<_>>`.
- [PASS] No TODO/FIXME markers.
- [PASS] Idempotent migration pattern is correct and well-tested.

### VERDICT: **REJECTED**

Reasons:
1. **CRITICAL: Schema divergence** between `migration.rs` and `sqlite_session_repository.rs`. These two define incompatible `sessions` table schemas. This will cause runtime failures.
2. SQL injection pattern in `table_exists()` via `format!` interpolation.
3. `save()` uses `format!` + manual escape instead of parameterized queries.
4. Functions over 25-line Farley limit.

---

## Bead hl-d3r: task command

**File**: `crates/cli/src/commands/handlers/task.rs` (970 lines)

### PHASE 1: Contract & Bead Parity

- [PASS] `TaskCommand` enum models all 6 subcommands with correct fields.
- [PASS] Output types (`TaskStatusOutput`, `TaskInfoOutput`, etc.) are serializable data types.
- [PASS] Calculations are pure: `validate_task_command`, `task_state_to_output`, `task_to_output`, `filter_tasks_by_status`, `status_display_icon`, `truncate_description`.
- [FAIL] **CRITICAL: `handlers/task.rs` is dead code.** The CLI dispatches to `commands::task::*` (from `commands/task.rs`), NOT `commands::handlers::task::*`. Verified:
  - `cli/main.rs` line 188-202: calls `commands::task::list()`, `commands::task::show()`, etc.
  - `commands/task.rs` has its own `list()`, `show()`, `claim()`, `start()`, `done()` functions.
  - `handlers::task` is only imported from `ai_kani.rs` for types.
  - `run_task_command()` and `execute_task_command()` are never called from production code.

### PHASE 2: Farley Engineering Rigor

- [FAIL] **File is 970 lines. 3.2x over 300-line limit.**
- [FAIL] Functions exceeding 25-line limit:
  - `execute_list`: 36 lines (369-404)
  - `execute_show`: 26 lines (407-432)

### PHASE 3: Functional Rust (Big 6)

- [FAIL] **`truncate_description` uses `.unwrap_or(0)` on line 302.** While this is technically safe (the `take_while` on a non-empty string will always yield at least one item for `end > 0`), the deny lint only applies `cfg_attr(not(test))`, and this IS in src code. The clippy lint should catch it. However, the `unwrap_or(0)` fallback means an empty `desc[..0]` which produces just "..." -- that is a behavioral bug hiding behind a "safe" default.
- [WARN] `get_agent_id()` (line 308-312) returns `String` directly, no `Result`. The `unwrap_or_else` is fine (it provides a fallback), but this is I/O (env var read) masquerading as a pure function. It should be in Tier 3.
- [WARN] `TaskCommand` variants use `String` for `task_id` and `agent_id` instead of validated `TaskId`/`AgentId` newtypes. The validation happens inside `execute_*` via `TaskId::new(task_id)`, violating Parse Don't Validate at the boundary -- the boundary should be the `TaskCommand` constructor.
- [WARN] `TaskStatusOutput` duplicates `TaskState` domain knowledge. The mapping is explicit, but this is a parallel hierarchy that must be kept in sync.

### PHASE 4: DDD & Simplicity

- [PASS] Clean separation of Data/Calc/Actions.
- [PASS] Domain transitions delegated to `task_validation` module.
- [WARN] `TaskClaimOutput` has both `claimed: bool` and `error: Option<String>`. This is an Option-based result type. Should be `enum ClaimResult { Success { holder: String }, Failed { reason: String } }`.

### PHASE 5: Bitter Truth

- [FAIL] **The entire file is dead code.** `run_task_command()` is the public entry point but is never called from any CLI dispatch path. The actual task commands are in `commands/task.rs`. This means:
  - All the "handler" architecture (Data/Calc/Actions separation, `execute_task_command` dispatch) is unused.
  - The tests verify dead code.
  - There is code duplication between `commands/task.rs` and `commands/handlers/task.rs`.
- [PASS] No TODO/FIXME markers in non-test code.
- [WARN] `TaskStartOutput` has a `status: String` field (line 189) instead of using the `TaskStatusOutput` enum. Untyped.

### VERDICT: **REJECTED**

Reasons:
1. **CRITICAL: Entire file is dead code.** Not wired into CLI dispatch. The actual implementations are in `commands/task.rs`.
2. 970 lines -- 3.2x over 300-line limit.
3. `truncate_description` has an `unwrap_or(0)` that produces "..." for edge cases.
4. `TaskCommand` uses raw `String` for IDs instead of newtypes at the boundary.

---

## Summary Table

| Bead | Status | Critical Issues |
|------|--------|----------------|
| hl-9nb (ai command) | **REJECTED** | 4x over line limit; AiEnvelope hack; untyped primitives |
| hl-c18 (session schema) | **REJECTED** | Schema divergence bug; SQL injection pattern; format! queries |
| hl-d3r (task command) | **REJECTED** | Entire file is dead code; 3.2x over line limit |

## Mandatory Remediation (by priority)

1. **hl-d3r**: Delete `handlers/task.rs` or wire it into the CLI. Currently it is completely dead code with no callers. If the intent was to replace `commands/task.rs`, finish the migration and delete the old one.
2. **hl-c18**: Reconcile the `sessions` table schema between `migration.rs` and `sqlite_session_repository.rs`. They define incompatible columns. Decide on ONE canonical schema definition.
3. **hl-c18**: Replace `format!` SQL construction in `table_exists()` and `save()` with parameterized queries or at minimum add identifier validation.
4. **hl-9nb**: Split `ai.rs` into submodules (data types, calculations, actions). Target <300 lines per file.
5. **hl-9nb**: Replace `String` primitives for `location` and `priority` with proper enums.
6. **hl-9nb**: Either make `scp_core::json::SchemaEnvelope` public or accept the local `AiEnvelope` with a tracked issue to fix the root cause.
