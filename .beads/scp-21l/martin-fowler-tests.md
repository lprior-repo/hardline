# Martin Fowler Test Plan

## Happy Path Tests
- test_cargo_check_orchestrator_passes
  Given: The orchestrator crate source code
  When: Running `cargo check -p orchestrator`
  Then: Exit code is 0

- test_cargo_check_all_crates_passes
  Given: All crates in the workspace
  When: Running `cargo check`
  Then: Exit code is 0

- test_datetime_type_in_scope_in_phases_rs
  Given: The file `crates/orchestrator/src/phases.rs`
  When: Compiling with rustc
  Then: No "cannot find type `DateTime`" error

## Error Path Tests
- test_compilation_fails_without_datetime_import
  Given: Code using `DateTime<Utc>` without the import
  When: Compiling
  Then: Error E0425 "cannot find type `DateTime` in this scope"

- test_compilation_fails_with_unused_variable
  Given: Code with unused variable (e.g., `result` not prefixed with `_`)
  When: Compiling with `#![deny(unused_variables)]`
  Then: Compilation fails due to unused variable

## Edge Case Tests
- test_no_warnings_introduced
  Given: All source files
  When: Running `cargo check`
  Then: No warnings emitted

- test_no_dead_code_warnings
  Given: All source files
  When: Running `cargo check`
  Then: No dead_code warnings

## Contract Verification Tests
- test_precondition_p1_cargo_check_fails_before_fix
  Given: Current code state (before fix applied)
  When: Running `cargo check -p orchestrator`
  Then: Returns non-zero exit code (fails)

- test_postcondition_q1_cargo_check_passes_after_fix
  Given: Fixed code with `use chrono::DateTime;` import added
  When: Running `cargo check -p orchestrator`
  Then: Returns exit code 0

- test_postcondition_q2_no_new_warnings
  Given: Fixed code
  When: Running `cargo check`
  Then: No warnings emitted

- test_postcondition_q3_datetime_type_available
  Given: The function `record_spec_review_metrics` in phases.rs
  When: Compiling
  Then: `DateTime<Utc>` is successfully resolved

- test_invariant_i1_no_new_clippy_warnings
  Given: Fixed code
  When: Running `cargo clippy`
  Then: No new clippy warnings

- test_invariant_i2_no_compiler_warnings
  Given: Fixed code
  When: Running `cargo check`
  Then: No warnings (treated as errors)

- test_invariant_i3_existing_tests_compile
  Given: All test files
  When: Running `cargo test --no-run`
  Then: All tests compile successfully

## Contract Violation Tests
- test_violation_q1_cargo_check_returns_nonzero
  Given: Code before fix (missing import)
  When: Running `cargo check -p orchestrator`
  Then: Returns non-zero exit code (NOT zero)

- test_violation_q2_warnings_are_denied
  Given: Code with warnings
  When: Compiling with `#![deny(warnings)]`
  Then: Compilation fails (NOT succeeds)

- test_violation_q3_datetime_not_in_scope
  Given: Code using DateTime without import
  When: Compiling
  Then: Error "cannot find type `DateTime`" (NOT success)

## Given-When-Then Scenarios

### Scenario 1: Fix missing DateTime import
Given: The orchestrator crate has code using `DateTime<Utc>` but only `Utc` is imported from chrono
When: Running `cargo check -p orchestrator`
Then:
- Compilation fails with error E0425 "cannot find type `DateTime` in this scope"
- Error occurs in phases.rs line 363

### Scenario 2: Fix unused variable
Given: The orchestrator crate has unused variable `result` at line 575
When: Running `cargo check -p orchestrator`
Then:
- Compilation fails with unused_variables warning (treated as error)
- The variable is in a `.map(|result| Decision::Escalate)` closure

### Scenario 3: Full workspace compilation
Given: All crates in the workspace
When: Running `cargo check`
Then:
- Exit code is 0
- No warnings emitted
- All dependencies compile successfully
