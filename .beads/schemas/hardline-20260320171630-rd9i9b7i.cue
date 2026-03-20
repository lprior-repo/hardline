
package validation

import "list"

// Validation schema for bead: hardline-20260320171630-rd9i9b7i
// Title: cli: implement task list and show commands
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320171630-rd9i9b7i.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320171630-rd9i9b7i"
  title: "cli: implement task list and show commands"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The domain repository must be accessible and readable.",
      "The provided task_id for 'task show' must pass basic structural validation.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "The task list or task details are printed to standard output without altering any task state.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Task state remains unchanged during list and show operations.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(2)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Run 'task list' and verify a structured list of tasks is returned.",
      "Run 'task show bd-123' and verify detailed task information is displayed.",
    ]

    // Required error path tests
    required_error_tests: [
      "Run 'task show invalid-id' and verify a domain error is returned.",
      "Run 'task show bd-999' for a non-existent task and verify a 'not found' error is gracefully handled.",
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