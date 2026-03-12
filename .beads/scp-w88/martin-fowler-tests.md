# Martin Fowler Test Plan

## Overview
This test plan covers the compilation fix for the scenarios crate. Since this is a build-time fix (not runtime), tests verify that the crate compiles successfully with no warnings when built standalone.

## Happy Path Tests
- test_scenarios_crate_compiles_successfully
  - Given: scenarios crate with fixed Cargo.toml (hardcoded values)
  - When: `cargo check` is run in the crate directory
  - Then: Exit code is 0, no errors printed

- test_scenarios_crate_has_no_warnings
  - Given: scenarios crate compiles successfully
  - When: `cargo check` output is inspected
  - Then: No warning messages are present

- test_all_scenarios_modules_are_valid_rust
  - Given: All source files (lib.rs, runner.rs, scenario.rs, sanitizer.rs)
  - When: Each file is checked individually
  - Then: Each compiles without errors

- test_scenarios_lib_exports_public_types
  - Given: Compiled scenarios crate
  - When: Checking public exports
  - Then: RunnerConfig, ScenarioResult, ScenarioRunner, StepResult, FeedbackLevel, Sanitizer, Scenario, Step, AssertStep, AssertionType, ExtractStep, HttpMethod, HttpStep are all exported

## Error Path Tests
- test_fails_without_package_name
  - Given: Cargo.toml missing [package] section
  - When: cargo check is run
  - Then: Returns error about missing package name

- test_fails_without_edition
  - Given: Cargo.toml missing edition field
  - When: cargo check is run
  - Then: Returns error about missing edition

- test_fails_with_missing_dependencies
  - Given: Cargo.toml missing a required dependency (e.g., serde)
  - When: cargo check is run
  - Then: Returns error about unresolved dependency

- test_fails_with_workspace_inheritance_outside_workspace
  - Given: Original Cargo.toml with version.workspace = true
  - When: cargo check is run outside workspace context
  - Then: Returns "error inheriting" from workspace root

## Edge Case Tests
- test_compiles_with_minimal_rust_version
  - Given: Cargo.toml specifies rust-version = "1.80"
  - When: Building with Rust 1.80+
  - Then: Compiles successfully

- test_workspace_inheritance_removed
  - Given: Cargo.toml using version.workspace = true
  - When: Building outside workspace
  - Then: Should either work (if values hardcoded) or fail gracefully

- test_excluded_crate_builds_independently
  - Given: scenarios crate excluded from workspace
  - When: Built as standalone crate
  - Then: Compiles successfully without workspace

- test_cargo_lock_not_required
  - Given: Fresh checkout without Cargo.lock
  - When: Running cargo check
  - Then: Generates Cargo.lock and compiles successfully

## Contract Verification Tests
- test_precondition_p1_workspace_inheritance_fails
  - Given: Original Cargo.toml with workspace.workspace = true inheritance
  - When: cargo check is run outside workspace
  - Then: Returns error about inheriting from workspace

- test_postcondition_q1_compilation_succeeds
  - Given: Fixed Cargo.toml with hardcoded values
  - When: cargo check is run
  - Then: Exit code 0

- test_postcondition_q2_no_warnings
  - Given: Fixed Cargo.toml
  - When: cargo check is run with warnings treated as errors
  - Then: No warnings in output

- test_postcondition_q3_no_workspace_inheritance
  - Given: Fixed Cargo.toml
  - When: Grepping for ".workspace = true" patterns
  - Then: No matches found in [package] section

- test_postcondition_q4_standalone_build
  - Given: Fixed Cargo.toml
  - When: Removing parent workspace directory (or simulating)
  - Then: Still compiles successfully

- test_invariant_i1_warnings_denied
  - Given: lib.rs has #![deny(warnings)]
  - When: crate compiles
  - Then: Any warning would cause build failure

- test_invariant_i3_sanitizer_unchanged
  - Given: Compiled sanitizer module
  - When: Checking FeedbackLevel enum variants
  - Then: Level1 through Level5 all present

## Contract Violation Tests
- VIOLATION test_p1_workspace_inherit_error
  - Given: Original broken Cargo.toml
  - When: `cargo check 2>&1`
  - Then: Returns non-zero exit code with "error inheriting" message

- VIOLATION test_q1_compilation_failure
  - Given: Broken scenarios crate
  - When: `cargo check`
  - Then: Returns non-zero exit code

- VIOLATION test_q2_warnings_present
  - Given: Crate that would produce warnings if allowed
  - When: Built with warnings enabled
  - Then: Warnings appear in output (but should be denied)

- VIOLATION test_q3_workspace_inheritance_still_present
  - Given: Fixed Cargo.toml
  - When: Checking for ".workspace = true" in [package]
  - Then: Should find none (violation if found)

- VIOLATION test_p2_missing_dependency_error
  - Given: Cargo.toml missing a required dependency
  - When: cargo check is run
  - Then: Returns error about unresolved dependency

- VIOLATION test_q4_standalone_build_fails
  - Given: Fixed Cargo.toml
  - When: Building without workspace context
  - Then: Should compile successfully (violation if it fails)

## Given-When-Then Scenarios

### Scenario 1: Standalone Compilation
Given: Developer clones repository and runs `cd crates/scenarios && cargo check`
When: The scenarios crate has proper self-sufficient Cargo.toml
Then:
- Build succeeds with exit code 0
- No warnings are printed
- All module files are validated

### Scenario 2: CI Pipeline
Given: CI runs `cargo check` on scenarios crate in isolation
When: The scenarios crate is built standalone
Then:
- Compilation completes successfully
- No dependency resolution errors
- No missing package errors

### Scenario 3: Warning-Free Build
Given: scenarios crate with #![deny(warnings)] in lib.rs
When: Building with `RUSTFLAGS="-Dwarnings"`
Then:
- Build succeeds
- Zero warnings emitted
- All lint checks pass

### Scenario 4: Type Annotation Verification
Given: All public functions and structs in scenarios crate
When: Reviewing the code
Then:
- All `pub` items have explicit return types where beneficial
- Complex closures have explicit type annotations
- Generic bounds are explicit

### Scenario 5: Information Barrier Preserved
Given: Scenarios crate compiled and sanitizer module imported
When: Using FeedbackLevel enum
Then:
- All five levels (Level1-Level5) are available
- Sanitizer struct can be instantiated with any level
- sanitize_result method returns String

## Test Execution Commands

```bash
# Direct compilation test
cd /home/lewis/src/scp/crates/scenarios && cargo check

# With warnings as errors
cd /home/lewis/src/scp/crates/scenarios && RUSTFLAGS="-Dwarnings" cargo check

# Check specific modules
cd /home/lewis/src/scp/crates/scenarios && cargo check --lib

# Run tests if they exist
cd /home/lewis/src/scp/crates/scenarios && cargo test

# Verify no workspace inheritance in package section
grep -E '\.workspace\s*=\s*true' /home/lewis/src/scp/crates/scenarios/Cargo.toml
```

## Success Criteria Summary

| Test | Criterion |
|------|-----------|
| Happy Path | cargo check returns 0 |
| Happy Path | No warnings in output |
| Happy Path | All modules valid |
| Happy Path | All public types exported |
| Error Path | Missing config detected |
| Error Path | Missing deps detected |
| Error Path | Workspace inheritance fails standalone |
| Edge Case | Standalone build works |
| Edge Case | Min version supported |
| Edge Case | Cargo.lock not required |
| Contract | Precondition violation documented |
| Contract | Postcondition satisfied |
| Contract | Invariants maintained |
| Contract | Violation tests match contract examples |
