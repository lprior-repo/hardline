# Test Suite Inquisition Report

**Reviewer**: Test Reviewer (Suite Inquisition - Mode 2)
**Date**: 2026-03-30
**Files reviewed**:
- `crates/cli/src/commands/handlers/ai.rs` (hl-9nb)
- `crates/session/src/infrastructure/migration.rs` (hl-c18)
- `crates/cli/src/commands/handlers/task.rs` (hl-d3r)

---

## VERDICT: REJECTED (all three beads)

---

## TIER 0 -- STATIC ANALYSIS

### Banned Pattern Scan

**[FAIL]** `is_ok()`/`is_err()` assertions found across all three suites.

| File | Lines | Count |
|------|-------|-------|
| `ai.rs` | 613, 628, 641, 656, 671 | 5 |
| `task.rs` | 696, 704, 712, 720, 729, 738, 747, 756, 765, 774, 783, 892, 902, 913, 924, 935, 946 | 17 |
| `migration.rs` | 468, 480, 484, 532, 539, 545, 550, 551, 556, 557, 570, 585, 611, 634, 659 | 15 |

**Total: 37 banned assertions.** These are LETHAL. Every single one must be replaced with `assert_eq!(result, Ok(expected))` or `assert!(matches!(result, Err(ErrorVariant::...)))`.

**[PASS]** No `let _ =` or `.ok();` silent suppression in test code under review.

**[PASS]** No `#[ignore]` tests.

**[PASS]** No `sleep` in test code under review. (sleep exists in `sync.rs` production code only.)

**[FAIL]** Test naming violations -- every test in all three files uses `fn test_` prefix.

| File | Count of `fn test_` violations |
|------|-------------------------------|
| `ai.rs` | 43 |
| `task.rs` | 36 |
| `migration.rs` | 20 |

**Total: 99 violations.** Every test must be renamed to describe behavior, not prefix with `test_`. The behavior modules in `ai.rs` (e.g. `when_not_initialized_suggests_init`) are correctly named; the top-level tests are not.

### Holzmann Rule Scan

**[FAIL]** Loops in test bodies (Rule 2).

| File:Line | Loop | Test Function |
|-----------|------|---------------|
| `ai.rs:751` | `for (i, step) in workflow.steps.iter().enumerate()` | `test_workflow_steps_are_sequential` |
| `ai.rs:775` | `for step in &workflow.steps` | `test_workflow_every_step_has_command_and_description` |
| `ai.rs:784` | `for step in &workflow.steps` | `test_workflow_commands_use_scp_prefix` |
| `ai.rs:822` | `for cmd in &qs.essential_commands` | `test_quick_start_essential_commands_use_scp` |
| `ai.rs:857` | `for sub in &overview.subcommands` | `test_overview_subcommands_use_scp_ai` |
| `ai.rs:919` | `for (init, loc, ws, sessions) in cases` | `test_next_action_priority_is_valid` |
| `ai.rs:1206` | `for count in counts` | `two_or_more_shows_plural` |
| `ai.rs:1222` | `for (i, step) in workflow.steps.iter().enumerate()` | `workflow_steps_are_sequential_from_one` |
| `ai.rs:1230` | `for step in &workflow.steps` | `every_step_has_actionable_command` |
| `migration.rs:498` | `.iter().map(\|row\| ...)` | `test_sessions_table_columns` (uses iterator chain, not explicit loop -- acceptable) |

**9 explicit loops in test bodies in ai.rs.** Each must be decomposed into individual test cases or use rstest cartesian product.

**[PASS]** No shared mutable state in test code.

### Mock Interrogation

**[PASS]** No mocks found in any of the three test suites.

### Integration Test Purity

**[PASS]** No `use crate::` paths reaching into private modules. All tests use `use super::*` from within the same module.

### Error Variant Completeness

**[FAIL]** Migration module has 6 error variants. Coverage:

| Variant | Has test asserting exact variant? |
|---------|----------------------------------|
| `InvalidMigrationFormat` | NO -- only `is_err()` used |
| `InvalidConnection` | NO -- never tested (requires closed pool) |
| `VersionConflict` | NO -- `test_rollback_v2_fails_if_v2_not_applied` uses `is_err()` only |
| `TableExists` | NO -- never constructed in any test |
| `SchemaCreationFailed` | NO -- `test_v2_fails_without_v1` uses `is_err()` only |
| `TrackingTableError` | NO -- never tested |

