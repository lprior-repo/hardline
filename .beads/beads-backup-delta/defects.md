# Defects Register

## Bead hl-9nb (ai command) -- STATUS: REJECTED

### DEFECT-9NB-1: File size violation (1237 lines / 300 limit)
- **Severity**: HIGH
- **File**: `crates/cli/src/commands/handlers/ai.rs`
- **Description**: File is 4x the 300-line architectural drift limit.
- **Remediation**: Split into submodules: `ai/data.rs`, `ai/calculations.rs`, `ai/actions.rs`, `ai/tests/`.

### DEFECT-9NB-2: AiEnvelope duplicates core type (visibility hack)
- **Severity**: MEDIUM
- **File**: `crates/cli/src/commands/handlers/ai.rs`, lines 36-62
- **Description**: `AiEnvelope<T>` duplicates `scp_core::json::SchemaEnvelope` because the core module is not public. Line 39: "The core json module is not yet public, so we define this here."
- **Remediation**: Make `scp_core::json::SchemaEnvelope` public, or re-export it. Do not duplicate structural types across crate boundaries.

### DEFECT-9NB-3: Primitive string for `location` and `priority`
- **Severity**: MEDIUM
- **File**: `crates/cli/src/commands/handlers/ai.rs`, lines 72, 83, 176
- **Description**: `AiStatusOutput.location` is `String` when it should be `enum Location { Main, Workspace(String), NotInRepo, Unknown }`. `NextActionOutput.priority` is `String` when it should be `enum Priority { High, Medium, Low }`. These are domain concepts that should make illegal states unrepresentable.
- **Remediation**: Define `Location` and `Priority` enums. Update `determine_ready_state` and `determine_next_action` signatures accordingly.

### DEFECT-9NB-4: Multiple functions over 25-line Farley limit
- **Severity**: MEDIUM
- **File**: `crates/cli/src/commands/handlers/ai.rs`
- **Functions**: `build_workflow` (42 lines), `build_quick_start` (46 lines), `build_overview` (28 lines), `format_status_human` (37 lines), `determine_ready_state` (27 lines)
- **Remediation**: Convert `build_workflow` and `build_quick_start` to const/lazy-static data tables. Extract `format_status_human` into smaller sub-formatters.

---

## Bead hl-c18 (session schema reconciliation) -- STATUS: REJECTED

### DEFECT-C18-1: CRITICAL -- Schema divergence between migration and repository
- **Severity**: CRITICAL
- **Files**:
  - `crates/session/src/infrastructure/migration.rs`, lines 99-113 (v1 schema)
  - `crates/session/src/infrastructure/sqlite_session_repository.rs`, lines 119-129 (init_schema)
- **Description**: The migration v1 schema defines columns: `id, name, status, state, workspace_path, created_at, updated_at, metadata, owner`. The repository's `init_schema()` defines: `id, name, workspace, bead, branch_state, branch_name, session_state, last_synced, created_at`. These share only 3 columns (`id, name, created_at`). If both paths run, the second sees a table with the wrong shape.
- **Remediation**: Choose ONE canonical schema definition. Either the migration is the sole schema authority (and the repository uses it), or the repository owns schema creation and the migration is removed. Never have two independent `CREATE TABLE` statements for the same table.

### DEFECT-C18-2: SQL injection pattern in `table_exists()`
- **Severity**: HIGH
- **File**: `crates/session/src/infrastructure/migration.rs`, lines 173-181
- **Description**: `table_exists()` interpolates `table_name` directly into SQL via `format!()`: `"... name='{}'"`. Although current callers pass hardcoded strings, this function is `pub(crate)` and establishes a dangerous pattern.
- **Remediation**: Use parameterized query: `sqlx::query("SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name=?").bind(table_name)`.

### DEFECT-C18-3: `save()` uses format! SQL instead of parameterized queries
- **Severity**: HIGH
- **File**: `crates/session/src/infrastructure/sqlite_session_repository.rs`, lines 150-185
- **Description**: The `save()` method builds its INSERT/UPDATE SQL via `format!()` with `escape_sql_string()`. Manual SQL escaping is inherently fragile and a security anti-pattern. Should use sqlx bind parameters.
- **Remediation**: Rewrite using `sqlx::query(...).bind(val1).bind(val2)...` pattern.

### DEFECT-C18-4: Functions over 25-line Farley limit
- **Severity**: MEDIUM
- **File**: `crates/session/src/infrastructure/migration.rs`
- **Functions**: `migrate_sessions_table` (45 lines), `migrate_v2_add_branch_and_last_synced` (58 lines)
- **Remediation**: Extract connection validation and tracking table creation into helper functions.

### DEFECT-C18-5: `SessionRow` uses `String` for `session_state`
- **Severity**: LOW
- **File**: `crates/session/src/infrastructure/sqlite_session_repository.rs`, line 19
- **Description**: `SessionRow.session_state` is `String`, requiring conversion in `TryFrom`. Should parse at the row level, not later.
- **Remediation**: Store `SessionState` directly in the row struct, parsing at the database boundary.

---

## Bead hl-d3r (task command) -- STATUS: REJECTED

### DEFECT-D3R-1: CRITICAL -- Entire file is dead code
- **Severity**: CRITICAL
- **File**: `crates/cli/src/commands/handlers/task.rs` (all 970 lines)
- **Description**: The CLI dispatches to `commands::task::*` (from `commands/task.rs`), NOT `commands::handlers::task::*`. Verified in `cli/main.rs` lines 188-202. `run_task_command()` and `execute_task_command()` have zero production callers. The file duplicates logic from `commands/task.rs`.
- **Remediation**: Either delete this file entirely, or wire it into the CLI dispatch and delete `commands/task.rs`. Do not maintain two parallel task command implementations.

### DEFECT-D3R-2: File size violation (970 lines / 300 limit)
- **Severity**: HIGH
- **File**: `crates/cli/src/commands/handlers/task.rs`
- **Description**: File is 3.2x the 300-line architectural drift limit.
- **Remediation**: If this file is kept (after resolving DEFECT-D3R-1), split into submodules.

### DEFECT-D3R-3: `unwrap_or(0)` in `truncate_description`
- **Severity**: LOW
- **File**: `crates/cli/src/commands/handlers/task.rs`, line 302
- **Description**: The `.unwrap_or(0)` on `char_indices().take_while().last()` produces `desc[..0]` for edge cases, resulting in just "..." output. While not a panic risk, the fallback silently produces a degenerate result.
- **Remediation**: Handle the empty case explicitly: if `end == 0`, return "..." directly rather than slicing.

### DEFECT-D3R-4: `TaskCommand` uses raw `String` for IDs instead of newtypes
- **Severity**: MEDIUM
- **File**: `crates/cli/src/commands/handlers/task.rs`, lines 40-82
- **Description**: All `TaskCommand` variants use `String` for `task_id` and `agent_id`. Validation via `TaskId::new()` happens inside `execute_*` functions, violating "Parse Don't Validate at the boundary."
- **Remediation**: Define `TaskCommand` with `TaskId` and `AgentId` newtypes, parsing at construction time.

### DEFECT-D3R-5: `TaskStartOutput.status` is `String` instead of `TaskStatusOutput`
- **Severity**: LOW
- **File**: `crates/cli/src/commands/handlers/task.rs`, line 189
- **Description**: `TaskStartOutput.status: String` when `TaskStatusOutput` enum already exists.
- **Remediation**: Use the existing `TaskStatusOutput` type.
