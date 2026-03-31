# Red Queen Adversarial Test Report

Date: 2026-03-30
Targets: hl-9nb, hl-c18, hl-d3r

---

## Summary

| Bead  | Status   | Bugs Found | Severity |
|-------|----------|------------|----------|
| hl-9nb (ai.rs) | SURVIVED with vulnerabilities | 2 confirmed | Medium |
| hl-c18 (migration.rs) | **BROKEN** | 2 confirmed (1 data integrity, 1 logic) | High |
| hl-d3r (task.rs) | **BROKEN** | 4 confirmed | High |

**Total adversarial tests written**: 32 (cli) + 17 (session) = 49
**Tests exposing real bugs**: 8 (across all three beads)

---

## hl-9nb: AI Command (ai.rs)

### Status: SURVIVED with vulnerabilities

The pure calculation functions are structurally sound but have logic gaps and output injection weaknesses.

### Confirmed Vulnerabilities

#### VULN-9NB-1: Contradictory advice between `determine_ready_state` and `determine_next_action`

**Test**: `adversarial_ready_state_uninitialized_and_not_in_repo_suggests_init_not_cd`

When `initialized=false, location="not_in_repo"`:
- `determine_ready_state` returns `("SCP not initialized", "scp init")`
- `determine_next_action` returns `("Enter a JJ repository", "cd <repo> && scp init")`

The two functions give **contradictory guidance** for the same input state. An AI agent using `determine_ready_state` would try to `scp init` outside a repo, while one using `determine_next_action` would first try to enter a repo. This is a logic priority inversion -- the not_in_repo check should arguably take precedence since `scp init` is meaningless outside a repo.

**Severity**: Medium (confusing to AI agents, no crash risk)

**Fix**: Add `not_in_repo` check before `initialized` check in `determine_ready_state`, matching the priority order in `determine_next_action`.

#### VULN-9NB-2: Newline injection in `format_status_human`

**Tests**: `adversarial_format_status_human_newline_injection_in_suggestion`, `adversarial_format_status_human_newline_in_next_command`

The `format_status_human` function takes `AiStatusOutput` fields (suggestion, next_command) and directly formats them into output lines. If either field contains newlines, an attacker can inject forged status lines:

```rust
suggestion: "All good!\nStatus:         COMPROMISED"
next_command: "scp work test\n  rm -rf /"
```

The injected lines appear as legitimate output lines with proper alignment, making them indistinguishable from real status output.

**Severity**: Medium (output spoofing, could mislead AI agents)

**Fix**: Sanitize or escape newlines in suggestion/next_command fields before formatting.

### Survived Tests (no bugs found)

- `adversarial_ready_state_empty_location_falls_through_to_ready` - empty string location falls through to generic "Ready". This is debatable behavior, not clearly a bug.
- `adversarial_ready_state_extremely_long_location` - handles without issue
- `adversarial_format_session_count_zero_grammatically_acceptable` - "0 sessions" is grammatically fine
- `adversarial_format_session_count_max_usize` - handles without panic
- `adversarial_next_action_workspace_name_injection` - workspace name interpolated into action field unsanitized, but this is a display-only concern in a pure function
- `adversarial_envelope_schema_name_injection` - JSON serialization escapes newlines, safe
- `adversarial_status_output_serialization_extreme_sessions` - serializes fine

---

## hl-c18: Session Migration (migration.rs)

### Status: **BROKEN**

Two hard bugs confirmed: one data integrity issue, one input validation gap.

### Confirmed Bugs

#### BUG-C18-1: Orphaned migration tracking record causes PRIMARY KEY violation

**Test**: `adversarial_migration_version_with_orphaned_tracking_table`

**Reproduction**:
1. Create `schema_migrations` table directly
2. Insert version=1 record manually
3. Delete `sessions` table (simulating corruption)
4. Call `migrate_sessions_table()`

**Expected**: Self-heal by recreating the sessions table.
**Actual**: **Fails with PRIMARY KEY violation** because it tries to INSERT version=1 again (already exists from step 2).

The `migrate_sessions_table` function only checks if the `sessions` table exists to decide whether to skip, but it doesn't check whether the tracking record already exists before inserting. If the tracking table has a stale record but sessions doesn't exist, the migration fails.

```rust
// Line 217-221: Only checks sessions table, not tracking record
let sessions_exists = table_exists(pool, "sessions").await?;
if sessions_exists {
    return Ok(());  // Only this early-exit is safe
}

// Line 233-241: Blindly inserts version=1 without checking for duplicates
sqlx::query(sql::INSERT_MIGRATION)
    .bind(version.as_i64())  // Always 1
    .bind("create_sessions_table")
    .execute(pool)
    .await  // PANICS on duplicate PRIMARY KEY
```

**Severity**: High (migration becomes un-runnable after tracking/sessions desync)

**Fix**: Before inserting the migration record, check if version 1 already exists in `schema_migrations`. Use `INSERT OR IGNORE` or check-and-insert pattern.

