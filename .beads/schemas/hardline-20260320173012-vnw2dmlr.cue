
package validation

import "list"

// Validation schema for bead: hardline-20260320173012-vnw2dmlr
// Title: cli: implement task claim and yield commands
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320173012-vnw2dmlr.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320173012-vnw2dmlr"
  title: "cli: implement task claim and yield commands"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The target task must exist in the system",
      "The target task must not be in a terminal state (e.g., Closed)",
      "The agent executing the command must have a valid AgentId",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "After a successful claim, the task's metadata reflects the new assignee and TTL lock expiration",
      "After a successful yield, the task has no assignee and is available for other agents",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "A single task can have at most one active claim/assignee at any given time",
      "An agent can only yield a task that they currently hold the claim for",
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
      "Verify that an agent can successfully claim an unassigned task and the TTL lock is established",
      "Verify that an agent can successfully yield a task they own, making it available again",
    ]

    // Required error path tests
    required_error_tests: [
      "Verify that claiming a task already claimed by another agent fails with a ConflictError",
      "Verify that attempting to yield a task claimed by another agent fails with an UnauthorizedError",
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