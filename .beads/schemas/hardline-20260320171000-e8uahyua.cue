
package validation

import "list"

// Validation schema for bead: hardline-20260320171000-e8uahyua
// Title: session: Add Bead domain types
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320171000-e8uahyua.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320171000-e8uahyua"
  title: "session: Add Bead domain types"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Title must not exceed maximum length (255 chars).",
      "IssueType must be one of the defined variants (Bug, Feature, Task, Epic, Chore, MergeRequest).",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Dependencies are successfully wrapped in a collection type.",
      "Priority maps precisely to the 0-4 scale.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Title and Description types must not allow un-trimmed whitespace at the edges.",
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
      "Valid Title string creates a Title instance.",
      "Priority::from_u8(0) returns Ok(Priority::P0).",
    ]

    // Required error path tests
    required_error_tests: [
      "Title string exceeding 255 chars returns validation error.",
      "Priority::from_u8(5) returns validation error.",
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
//     timestamp: "2026-03-20T17:10:00Z"
//   }
// }