
package validation

import "list"

// Validation schema for bead: hardline-20260320173012-cevtih6b
// Title: cli: implement abort command
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320173012-cevtih6b.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320173012-cevtih6b"
  title: "cli: implement abort command"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The specified WorkspaceName must exist.",
      "The WorkspaceState must not be 'Merged'.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "The WorkspaceState is transitioned to 'Abandoned'.",
      "The physical workspace is entirely removed from the filesystem.",
      "The main branch remains completely unaffected.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "No file changes from the aborted workspace can leak into the main branch or other active workspaces.",
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
      "Given an active workspace `bad-idea` with uncommitted broken code, when executing `hardline abort --workspace bad-idea`, the directory is deleted and main is unchanged.",
      "Given an active workspace that is currently holding a lock on a Bead, when executing `abort`, the lock is yielded and the Bead is returned to the pool.",
    ]

    // Required error path tests
    required_error_tests: [
      "Given a workspace name that does not exist, when executing `abort`, it returns a WorkspaceNotFoundError.",
      "Given a workspace that has already been merged, when executing `abort`, it returns an InvalidStateTransition error.",
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
//     timestamp: "2026-03-20T17:30:13Z"
//   }
// }