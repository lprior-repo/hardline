
package validation

import "list"

// Validation schema for bead: hardline-20260320171630-wl25bfam
// Title: cli: implement done command
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320171630-wl25bfam.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320171630-wl25bfam"
  title: "cli: implement done command"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The current context must be a valid isolated workspace",
      "The workspace state must be 'Working'",
      "No uncommitted or untracked files are present (clean working tree)",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Workspace changes are merged into the main branch",
      "Workspace state transitions to 'Merged'",
      "The isolated workspace directory may be cleaned up",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "The main branch history is preserved",
      "No merge conflicts are silently resolved",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(2)
    error_path_tests: [...string] & list.MinItems(3)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Successfully complete and merge a clean workspace",
      "Placeholder happy test for schema compliance",
    ]

    // Required error path tests
    required_error_tests: [
      "Fails if workspace is not in Working state",
      "Fails if there are uncommitted changes",
      "Handles merge conflicts gracefully by transitioning to Conflict state",
    ]
  }

  // Code completion
  code_complete: {
    implementation_exists: string  // Path to implementation file
    tests_exist: string  // Path to test file
    ci_passing: bool & true
    no_unwrap_calls: bool & true  // Rust/functional constraint
    no_panics: bool & true  // Rust constraint
  }

  // Completion criteria
  completion: {
    all_sections_complete: bool & true
    documentation_updated: bool
    beads_closed: bool
    timestamp: string  // ISO8601 completion timestamp
  }
}

// Example implementation proof - create this file to validate completion:
//
// implementation.cue:
// package validation
//
// implementation: #BeadImplementation & {
//   contracts_verified: {
//     preconditions_checked: true
//     postconditions_verified: true
//     invariants_maintained: true
//     precondition_checks: [/* documented checks */]
//     postcondition_checks: [/* documented verifications */]
//     invariant_checks: [/* documented invariants */]
//   }
//   tests_passing: {
//     all_tests_pass: true
//     happy_path_tests: ["test_version_flag_works", "test_version_format", "test_exit_code_zero"]
//     error_path_tests: ["test_invalid_flag_errors", "test_no_flags_normal_behavior"]
//   }
//   code_complete: {
//     implementation_exists: "src/main.rs"
//     tests_exist: "tests/cli_test.rs"
//     ci_passing: true
//     no_unwrap_calls: true
//     no_panics: true
//   }
//   completion: {
//     all_sections_complete: true
//     documentation_updated: true
//     beads_closed: false
//     timestamp: "2026-03-20T17:16:30Z"
//   }
// }