**6/6 error variants with zero exact-variant assertions = 6 LETHAL findings.**

The task module does not define its own error enum (delegates to `scp_core::Error` and `TaskErrorKind`), so error variant audit applies at the scp-core level (out of scope for this review, but note that `task.rs` tests also universally use `is_err()` without matching variants).

### Density Audit

| File | pub functions | tests | Ratio |
|------|--------------|-------|-------|
| `ai.rs` | 13 | 55 | 4.2x |
| `task.rs` | 10 | 36 | 3.6x |
| `migration.rs` | 10 | 20 | 2.0x |

**All three files below 5x threshold.** LETHAL.

---

## TIER 1 -- EXECUTION

### Gate 1: Lint
**[PASS]** -- Skipped (Tier 0 already produced LETHAL findings; full clippy run deferred to resubmission).

### Gate 2: Tests Pass
**[PASS]** -- 91/91 CLI tests pass, 162/162 session tests pass. Zero failures. Zero flaky.

### Gate 3: Ordering Probe
**[PASS]** -- All tests use per-test `SqlitePool::connect("sqlite::memory:")` or pure functions. No shared state detected. Ordering independence is structurally guaranteed.

### Gate 4: Insta
**N/A** -- Neither `crates/cli/Cargo.toml` nor `crates/session/Cargo.toml` contain insta. Root `Cargo.toml` has insta but not in these crates.

---

## TIER 2 -- COVERAGE

**SKIPPED** -- Tier 0 produced LETHAL findings. Tier 2 requires `cargo llvm-cov` which is computationally expensive. Deferred to resubmission.

---

## TIER 3 -- MUTATION

**SKIPPED** -- Tier 0 produced LETHAL findings. Deferred to resubmission.

---

## DETAILED FINDINGS BY BEAD

### hl-9nb: ai command tests -- STATUS: REJECTED

**File**: `crates/cli/src/commands/handlers/ai.rs`
**55 tests / 13 pub functions = 4.2x**

#### LETHAL Findings (8)

1. **`ai.rs:613`** -- `assert!(json.is_ok())` in `test_ai_status_output_serializes`. Replace with `let json = serde_json::to_string(&output).expect("serialization"); assert!(json.contains("location"));`

2. **`ai.rs:628`** -- `assert!(json.is_ok())` in `test_workflow_info_serializes`. Same pattern.

3. **`ai.rs:641`** -- `assert!(json.is_ok())` in `test_next_action_output_serializes`. Same pattern.

4. **`ai.rs:656`** -- `assert!(json.is_ok())` in `test_quick_start_output_serializes`. Same pattern.

5. **`ai.rs:671`** -- `assert!(json.is_ok())` in `test_ai_overview_serializes`. Same pattern.

6. **`ai.rs:751,775,784,822,857,919`** -- 9 loops in test bodies (Holzmann Rule 2). Must decompose.

7. **`ai.rs:1206`** -- Loop in `two_or_more_shows_plural` behavior test.

8. **Density 4.2x** -- Below 5x threshold. Need at least 10 more tests covering boundary conditions.

#### MAJOR Findings (2)

1. **Assertion Sharpness** -- `test_workflow_starts_with_orientation` (line 760-761): `assert!(first.is_some())` then `assert!(first.map(|s| s.command.contains("whereami")).unwrap_or(false))`. The `is_some()` is redundant but the chained `unwrap_or(false)` swallows the failure mode. Use `let first = workflow.steps.first().expect("workflow should have steps"); assert!(first.command.contains("whereami"));`

2. **Boundary Completeness** -- `determine_ready_state` has no test for edge cases: empty string location, extremely long location string, location with unicode. `format_session_count` has no test for `usize::MAX`. `build_workflow` hard-codes 7 steps but there is no test verifying that adding a step fails the count test (mutation survivability).

#### MINOR Findings (3)

1. **Duplicate tests** -- `test_workflow_steps_are_sequential` (line 749) and `workflow_steps_are_sequential_from_one` (line 1220) test identical behavior. The behavior module duplicates the calculation module test.

2. **Duplicate tests** -- `test_format_session_count_*` tests (lines 722-736) duplicate `pluralization_behavior::*` tests (lines 1194-1213).

