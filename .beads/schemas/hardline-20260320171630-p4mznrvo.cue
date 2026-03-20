
package validation

import "list"

// Validation schema for bead: hardline-20260320171630-p4mznrvo
// Title: gix: Implement branch operations
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320171630-p4mznrvo.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320171630-p4mznrvo"
  title: "gix: Implement branch operations"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "`gix` crate and `GitError` are available.",
      "`crate::domain::entities::Branch` is defined.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Branch creation creates a new reference pointing to the current HEAD.",
      "Switching branches updates the HEAD reference and attaches it to the target branch OID.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "All branch references conform to `refs/heads/<name>` format for local branches.",
      "Operations modifying refs must use transactions.",
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
      "Verify `create` followed by `list` shows the newly created branch.",
      "Verify `switch` updates the current HEAD to the target branch.",
    ]

    // Required error path tests
    required_error_tests: [
      "Verify `create` without force returns an error if the branch exists.",
      "Verify `switch` to a non-existent branch returns `GitError::InvalidRef`.",
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