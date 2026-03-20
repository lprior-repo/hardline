
package validation

import "list"

// Validation schema for bead: hardline-20260320173012-gu6wnzkq
// Title: session: implement TaskId type
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320173012-gu6wnzkq.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320173012-gu6wnzkq"
  title: "session: implement TaskId type"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The input must be a non-empty string.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "The returned TaskId object is guaranteed to be a valid, 'bd-'-prefixed hex string.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "TaskId.to_string() always starts with 'bd-'.",
      "The suffix after 'bd-' consists entirely of characters in [0-9a-fA-F].",
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
      "ATDD: Given the input 'bd-1a2b3c', When creating a TaskId, Then it successfully creates the instance.",
      "ATDD: Given a valid TaskId instance, When formatted as a string, Then it produces 'bd-<hex>'.",
    ]

    // Required error path tests
    required_error_tests: [
      "ATDD: Given the input '1a2b3c' (missing prefix), When creating a TaskId, Then it returns a MissingPrefixError.",
      "ATDD: Given the input 'bd-1a2x3c' (invalid hex 'x'), When creating a TaskId, Then it returns an InvalidHexError.",
      "ATDD: Given the input 'bd-', When creating a TaskId, Then it returns an EmptyHexSuffixError.",
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
//     timestamp: "2026-03-20T17:30:12Z"
//   }
// }