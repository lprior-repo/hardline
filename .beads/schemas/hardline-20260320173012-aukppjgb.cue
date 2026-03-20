
package validation

import "list"

// Validation schema for bead: hardline-20260320173012-aukppjgb
// Title: agent-aggregate: implement agent state transitions
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320173012-aukppjgb.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320173012-aukppjgb"
  title: "agent-aggregate: implement agent state transitions"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "AgentId must be valid (1-128 chars, alphanumeric)",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Transition to Active must associate the Agent with a specific TaskId or WorkspaceId",
      "Transition to Error must record the error reason",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "An agent can only be active on one task at a time",
      "Offline agents cannot be assigned new tasks",
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
      "Given an Idle agent, when assign is called with a TaskId, then it transitions to Active",
      "Given an Active agent, when fail is called, then it transitions to Error",
      "Given an Error agent, when reset is called, then it transitions to Idle",
    ]

    // Required error path tests
    required_error_tests: [
      "Given an Offline agent, when assign is called, then it returns an AgentOfflineError",
      "Given an Active agent, when assign is called with a new task, then it returns an AgentAlreadyBusyError",
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