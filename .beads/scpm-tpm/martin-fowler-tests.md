---
bead_id: scpm-tpm
title: "cli: implement abort command"
type: feature
phase: TEST_PLAN
updated_at: "2026-03-21T02:45:00Z"
---

# Martin Fowler Test Plan: cli abort command

## Test Philosophy
- Tests are executable specifications
- Given-When-Then structure for clarity
- Happy path + error path + edge cases

## Happy Path Tests

### test_abort_workspace_succeeds_when_valid
**Given**: A workspace `test-ws` exists with clean working copy and non-merged state
**When**: User executes `abort("test-ws")`
**Then**: 
- Exit code is 0
- Workspace is deleted from filesystem
- `WorkspaceAborted` event is emitted
- Success message is displayed

### test_abort_current_workspace_uses_current_when_none_specified
**Given**: Current workspace is `current-ws` with clean working copy
**When**: User executes `abort(None)` (no workspace name)
**Then**:
- Exit code is 0
- Current workspace `current-ws` is aborted
- Workspace is deleted from filesystem

### test_abort_transitions_workspace_state_to_abandoned
**Given**: A workspace with tracked state exists
**When**: `abort()` is called successfully
**Then**:
- Internal workspace state transitions to 'Abandoned'
- Event `WorkspaceAborted { name, reason: "user_requested" }` is emitted

## Error Path Tests

### test_abort_returns_error_when_workspace_not_found
**Given**: No workspace named `nonexistent` exists
**When**: User executes `abort("nonexistent")`
**Then**:
- Exit code is non-zero (10)
- Error message: "Workspace not found: nonexistent"
- No workspace is modified

### test_abort_returns_error_when_workspace_is_merged
**Given**: A workspace `merged-ws` exists in 'Merged' state
**When**: User executes `abort("merged-ws")`
**Then**:
- Exit code is non-zero (96)
- Error message: "cannot abort merged workspace"
- Workspace is not deleted

### test_abort_returns_error_when_workspace_is_main
**Given**: Main workspace exists
**When**: User executes `abort("main")`
**Then**:
- Exit code is non-zero (96)
- Error message: "cannot abort the main workspace"
- Main workspace is not modified

### test_abort_returns_error_when_working_copy_dirty
**Given**: A workspace `dirty-ws` exists but has uncommitted changes
**When**: User executes `abort("dirty-ws")`
**Then**:
- Exit code is non-zero (38)
- Error message: "Working copy has uncommitted changes"
- Workspace is not deleted

## Edge Case Tests

### test_abort_handles_empty_workspace_name_gracefully
**Given**: Valid workspace exists
**When**: `abort("")` is called with empty string
**Then**:
- Error is returned indicating invalid workspace name
- No panic or unwrap failure

### test_abort_handles_special_characters_in_workspace_name
**Given**: A workspace with special characters in name (if allowed)
**When**: Abort is attempted with invalid characters
**Then**: Proper validation error is returned

### test_abort_idempotent_if_called_twice
**Given**: After successful abort of `test-ws`
**When**: User executes `abort("test-ws")` again
**Then**:
- Error returned: "Workspace not found: test-ws"
- No panic or unexpected behavior

## Contract Verification Tests

### test_precondition_p1_workspace_exists_verification
**Given**: `validate_abort_preconditions` function
**When**: Called with non-existent workspace
**Then**: Returns `Err(Error::WorkspaceNotFound(...))`

### test_precondition_p2_not_merged_verification
**Given**: `validate_abort_preconditions` function
**When**: Called with merged workspace
**Then**: Returns `Err(Error::InvalidOperation(...))`

### test_precondition_p3_not_main_verification
**Given**: `validate_abort_preconditions` function
**When**: Called with "main" workspace
**Then**: Returns `Err(Error::InvalidOperation("cannot abort the main workspace"))`

### test_precondition_p4_clean_working_copy_verification
**Given**: `validate_abort_preconditions` function
**When**: Called with dirty working copy
**Then**: Returns `Err(Error::WorkingCopyDirty)`

### test_postcondition_q1_workspace_aborted_event_emitted
**Given**: Successful abort operation
**When**: Operation completes
**Then**: `WorkspaceAborted` event is in event history

### test_postcondition_q2_workspace_directory_removed
**Given**: Successful abort operation
**When**: Operation completes
**Then**: `jj workspace list` does not include the aborted workspace

### test_postcondition_q3_main_branch_unaffected
**Given**: Successful abort of non-main workspace
**When**: Operation completes
**Then**: Main branch has same commits as before abort

## Contract Violation Tests

### test_p1_violation_returns_workspace_not_found
**Given**: Non-existent workspace `ghost-ws`
**When**: `abort("ghost-ws")` is called
**Then**: Returns `Err(Error::WorkspaceNotFound("ghost-ws"))`
**Not**: Returns panic, unwrap failure, or unexpected success

### test_p2_violation_returns_invalid_operation
**Given**: Workspace in 'Merged' state
**When**: `abort()` is called
**Then**: Returns `Err(Error::InvalidOperation("cannot abort merged workspace"))`
**Not**: Returns panic or allows abort

### test_p3_violation_returns_invalid_operation
**Given**: Main workspace
**When**: `abort("main")` is called
**Then**: Returns `Err(Error::InvalidOperation("cannot abort the main workspace"))`
**Not**: Returns panic or allows abort

### test_p4_violation_returns_working_copy_dirty
**Given**: Workspace with uncommitted changes
**When**: `abort()` is called
**Then**: Returns `Err(Error::WorkingCopyDirty)`
**Not**: Returns panic or proceeds with abort

## Given-When-Then Scenarios

### Scenario 1: Successful workspace abort
**Given**: A developer has finished working on feature-x and wants to discard changes
**And**: The workspace has clean working copy
**And**: The workspace is not merged
**When**: They run `scp abort feature-x`
**Then**: The workspace is destroyed
**And**: They see "Workspace 'feature-x' aborted and deleted"
**And**: The working directory is clean

### Scenario 2: Attempt to abort already-merged workspace
**Given**: A workspace `feature-y` was merged into main
**When**: Developer runs `scp abort feature-y`
**Then**: Command fails with error
**And**: Error explains merged workspaces cannot be aborted
**And**: Workspace remains available for reference

### Scenario 3: Prevent accidental main abort
**Given**: User mistakenly types `scp abort main`
**When**: Command executes
**Then**: Error is returned
**And**: Main workspace is untouched
**And**: User is informed main cannot be aborted

## End-to-End Test

### test_full_abort_pipeline
**Given**: Fresh workspace `e2e-test-ws` with some commits
**When**: `scp abort e2e-test-ws` is executed
**Then**:
- Exit code 0
- `jj workspace list` does not show `e2e-test-ws`
- `jj log` shows no loss of main branch commits
- Event log contains `WorkspaceAborted` event