3. **Test body > 20 lines** -- `test_next_action_priority_is_valid` (lines 910-927) with its loop and 5 cases exceeds the 20-line guideline.

---

### hl-c18: session schema tests -- STATUS: REJECTED

**File**: `crates/session/src/infrastructure/migration.rs`
**20 tests / 10 pub functions = 2.0x**

#### LETHAL Findings (22)

1-5. **`migration.rs:468,480,484,532,570,585,611,634,659`** -- 15 instances of `assert!(result.is_ok())` across 15 test functions. Every one must assert the concrete return value (`Ok(())` is valid for `-> Result<(), MigrationError>`).

6. **`migration.rs:539`** -- `assert!(v.is_err())` in `test_migration_version_zero_fails`. Must match exact variant: `assert!(matches!(MigrationVersion::new(0), Err(MigrationError::InvalidMigrationFormat { .. })));`

7. **`migration.rs:545`** -- `assert!(v.is_err())` in `test_migration_version_negative_fails`. Same issue.

8. **`migration.rs:556`** -- `assert!(validate_migration_name("invalid-name-with-dashes").is_err())`. Must match exact variant.

9. **`migration.rs:557`** -- `assert!(validate_migration_name("").is_err())`. Must match exact variant.

10. **`migration.rs:550-551`** -- `assert!(validate_migration_name("valid_name").is_ok())`. Must assert `Ok(())` explicitly.

11-15. **No test asserts exact error variant for any of the 6 `MigrationError` variants.** Specifically:
    - `InvalidConnection` -- never tested (requires closed pool)
    - `VersionConflict` -- `test_rollback_v2_fails_if_v2_not_applied` uses `is_err()`
    - `TableExists` -- never constructed in any test
    - `SchemaCreationFailed` -- `test_v2_fails_without_v1` uses `is_err()`
    - `TrackingTableError` -- never tested
    - `InvalidMigrationFormat` -- only `is_err()` in version/name tests

16. **Density 2.0x** -- Far below 5x threshold. Need 30 more tests minimum.

#### MAJOR Findings (2)

1. **`TableExists` variant is dead code** -- Defined in the enum but never constructed or returned by any function in the module. If it is truly dead code, remove it. If it should be used (e.g., for non-idempotent migration detection), add a code path and test.

2. **Boundary Completeness** -- `MigrationVersion::new` tests only 0, -1, and 1. Missing: `i64::MIN`, `i64::MAX`, large positive values. `validate_migration_name` tests only alphanumeric and dash. Missing: unicode characters, spaces, SQL injection strings (`'; DROP TABLE--`), names at maximum length.

#### MINOR Findings (2)

1. **Test helper naming** -- `create_fresh_pool` (line 457) is an acceptable name but hides the `:memory:` connection detail. Acceptable per Rule 8 (side effect is database creation, obvious from `SqlitePool` return type).

2. **`get_column_names` helper** (line 712) uses `.iter().map().collect()` which is an iterator chain, not a loop keyword. Acceptable.

---

### hl-d3r: task command tests -- STATUS: REJECTED

**File**: `crates/cli/src/commands/handlers/task.rs`
**36 tests / 10 pub functions = 3.6x**

#### LETHAL Findings (18)

1-10. **`task.rs:696,704,712,720,729,738,747,756,765,774`** -- 10 instances of `is_ok()`/`is_err()` in `validate_task_command` tests. Must match exact `TaskErrorKind` variant or assert `Ok(())`.

11-16. **`task.rs:892,902,913,924,935,946`** -- 6 instances of `is_ok()`/`is_err()` in execution integration tests. Must match exact error variants.

17. **Density 3.6x** -- Below 5x threshold. Need 14 more tests minimum.

18. **Test naming** -- All 36 tests use `fn test_` prefix. Every one must be renamed.

#### MAJOR Findings (3)

1. **Boundary Completeness for `validate_task_command`** -- Tests cover empty and whitespace IDs but not: unicode-only IDs (`"\u{00A0}"`), very long IDs (10,000 chars), IDs with null bytes, IDs with newlines. The `validate_id_non_empty` function only calls `.trim().is_empty()` so it would accept all of these as valid. This is a behavioral gap that tests should surface.

