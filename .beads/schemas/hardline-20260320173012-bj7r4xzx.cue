
package validation

import "list"

// Validation schema for bead: hardline-20260320173012-bj7r4xzx
// Title: vcs: implement git backend
//
// This schema validates that implementation is complete.
// Use: cue vet hardline-20260320173012-bj7r4xzx.cue implementation.cue

#BeadImplementation: {
  bead_id: "hardline-20260320173012-bj7r4xzx"
  title: "vcs: implement git backend"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Git CLI must be installed and available in PATH.",
      "The provided path must be a valid git repository (contains .git).",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Repository state matches the requested operation (e.g. branch checked out).",
      "Functions return explicitly typed Results.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Git config is never modified globally by the backend.",
      "No destructive/irreversible operations (like push --force) are executed without explicit flags.",
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
      "Test executing `git status` on a clean repository returns empty change list.",
      "Test creating a new branch, adding a file, and committing succeeds and is reflected in `git log`.",
    ]

    // Required error path tests
    required_error_tests: [
      "Test `git commit` fails with VcsError::NothingToCommit when working tree is clean.",
      "Test operations on a non-git directory fail with VcsError::NotARepository.",
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