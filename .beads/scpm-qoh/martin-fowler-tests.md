# Martin Fowler Test Plan: JJ Backend (scpm-qoh)

## Happy Path Tests

### test_current_branch_returns_branch_name
Given: A jj repository with an active bookmark
When: `current_branch()` is called
Then: Returns `Ok(String)` containing the current bookmark name

### test_list_branches_returns_all_bookmarks
Given: A jj repository with multiple bookmarks including one active
When: `list_branches()` is called
Then: Returns `Ok(Vec<Branch>)` with all bookmarks; active branch has `is_current: true`

### test_create_branch_succeeds_for_new_branch
Given: A jj repository at clean state
When: `create_branch("new-branch")` is called
Then: Returns `Ok(())` and the bookmark exists

### test_switch_branch_updates_active_bookmark
Given: A jj repository with "branch-a" and "branch-b" bookmarks
When: `switch_branch("branch-b")` is called
Then: Returns `Ok(())` and subsequent `current_branch()` returns "branch-b"

### test_push_executes_jj_git_push
Given: A jj repository with commits to push
When: `push()` is called
Then: Returns `Ok(())` and commits appear in remote

### test_pull_executes_jj_git_fetch
Given: A jj repository with remote changes
When: `pull()` is called
Then: Returns `Ok(())` and local state matches remote

### test_rebase_rebases_onto_target
Given: A jj repository with feature branch and main
When: `rebase("main")` is called from feature branch
Then: Returns `Ok(())` and feature commits are rebased onto main

### test_merge_merges_branch
Given: A jj repository with two divergent bookmarks
When: `merge("other-branch")` is called
Then: Returns `Ok(())` and commits from other-branch are integrated

### test_log_returns_commit_history
Given: A jj repository with multiple commits
When: `log(10)` is called
Then: Returns `Ok(Vec<Commit>)` with at most 10 commits, each having id, message, author, timestamp, parents

### test_status_returns_dirty_for_modified_working_copy
Given: A jj repository with uncommitted file changes
When: `status()` is called
Then: Returns `Ok(VcsStatus::Dirty)`

### test_status_returns_clean_for_no_changes
Given: A jj repository at clean state (no uncommitted changes)
When: `status()` is called
Then: Returns `Ok(VcsStatus::Clean)`

### test_status_returns_conflicted_during_merge_conflict
Given: A jj repository during an interrupted merge
When: `status()` is called
Then: Returns `Ok(VcsStatus::Conflicted)` when output contains "There are conflicts"

### test_is_initialized_returns_true_for_jj_repo
Given: A valid jj repository path
When: `is_initialized()` is called
Then: Returns `Ok(true)`

### test_create_workspace_adds_new_workspace
Given: A jj repository
When: `create_workspace("new-workspace")` is called
Then: Returns `Ok(())` and `list_workspaces()` includes the new workspace

### test_list_workspaces_returns_all_workspaces
Given: A jj repository with multiple workspaces
When: `list_workspaces()` is called
Then: Returns `Ok(Vec<Workspace>)` with all workspaces; current workspace has `is_current: true`

### test_delete_workspace_removes_workspace
Given: A jj repository with an existing workspace
When: `delete_workspace("old-workspace")` is called
Then: Returns `Ok(())` and workspace no longer appears in list

### test_fork_workspace_creates_from_source
Given: A jj repository with "source-workspace"
When: `fork_workspace("source-workspace", "target-workspace")` is called
Then: Returns `Ok(())` and target workspace exists with same bookmark

## Error Path Tests

### test_current_branch_returns_conflict_on_jj_failure
Given: A jj repository but jj command fails
When: `current_branch()` is called and jj exits non-zero
Then: Returns `Err(VcsError::Conflict(...))` with stderr content

### test_list_branches_returns_vec_not_error_on_empty
Given: A jj repository with no bookmarks
When: `list_branches()` is called
Then: Returns `Ok(Vec::new())` (empty vec, not an error)

### test_create_branch_returns_error_for_existing
Given: A jj repository with existing "existing-branch" bookmark
When: `create_branch("existing-branch")` is called
Then: Returns `Err(VcsError::BranchExists("existing-branch"))`

### test_switch_branch_returns_error_for_nonexistent
Given: A jj repository with no "nonexistent" bookmark
When: `switch_branch("nonexistent")` is called
Then: Returns `Err(VcsError::BranchNotFound("nonexistent"))`

### test_push_returns_error_on_failure
Given: A jj repository with network issues
When: `push()` is called and jj git push fails
Then: Returns `Err(VcsError::PushFailed(...))` with stderr

### test_pull_returns_error_on_failure
Given: A jj repository that cannot reach remote
When: `pull()` is called and jj git fetch fails
Then: Returns `Err(VcsError::PullFailed(...))` with stderr

