# Martin Fowler Test Plan: Git CLI Backend

## Happy Path Tests
- test_git_cli_backend_creation_returns_valid_instance
- test_status_returns_clean_on_clean_repository
- test_status_returns_dirty_on_repository_with_changes
- test_log_returns_commits_in_reverse_chronological_order
- test_log_with_limit_returns_exactly_n_commits
- test_diff_returns_empty_string_on_clean_repository
- test_current_branch_returns_none_on_detached_head
- test_current_branch_returns_branch_name_on_attached_head
- test_list_branches_returns_all_local_branches

## Error Path Tests
- test_status_returns_not_initialized_when_no_git_directory
- test_log_returns_not_initialized_when_no_git_directory
- test_diff_returns_not_initialized_when_no_git_directory
- test_git_not_installed_returns_git_not_installed_error
- test_invalid_repository_path_returns_error

## Edge Case Tests
- test_log_with_zero_limit_returns_empty_vector
- test_log_on_repository_with_no_commits_returns_empty_vector
- test_status_on_newly_initialized_repository_returns_clean
- test_list_branches_on_repository_with_no_branches_returns_only_current

## Contract Verification Tests
- test_status_never_panics_returns_result
- test_log_never_panics_returns_result
- test_diff_never_panics_returns_result
- test_all_operations_return_result_not_option

## Contract Violation Tests
- test_status_violation_returns_err_not_panic
  Given: Repository path does not contain `.git` directory
  When: `git_cli_backend.status()` is called
  Then: Returns `Err(VcsError::NotInitialized)` -- NOT a panic

- test_log_violation_returns_err_not_panic
  Given: Repository path does not contain `.git` directory
  When: `git_cli_backend.log(10)` is called
  Then: Returns `Err(VcsError::NotInitialized)` -- NOT a panic

- test_diff_violation_returns_err_not_panic
  Given: Repository path does not contain `.git` directory
  When: `git_cli_backend.diff()` is called
  Then: Returns `Err(VcsError::NotInitialized)` -- NOT a panic

- test_git_not_found_violation_returns_err_not_panic
  Given: Git CLI is not installed or not in PATH
  When: Any git operation is attempted
  Then: Returns `Err(VcsError::GitNotInstalled)` -- NOT a panic

## Given-When-Then Scenarios

### Scenario 1: Check status of clean repository
Given: A git repository with no uncommitted changes exists at `/tmp/test_repo_clean`
And: GitCliBackend is created with path `/tmp/test_repo_clean`
When: `status()` is called
Then:
- Returns `Ok(VcsStatus::Clean)`
- Does not panic
- Returns within 1 second

### Scenario 2: Check status of dirty repository
Given: A git repository with uncommitted changes exists at `/tmp/test_repo_dirty`
And: A file `modified.txt` has been changed but not staged
And: GitCliBackend is created with path `/tmp/test_repo_dirty`
When: `status()` is called
Then:
- Returns `Ok(VcsStatus::Dirty)`
- Does not panic

### Scenario 3: Get commit log with limit
Given: A git repository with 5 commits exists at `/tmp/test_repo_log`
And: GitCliBackend is created with path `/tmp/test_repo_log`
When: `log(3)` is called
Then:
- Returns `Ok(commits)` where `commits.len() == 3`
- First commit is the most recent
- Each commit has valid id, message, author, timestamp, parents

### Scenario 4: Get diff of changes
Given: A git repository with uncommitted changes exists at `/tmp/test_repo_diff`
And: File `example.txt` has been modified
And: GitCliBackend is created with path `/tmp/test_repo_diff`
When: `diff()` is called
Then:
- Returns `Ok(diff_output)` where output contains "example.txt"
- Returns empty string when no changes exist

### Scenario 5: Get current branch name
Given: A git repository on branch `main` exists at `/tmp/test_repo_branch`
And: GitCliBackend is created with path `/tmp/test_repo_branch`
When: `current_branch()` is called
Then:
- Returns `Ok(Some("main"))`

### Scenario 6: List all branches
Given: A git repository with branches `main` and `feature` exists at `/tmp/test_repo_branches`
And: GitCliBackend is created with path `/tmp/test_repo_branches`
When: `list_branches()` is called
Then:
- Returns `Ok(branches)` where `branches.len() >= 2`
- At least one branch has `is_current == true`

### Scenario 7: Repository not initialized
Given: A directory exists at `/tmp/test_repo_none` but is not a git repository
And: GitCliBackend is created with path `/tmp/test_repo_none`
When: `status()` is called
Then:
- Returns `Err(VcsError::NotInitialized)`
- Does not panic

### Scenario 8: Git not installed
Given: The git command is not available in PATH
And: GitCliBackend is created with any valid path
When: Any operation is attempted
Then:
- Returns `Err(VcsError::GitNotInstalled)`
- Does not panic