#### BUG-C18-2: `validate_migration_name` accepts Unicode characters

**Test**: `adversarial_migration_name_unicode_rejected`

The validation function uses `char::is_alphanumeric()` which returns `true` for all Unicode letters/digits, not just ASCII:

```rust
// Line 161
&& name.chars().all(|c| c.is_alphanumeric() || c == '_');
```

This allows names like "migracion" (with accented o) and CJK characters. While SQLite itself supports Unicode identifiers, the function's documented contract says "must be valid SQL identifier (alphanumeric, underscore only)" which implies ASCII-only.

**Severity**: Medium (surprising behavior, potential downstream issues)

**Fix**: Change to `c.is_ascii_alphanumeric()` to match the documented contract.

### Survived Tests (no bugs found)

- `adversarial_table_exists_sql_injection_attempt` - SQLite's string quoting prevents injection here, but the raw format! pattern is still risky
- `adversarial_table_exists_drop_injection` - Same, SQLite protects against this
- `adversarial_rollback_then_reapply_v2` - Rollback + re-apply cycle works correctly
- `adversarial_v2_double_migration_preserves_data` - Idempotent, data safe
- `adversarial_double_rollback_fails` - Correctly fails on double rollback
- `adversarial_rapid_repeated_migrations` - No duplicate records after 10 runs
- `adversarial_migrate_with_version_ignores_custom_version` - API is misleading but not broken
- All CHECK/UNIQUE/PRIMARY KEY constraint tests pass (schema is well-defined)
- `adversarial_column_exists_nonexistent_table` - Handles gracefully

### Security Note: `table_exists` SQL Format Pattern

While the current tests show SQLite protects against injection, the pattern at line 174-175 is still a code smell:

```rust
let sql = format!(
    "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='{}';",
    table_name
);
```

The `table_name` parameter comes from hardcoded constants in the current code, but if it ever accepts external input, this would be exploitable. Recommend using parameterized queries even for internal use.

---

## hl-d3r: Task Command (task.rs)

### Status: **BROKEN**

Four confirmed bugs, two critical validation gaps and two state machine violations.

### Confirmed Bugs

#### BUG-D3R-1: Two-phase validation gap between `validate_task_command` and `TaskId::new`

**Test**: `adversarial_validate_allows_special_chars_that_fail_taskid`

`validate_task_command` only checks that IDs are non-empty (after trimming). But `TaskId::new` enforces a strict regex (`^[a-zA-Z0-9_-]+$`). This means IDs like `"task!/script"`, `"task with spaces"`, and `"../../../etc/passwd"` pass validation but fail later with a different error message during execution.

The user sees:
- Expected: `"Task ID cannot be empty for show"` (from validation)
- Actual: `"Task ID must be alphanumeric with - or _, got: task!/script"` (from TaskId)

This creates an inconsistent error experience and means `validate_task_command` provides a false sense of security.

**Severity**: High (validation bypass, confusing error messages)

**Fix**: Move the `TaskId` regex validation into `validate_id_non_empty`, or at minimum check the same pattern.

#### BUG-D3R-2: Empty/whitespace-only agent_id passes validation

**Tests**: `adversarial_empty_agent_id_passes_validation`, `adversarial_whitespace_agent_id_passes_validation`

`validate_task_command` does not validate `agent_id` at all. An empty string or whitespace-only agent_id passes validation and would be used in claim/yield/start/done operations, creating tasks assigned to `""` or `"   "`.

**Severity**: Medium (data quality issue)

**Fix**: Add `validate_id_non_empty` check for `agent_id` fields.

#### BUG-D3R-3: State transitions are unguarded -- can resurrect Closed tasks

**Tests**: `adversarial_yield_transitions_closed_to_open`, `adversarial_start_transitions_closed_to_in_progress`, `adversarial_claim_transitions_closed_to_in_progress`

The `transition_to_*` pure functions do not validate the current state. They will happily:
- `transition_to_yielded(closed_task)` -> task becomes Open again
- `transition_to_started(closed_task)` -> task becomes InProgress again
- `transition_to_claimed(closed_task, "new_agent")` -> assignee changes on closed task

While the action handlers (`execute_claim`, `execute_done`) call `validate_not_closed`, the pure transition functions themselves are unsafe. Any caller that forgets the validation step can corrupt task state.

**Severity**: High (state machine integrity)

**Fix**: Add state guards to `transition_to_*` functions that return `Result<Task>` instead of `Task`, rejecting invalid source states. Alternatively, document the precondition contract and make the functions `unsafe`-by-convention with clear naming.

#### BUG-D3R-4: `truncate_description` produces output longer than `max_len` when max_len < 3

**Tests**: `adversarial_truncate_max_len_one`, `adversarial_truncate_max_len_two`

When `max_len < 3`, the function produces `"..."` (3 characters), which is longer than the requested maximum. For example:
- `truncate_description("hello", 1)` returns `"..."` (3 chars, exceeding max of 1)
- `truncate_description("hello", 2)` returns `"..."` (3 chars, exceeding max of 2)

