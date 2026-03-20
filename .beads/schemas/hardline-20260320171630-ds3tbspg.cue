
package validation

import "list"

// Validation schema for bead: hardline-20260320171630-ds3tbspg
// Title: orchestrator: parallel phase execution
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320171630-ds3tbspg.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320171630-ds3tbspg"
  title: "orchestrator: parallel phase execution"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Phase graph must be acyclic.",
      "All phase dependencies must reference valid existing phases.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "All reachable phases complete successfully or return an aggregated error.",
      "Execution time for parallel phases must not exceed the longest individual phase plus overhead.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "A phase is never executed before its dependencies have completed.",
      "No phase is executed more than once.",
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
      "Linear graph executes sequentially.",
      "Fork-join graph executes independent branches in parallel.",
    ]

    // Required error path tests
    required_error_tests: [
      "Cyclic dependency returns structural validation error before execution.",
      "Missing dependency reference returns validation error.",
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