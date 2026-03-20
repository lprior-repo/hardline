
package validation

import "list"

// Validation schema for bead: hardline-20260320173012-gwhz1lnd
// Title: cli: implement task start and done commands
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320173012-gwhz1lnd.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320173012-gwhz1lnd"
  title: "cli: implement task start and done commands"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The target task must exist and be claimed by the executing agent",
      "For `task start`, the task must be in the Open or Ready state",
      "For `task done`, the task must be in the InProgress state",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "After `task start`, the task's internal status is updated to InProgress",
      "After `task done`, the task's internal status is updated to a terminal Closed state with a resolution reason",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "A task can only be transitioned by the agent holding its active claim",
      "A task cannot be moved backward from a terminal state to an active state",
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
      "Verify that an agent can start a task they have claimed, moving its state to InProgress",
      "Verify that an agent can mark a started task as done, capturing the resolution reason and closing the task",
    ]

    // Required error path tests
    required_error_tests: [
      "Verify that starting a task without an active claim fails with a PreconditionFailedError",
      "Verify that marking a task as done when it is already Closed fails with an InvalidStateTransitionError",
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