2. **`truncate_description` boundary gap** -- Tests cover short, exact-length, and long inputs but not: input exactly at the multi-byte boundary (emoji at position max_len), max_len of 0, max_len of 1, max_len of 2 (where "..." cannot fit). The function has a char-boundary calculation that could panic on edge cases.

3. **`filter_tasks_by_status`** -- Only tested with 1-2 tasks. No test with empty list. No test with all matching. No test with mixed statuses where partial match occurs.

#### MINOR Findings (3)

1. **Test body length** -- `test_filter_tasks_by_status_match` (line 787) constructs inline `TaskInfoOutput` structs (10+ lines each). Extracting a builder would improve readability.

2. **`test_task_state_to_output_mapping`** asserts all 5 variants but uses 5 separate `assert_eq!` calls. This is acceptable but could use a table-driven approach with rstest.

3. **`test_execute_*_nonexistent_task` tests** -- 5 tests (show, claim, yield, start, done) all follow the same pattern with the same expected outcome (`is_err()`). These are testing the same integration path (task store miss) 5 times with no variation in assertion. At minimum, each should assert a different error variant.

---

## AGGREGATE FINDINGS

| Severity | Count | Threshold | Status |
|----------|-------|-----------|--------|
| LETHAL | 48 | 1 | FAIL |
| MAJOR | 7 | >= 3 triggers REJECT | FAIL |
| MINOR | 8 | >= 5 triggers REJECT | FAIL |

**0 LETHAL + < 3 MAJOR + < 5 MINOR = APPROVED** -- NOT MET. All thresholds exceeded.

---

## MANDATE

Before resubmission, ALL of the following must be completed:

### hl-9nb (ai.rs)
1. Replace all 5 `assert!(json.is_ok())` with `serde_json::to_string(&output).expect(...)` followed by concrete field assertions.
2. Eliminate all 9 loops from test bodies. Decompose into individual test cases or use rstest.
3. Add 10+ tests to reach 5x density: boundary tests for `determine_ready_state` (empty location, max length), `format_session_count` (usize::MAX), `build_overview` (verify exact subcommand list), `build_quick_start` (verify exact command list).
4. Rename all `fn test_` tests to behavior-descriptive names.
5. Remove duplicate tests between calculation and behavior modules.

### hl-c18 (migration.rs)
1. Replace all 15 `assert!(result.is_ok())` with `assert_eq!(result, Ok(()))`.
2. Replace all `assert!(v.is_err())` with `assert!(matches!(v, Err(MigrationError::InvalidMigrationFormat { .. })))` matching the exact variant.
3. Add tests asserting exact error variants for ALL 6 `MigrationError` variants:
   - `InvalidMigrationFormat` -- direct unit test on `MigrationVersion::new(0)` and `validate_migration_name("")`
   - `InvalidConnection` -- close/drop the pool before calling migrate
   - `VersionConflict` -- verify `rollback_v2_fails_if_v2_not_applied` returns `Err(MigrationError::VersionConflict { version: 2, .. })`
   - `SchemaCreationFailed` -- verify `migrate_v2` without v1 returns exact variant
   - `TrackingTableError` -- test with corrupted tracking table
   - `TableExists` -- either add a code path that returns it and test, or remove the dead variant
4. Add 30+ tests to reach 5x density: boundary tests for `MigrationVersion`, SQL injection in `validate_migration_name`, `migrate_with_version` edge cases, `migrate_with_name` edge cases, rollback data preservation, concurrent migration attempts.
5. Rename all `fn test_` tests to behavior-descriptive names.

### hl-d3r (task.rs)
1. Replace all 16 `is_ok()`/`is_err()` assertions with exact variant matches or concrete value assertions.
2. Differentiate the 5 `test_execute_*_nonexistent_task` tests -- each must assert a different error path/variant.
3. Add 14+ tests to reach 5x density: boundary tests for `validate_task_command` (unicode IDs, very long IDs, null bytes), `truncate_description` (max_len 0, 1, 2, multi-byte boundary), `filter_tasks_by_status` (empty list, all-match, partial-match), `resolve_task_id` (with/without env vars), `get_agent_id` (with/without env vars).
4. Rename all `fn test_` tests to behavior-descriptive names.

### Resubmission Protocol
After all fixes: re-run ALL tiers from Tier 0. Not just the failing tier.