**Severity**: Low (cosmetic, max_len < 3 is an unusual call site)

**Fix**: For `max_len < 3`, return an empty string or clamp to `max_len` without the `"..."` suffix.

### Survived Tests (no bugs found)

- `adversarial_validate_allows_whitespace_padded_id_that_fails_taskid` - the whitespace-padded ID `" valid-id "` has content after trim, so validation passes, but `TaskId::new` rejects the spaces. This is the same class as BUG-D3R-1.
- `adversarial_truncate_multi_byte_chars_at_boundary` - correctly handles multi-byte chars
- `adversarial_truncate_max_len_smaller_than_char` - produces `"..."` as expected
- `adversarial_truncate_zero_max_len` - produces `"..."` as expected
- `adversarial_truncate_max_len_three` - correct
- `adversarial_double_close_produces_different_timestamp` - no crash, timestamps monotonic
- `adversarial_resolve_task_id_whitespace_only_falls_through` - correctly falls to env var
- `adversarial_taskid_boundary_single_char` - all valid
- `adversarial_taskid_extremely_long` - accepted
- `adversarial_filter_by_empty_status_returns_nothing` - correct
- `adversarial_filter_by_unicode_status` - correctly returns nothing

---

## Recommended Fixes (Priority Order)

1. **BUG-D3R-1**: Align `validate_task_command` with `TaskId::new` regex
2. **BUG-C18-1**: Use `INSERT OR IGNORE` or check-before-insert for migration tracking
3. **BUG-D3R-3**: Add state guards to `transition_to_*` functions
4. **BUG-D3R-2**: Validate `agent_id` in `validate_task_command`
5. **BUG-C18-2**: Change `is_alphanumeric()` to `is_ascii_alphanumeric()`
6. **VULN-9NB-1**: Fix priority order in `determine_ready_state`
7. **VULN-9NB-2**: Sanitize newlines in `format_status_human`
8. **BUG-D3R-4**: Handle `max_len < 3` in `truncate_description`

---

## Test Inventory

### ai.rs adversarial tests (12 tests)
- `adversarial_ready_state_uninitialized_and_not_in_repo_suggests_init_not_cd`
- `adversarial_ready_state_vs_next_action_consistency`
- `adversarial_ready_state_empty_location_falls_through_to_ready`
- `adversarial_ready_state_extremely_long_location`
- `adversarial_ready_state_unicode_location`
- `adversarial_format_session_count_zero_grammatically_acceptable`
- `adversarial_format_session_count_max_usize`
- `adversarial_format_status_human_newline_injection_in_suggestion`
- `adversarial_format_status_human_newline_in_next_command`
- `adversarial_next_action_huge_session_count`
- `adversarial_next_action_workspace_name_injection`
- `adversarial_envelope_schema_name_injection`
- `adversarial_status_output_serialization_extreme_sessions`

### task.rs adversarial tests (20 tests)
- `adversarial_validate_allows_whitespace_padded_id_that_fails_taskid`
- `adversarial_validate_allows_special_chars_that_fail_taskid`
- `adversarial_empty_agent_id_passes_validation`
- `adversarial_whitespace_agent_id_passes_validation`
- `adversarial_truncate_multi_byte_chars_at_boundary`
- `adversarial_truncate_max_len_smaller_than_char`
- `adversarial_truncate_zero_max_len`
- `adversarial_truncate_max_len_one`
- `adversarial_truncate_max_len_two`
- `adversarial_truncate_max_len_three`
- `adversarial_filter_by_empty_status_returns_nothing`
- `adversarial_yield_transitions_closed_to_open`
- `adversarial_start_transitions_closed_to_in_progress`
- `adversarial_claim_transitions_closed_to_in_progress`
- `adversarial_double_close_produces_different_timestamp`
- `adversarial_resolve_task_id_whitespace_only_falls_through`
- `adversarial_taskid_boundary_single_char`
- `adversarial_taskid_extremely_long`
- `adversarial_filter_by_unicode_status`

### migration.rs adversarial tests (17 tests)
- `adversarial_table_exists_sql_injection_attempt`
- `adversarial_table_exists_drop_injection`
- `adversarial_rollback_then_reapply_v2`
- `adversarial_v2_double_migration_preserves_data`
- `adversarial_v2_without_v1_error_type`
- `adversarial_migration_version_with_orphaned_tracking_table`
- `adversarial_migration_name_unicode_rejected`
- `adversarial_migration_name_sql_comment_rejected`
- `adversarial_migration_version_max_i64`
- `adversarial_rapid_repeated_migrations`
- `adversarial_double_rollback_fails`
- `adversarial_migrate_with_version_ignores_custom_version`
- `adversarial_insert_invalid_status_rejected`
- `adversarial_insert_invalid_state_rejected`
- `adversarial_duplicate_session_name_rejected`
- `adversarial_duplicate_session_id_rejected`
- `adversarial_column_exists_nonexistent_table`
