# Martin Fowler Test Plan

## Happy Path Tests

### switch command
- `test_switch_workspace_succeeds_when_workspace_exists_and_working_copy_clean`
  - Given: A workspace "test-ws" exists and working copy is clean
  - When: User runs `scp switch test-ws`
  - Then: Command succeeds, outputs "Switched to 'test-ws'"

- `test_switch_workspace_outputs_success_message`
  - Given: Valid workspace exists and is switchable
  - When: `switch_to_workspace("valid-ws")` is called
  - Then: Success message is printed to stdout

### context/whereami command
- `test_context_displays_current_workspace_name`
  - Given: User is in an initialized VCS with a workspace
  - When: User runs `scp context`
  - Then: Output includes current workspace name

- `test_context_displays_current_branch`
  - Given: User is in an initialized VCS
  - When: User runs `scp context` or `scp whereami`
  - Then: Output includes current branch name

- `test_context_displays_vcs_status`
  - Given: User is in an initialized VCS
  - When: User runs `scp context`
  - Then: Output includes VCS status (clean/dirty)

- `test_whereami_alias_works`
  - Given: User is in an initialized VCS
  - When: User runs `scp whereami`
  - Then: Output is identical to `scp context`

## Error Path Tests

### switch command
- `test_switch_returns_error_when_workspace_not_found`
  - Given: No workspace named "nonexistent" exists
  - When: User runs `scp switch nonexistent`
  - Then: Returns `Err(Error::WorkspaceNotFound("nonexistent".into()))`

- `test_switch_returns_error_when_working_copy_dirty`
  - Given: Workspace "test-ws" exists but working copy has uncommitted changes
  - When: User runs `scp switch test-ws`
  - Then: Returns `Err(Error::WorkingCopyDirty)` with suggestion to commit/stash

- `test_switch_returns_error_when_workspace_name_empty`
  - Given: Empty string provided as workspace name
  - When: User runs `scp switch ""`
  - Then: Returns `Err(Error::InvalidIdentifier(...))`

- `test_context_returns_error_when_vcs_not_initialized`
  - Given: Current directory is not a VCS repository
  - When: User runs `scp context`
  - Then: Returns `Err(Error::VcsNotInitialized)`

## Edge Case Tests

### switch command
- `test_switch_handles_single_workspace_list`
  - Given: Only one workspace exists
  - When: User switches to that workspace
  - Then: Command succeeds

- `test_switch_to_current_workspace_is_idempotent`
  - Given: User is already on workspace "current-ws"
  - When: User runs `scp switch current-ws`
  - Then: Command succeeds (no-op, but valid)

### context command
- `test_context_works_in_empty_repository`
  - Given: VCS is initialized but no commits
  - When: User runs `scp context`
  - Then: Output shows "no commits" or similar for branch

- `test_context_works_with_detached_head`
  - Given: VCS is in detached HEAD state
  - When: User runs `scp context`
  - Then: Output shows detached state appropriately

## Contract Verification Tests

- `test_precondition_workspace_name_not_empty`
  - Given: Empty string
  - When: `switch_to_workspace("")` is called
  - Then: Returns error (not panic)

- `test_precondition_workspace_exists`
  - Given: Nonexistent workspace name
  - When: `switch_to_workspace("does-not-exist")` is called
  - Then: Returns `Error::WorkspaceNotFound`

- `test_precondition_working_copy_clean`
  - Given: Dirty working copy
  - When: `switch_to_workspace("any-ws")` is called
  - Then: Returns `Error::WorkingCopyDirty`

- `test_postcondition_switch_changes_workspace`
  - Given: Valid switch request
  - When: After `switch_to_workspace` succeeds
  - Then: Current workspace is the target workspace

## Given-When-Then Scenarios

### Scenario 1: Successful workspace switch
**Given**: A Jujutsu repository with workspaces "feature-a" and "main", working copy is clean
**When**: User executes `scp switch feature-a`
**Then**:
- Command returns `Ok(())`
- Output shows "Switched to 'feature-a'"
- Current workspace is now "feature-a"

### Scenario 2: Failed switch - workspace not found
**Given**: A Jujutsu repository with only "main" workspace
**When**: User executes `scp switch nonexistent`
**Then**:
- Command returns `Err(Error::WorkspaceNotFound("nonexistent".into()))`
- Error message: "Workspace not found: nonexistent"
- Suggestion: "Try 'scp workspace list' to see available workspaces"

### Scenario 3: Failed switch - dirty working copy
**Given**: A workspace with uncommitted changes in working copy
**When**: User executes `scp switch other-workspace`
**Then**:
- Command returns `Err(Error::WorkingCopyDirty)`
- Error message: "Working copy has uncommitted changes"
- Suggestion: "Commit or stash your changes before continuing"

### Scenario 4: Context shows current location
**Given**: An initialized VCS with workspace "dev-branch" on branch "feature/ticket-123"
**When**: User executes `scp context`
**Then**:
- Output includes "Workspace: dev-branch"
- Output includes "Branch: feature/ticket-123"
- Output includes "Status: clean" (or "Status: dirty" if changes exist)

### Scenario 5: Whereami is alias for context
**Given**: Any valid VCS directory
**When**: User executes `scp whereami`
**Then**:
- Output is identical to `scp context`
- Both commands use the same underlying function
