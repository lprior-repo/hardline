# Martin Fowler Test Plan

## Happy Path Tests
- test_spawn_creates_workspace_successfully
  Given: A valid workspace name that doesn't exist
  When: spawn is called with the name and sync=false
  Then: New workspace is created in VCS backend

- test_spawn_with_sync_creates_and_switches_workspace
  Given: A valid workspace name that doesn't exist
  When: spawn is called with the name and sync=true
  Then: New workspace is created and switched to, rebased on main

- test_done_merges_workspace_to_main
  Given: A workspace that exists with changes
  When: done is called with the workspace name
  Then: Workspace is rebased on main and pushed to remote

- test_done_with_no_name_uses_current_workspace
  Given: Currently in a workspace
  When: done is called with None
  Then: Current workspace is completed

- test_abort_deletes_workspace_without_merge
  Given: A workspace that exists (not main)
  When: abort is called with the workspace name
  Then: Workspace is deleted from VCS backend

- test_abort_with_no_name_uses_current_workspace
  Given: Currently in a workspace (not main)
  When: abort is called with None
  Then: Current workspace is deleted

## Error Path Tests
- test_spawn_fails_with_empty_name
  Given: Empty string as workspace name
  When: spawn is called with ""
  Then: Returns `Err(Error::InvalidIdentifier(""))`

- test_spawn_fails_with_invalid_identifier
  Given: Invalid identifier as workspace name (starts with number)
  When: spawn is called with "123invalid"
  Then: Returns `Err(Error::InvalidIdentifier("123invalid"))`

- test_spawn_fails_when_workspace_exists
  Given: A workspace that already exists
  When: spawn is called with existing workspace name
  Then: Returns `Err(Error::WorkspaceExists(name))`

- test_done_fails_when_workspace_not_found
  Given: A workspace that doesn't exist
  When: done is called with nonexistent workspace name
  Then: Returns `Err(Error::WorkspaceNotFound(name))`

- test_abort_fails_when_workspace_not_found
  Given: A workspace that doesn't exist
  When: abort is called with nonexistent workspace name
  Then: Returns `Err(Error::WorkspaceNotFound(name))`

- test_abort_fails_when_trying_to_abort_main
  Given: "main" as workspace name
  When: abort is called with "main"
  Then: Returns `Err(Error::InvalidOperation(...))`

- test_switch_fails_with_dirty_working_copy
  Given: Working copy has uncommitted changes
  When: switch is called
  Then: Returns `Err(Error::WorkingCopyDirty)`

- test_done_fails_with_dirty_working_copy
  Given: Working copy has uncommitted changes
  When: done is called
  Then: Returns `Err(Error::WorkingCopyDirty)` (implicit - rebase would fail)

- test_abort_fails_with_dirty_working_copy
  Given: Working copy has uncommitted changes
  When: abort is called
  Then: Returns `Err(Error::WorkingCopyDirty)` (implicit - switch would fail)

## Edge Case Tests
- test_spawn_with_unicode_workspace_name
  Given: Unicode characters in workspace name
  When: spawn is called
  Then: Returns `Err(Error::InvalidIdentifier(...))` (depends on validation)

- test_abort_when_workspace_is_current
  Given: Currently in the workspace being aborted
  When: abort is called
  Then: Workspace is deleted, no longer in any workspace

- test_done_when_no_remote_configured
  Given: Repository without remote configured
  When: done is called
  Then: Returns `Err(Error::VcsPushFailed(...))`

- test_spawn_with_very_long_workspace_name
  Given: Very long workspace name (near limit)
  When: spawn is called
  Then: May succeed or fail depending on VCS limits

## Contract Verification Tests
- test_precondition_p1_workspace_name_valid
  Given: Invalid workspace names
  When: spawn is called
  Then: Each produces corresponding Error::InvalidIdentifier

- test_precondition_p2_workspace_not_exists
  Given: Existing workspace name
  When: spawn is called
  Then: Returns Error::WorkspaceExists

- test_precondition_p3_workspace_exists
  Given: Nonexistent workspace name
  When: done/abort is called
  Then: Returns Error::WorkspaceNotFound

- test_postcondition_q1_spawn_creates_workspace
  Given: Valid spawn call
  When: spawn completes successfully
  Then: Workspace exists in list_workspaces()

- test_postcondition_q3_done_pushes_to_remote
  Given: Valid done call
  When: done completes successfully
  Then: Changes are visible on remote

- test_postcondition_q4_abort_deletes_workspace
  Given: Valid abort call
  When: abort completes successfully
  Then: Workspace no longer exists in list_workspaces()

## Contract Violation Tests
- test_p1_violation_empty_name_returns_invalid_identifier
  Given: spawn("", false)
  When: function is called with empty name
  Then: returns Err(Error::InvalidIdentifier(""))

- test_p1_violation_invalid_identifier_returns_invalid_identifier
  Given: spawn("123invalid", false)
  When: function is called with name starting with number
  Then: returns Err(Error::InvalidIdentifier("123invalid"))

- test_p2_violation_workspace_exists_returns_workspace_exists
  Given: spawn("existing", false) where "existing" workspace exists
  When: function is called
  Then: returns Err(Error::WorkspaceExists("existing"))

- test_p3_violation_workspace_not_found_returns_not_found
  Given: done(Some("nonexistent"))
  When: function is called with nonexistent workspace
  Then: returns Err(Error::WorkspaceNotFound("nonexistent"))

- test_p5_violation_abort_main_returns_invalid_operation
  Given: abort(Some("main"))
  When: function is called with main workspace
  Then: returns Err(Error::InvalidOperation(...))

## Given-When-Then Scenarios

### Scenario 1: Create new feature workspace
Given: User wants to work on a new feature
When: They run `scp workspace spawn feature-branch --sync`
Then:
- New workspace "feature-branch" is created
- They are switched to the new workspace
- Their working copy is rebased on main

### Scenario 2: Complete feature and merge
Given: User has completed work in feature-branch workspace
When: They run `scp workspace done feature-branch`
Then:
- Their changes are rebased onto main
- Changes are pushed to remote
- Success message is displayed

### Scenario 3: Abandon feature work
Given: User started work but wants to discard it
When: They run `scp workspace abort feature-branch`
Then:
- Workspace is deleted from local jj
- No changes are pushed to remote
- User returns to previous workspace (or no workspace)

### Scenario 4: Spawn fails with existing workspace
Given: Workspace "my-feature" already exists
When: User runs `scp workspace spawn my-feature`
Then:
- Error message explains workspace exists
- Suggestion to use different name or list existing workspaces

### Scenario 5: Done fails with uncommitted changes
Given: User has uncommitted changes in workspace
When: They run `scp workspace done`
Then:
- Error explains working copy is dirty
- Suggestion to commit or stash changes first
