
package validation

import "list"

// Validation schema for bead: hardline-20260320171630-kl8v9ieq
// Title: gix: implement status and tag modules
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320171630-kl8v9ieq.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320171630-kl8v9ieq"
  title: "gix: implement status and tag modules"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Repository must be successfully opened with gix.",
      "For tag creation, a valid HEAD commit must exist.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Status query does not modify repository state.",
      "Tag creation results in a new refs/tags/ object pointing to the HEAD commit.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "All errors must be mapped to GitError.",
      "No git cli commands are spawned.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(3)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Status reports Clean on fresh repo.",
      "Status reports Dirty on modified file.",
      "Create and list tags works successfully.",
    ]

    // Required error path tests
    required_error_tests: [
      "Tag creation fails gracefully on detached HEAD with no commits.",
      "Status check handles missing workdir gracefully.",
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