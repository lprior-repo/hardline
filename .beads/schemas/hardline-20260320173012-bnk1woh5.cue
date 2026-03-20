
package validation

import "list"

// Validation schema for bead: hardline-20260320173012-bnk1woh5
// Title: cli: implement workspace context and switch commands
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320173012-bnk1woh5.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320173012-bnk1woh5"
  title: "cli: implement workspace context and switch commands"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The provided WorkspaceName must be valid and conform to the 1-255 chars validation rule.",
      "The agent executing the command must be active and registered.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "The active session environment strictly points to the target workspace after a successful switch.",
      "The context command outputs accurate, deterministic JSON representing the current state.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "A session cannot be active in a non-existent or inaccessible workspace.",
      "Context discovery always returns a deterministic location without side effects.",
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
      "Test `switch` to an existing valid workspace successfully updates the current active directory and session state.",
      "Test `context` returns valid JSON containing absolute path, active workspace name, and agent ID.",
    ]

    // Required error path tests
    required_error_tests: [
      "Test `switch` to a non-existent workspace returns an explicit Error type and leaves the current context unchanged.",
      "Test `context` outside of any valid workspace returns a specific NotInWorkspace error instead of panicking.",
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