### test_rebase_returns_error_on_failure
Given: A jj repository and invalid rebase target
When: `rebase("nonexistent")` is called
Then: Returns `Err(VcsError::RebaseFailed(...))` with stderr

### test_merge_returns_conflict_error_on_conflict
Given: A jj repository with conflicting changes
When: `merge("conflicting-branch")` is called and conflict occurs
Then: Returns `Err(VcsError::Conflict(...))` with conflict details

### test_status_returns_clean_for_detached_head
Given: A jj repository in detached HEAD state
When: `status()` is called
Then: Returns `Ok(VcsStatus::Clean)` (detached HEAD is not Dirty)

### test_is_initialized_returns_false_for_non_jj_directory
Given: A path without .jj directory
When: `is_initialized()` is called
Then: Returns `Ok(false)` (not an error)

### test_create_workspace_returns_error_for_existing
Given: A jj repository with "existing-workspace" already present
When: `create_workspace("existing-workspace")` is called
Then: Returns `Err(VcsError::WorkspaceExists("existing-workspace"))`

### test_switch_workspace_returns_error_for_nonexistent
Given: A jj repository with no "nonexistent" workspace
When: `switch_workspace("nonexistent")` is called
Then: Returns `Err(VcsError::WorkspaceNotFound("nonexistent"))`

### test_delete_workspace_returns_error_for_nonexistent
Given: A jj repository with no "nonexistent" workspace
When: `delete_workspace("nonexistent")` is called
Then: Returns `Err(VcsError::WorkspaceNotFound("nonexistent"))`

### test_fork_workspace_returns_error_for_existing_target
Given: A jj repository where "target" already exists
When: `fork_workspace("source", "target")` is called
Then: Returns `Err(VcsError::WorkspaceExists("target"))`

### test_merge_workspace_returns_error_for_nonexistent
Given: A jj repository with no "nonexistent" workspace
When: `merge_workspace("nonexistent")` is called
Then: Returns `Err(VcsError::WorkspaceNotFound("nonexistent"))`

## Edge Case Tests

### test_list_branches_handles_lines_with_colons
Given: A jj repository with bookmark containing ":" in name
When: `list_branches()` is called
Then: Correctly parses bookmark name before first colon only

### test_log_handles_empty_repository
Given: A jj repository with no commits
When: `log(10)` is called
Then: Returns `Ok(Vec::new())` (empty vec)

### test_status_handles_working_copy_without_files
Given: A jj repository with no files
When: `status()` is called
Then: Returns `Ok(VcsStatus::Clean)`

### test_workspace_operations_handle_paths_with_spaces
Given: A jj repository in a path containing spaces
When: All workspace operations are called
Then: Commands execute correctly (PathBuf handling)

### test_branch_operations_handle_unicode_names
Given: A jj repository with unicode bookmark names
When: `create_branch("feature-日本語")` is called
Then: Returns `Ok(())` and branch is queryable

## Contract Verification Tests

### test_all_methods_return_result_never_panic
Given: JjBackend instance
When: All public methods are called with invalid inputs
Then: Every method returns Result<T, VcsError> and never panics

### test_vcserror_variants_are_exhaustive
Given: All VcsError variants defined
When: Error handling code is written
Then: All JJ failure modes map to appropriate VcsError variant

### test_zero_unwrap_in_source_code
Given: Source code of jj.rs (excluding tests)
When: Code is analyzed
Then: No unwrap(), unwrap_or(), unwrap_or_else(), expect(), panic!() calls exist

## Given-When-Then Scenarios

### Scenario 1: Query current branch in healthy repository
Given: A jj repository with "main" as current bookmark
When: Developer calls `current_branch()`
Then: System returns `Ok("main".to_string())`
And: No CLI output is displayed to user
And: No side effects occur

### Scenario 2: Create branch that already exists
Given: A jj repository where "feature" bookmark already exists
When: Developer calls `create_branch("feature")`
Then: System returns `Err(VcsError::BranchExists("feature".to_string()))`
And: No new bookmark is created
And: Error message is actionable

### Scenario 3: Push to remote with no network
Given: A jj repository with local commits
And: Network connectivity is unavailable
When: Developer calls `push()`
Then: System returns `Err(VcsError::PushFailed(...))` with network error details
And: Local state remains unchanged

### Scenario 4: Check status in conflicted working copy
Given: A jj repository with unresolved merge conflicts
When: Developer calls `status()`
Then: System returns `Ok(VcsStatus::Conflicted)`
And: Developer can distinguish this from `VcsStatus::Dirty`

### Scenario 5: List workspaces in repository with multiple workspaces
Given: A jj repository with "default", "feature-a", "feature-b" workspaces
When: Developer calls `list_workspaces()`
Then: System returns 3 Workspace entities
And: "default" workspace has `is_current: true`
And: Each workspace has name and branch correctly populated
