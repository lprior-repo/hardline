---
bead_id: scp-fim
bead_title: Create SQLite schema for workspaces
phase: contract
updated_at: 2026-03-13T18:30:00Z
---

# Martin Fowler Test Plan

## Overview

This test plan follows Martin Fowler's approach with expressive test names and Given-When-Then scenarios. Tests are organized by category and map to the contract specification.

## Happy Path Tests

### Schema Migration

- `test_migrate_workspaces_creates_table_successfully`
  - Given: A valid SqlitePool connected to a fresh database
  - When: `migrate_workspaces(pool)` is called
  - Then: Returns `Ok(())` and the workspaces table exists

- `test_migrate_workspaces_is_idempotent`
  - Given: A database where workspaces table already exists
  - When: `migrate_workspaces(pool)` is called again
  - Then: Returns `Ok(())` without errors

- `test_migrate_workspaces_creates_required_indexes`
  - Given: A valid SqlitePool
  - When: `migrate_workspaces(pool)` completes successfully
  - Then: Indexes idx_workspaces_name, idx_workspaces_state, idx_workspaces_lock_holder all exist

### Workspace Repository Operations

- `test_save_workspace_returns_persisted_workspace`
  - Given: A valid Workspace with name, path, and config
  - When: `repository.save(workspace)` is called
  - Then: Returns `Ok(workspace)` with generated ID and timestamps

- `test_get_workspace_by_id_returns_some_when_exists`
  - Given: A workspace saved in the database
  - When: `repository.get(&workspace_id)` is called
  - Then: Returns `Ok(Some(workspace))` with matching fields

- `test_get_workspace_by_name_returns_some_when_exists`
  - Given: A workspace with name "test-workspace" saved in database
  - When: `repository.get_by_name("test-workspace")` is called
  - Then: Returns `Ok(Some(workspace))`

- `test_list_returns_all_workspaces_including_deleted`
  - Given: Multiple workspaces in various states (Active, Deleted)
  - When: `repository.list()` is called
  - Then: Returns all workspaces

- `test_list_active_returns_only_active_workspaces`
  - Given: Mixed workspaces (Active, Initializing, Locked, Deleted)
  - When: `repository.list_active()` is called
  - Then: Returns only workspaces where state = Active

- `test_delete_workspace_performs_soft_delete`
  - Given: An Active workspace
  - When: `repository.delete(&workspace_id)` is called
  - Then: Returns workspace with state = Deleted

### State Transition Tests

- `test_state_transition_initializing_to_active_succeeds`
  - Given: A workspace in Initializing state
  - When: `WorkspaceStateMachine::transition(Initializing, Active)` is called
  - Then: Returns `Ok(WorkspaceState::Active)`

- `test_state_transition_active_to_locked_succeeds`
  - Given: A workspace in Active state
  - When: `WorkspaceStateMachine::transition(Active, Locked)` is called
  - Then: Returns `Ok(WorkspaceState::Locked)`

- `test_state_transition_locked_to_active_succeeds`
  - Given: A workspace in Locked state
  - When: `WorkspaceStateMachine::transition(Locked, Active)` is called
  - Then: Returns `Ok(WorkspaceState::Active)`

- `test_state_transition_active_to_corrupted_succeeds`
  - Given: A workspace in Active state
  - When: `WorkspaceStateMachine::transition(Active, Corrupted)` is called
  - Then: Returns `Ok(WorkspaceState::Corrupted)`

- `test_state_transition_locked_to_corrupted_succeeds`
  - Given: A workspace in Locked state
  - When: `WorkspaceStateMachine::transition(Locked, Corrupted)` is called
  - Then: Returns `Ok(WorkspaceState::Corrupted)`

- `test_state_transition_corrupted_to_deleted_succeeds`
  - Given: A workspace in Corrupted state
  - When: `WorkspaceStateMachine::transition(Corrupted, Deleted)` is called
  - Then: Returns `Ok(WorkspaceState::Deleted)`

- `test_state_transition_any_to_deleted_succeeds`
  - Given: A workspace in any state (Initializing, Active, Locked, Corrupted)
  - When: `WorkspaceStateMachine::transition(current_state, Deleted)` is called
  - Then: Returns `Ok(WorkspaceState::Deleted)`

- `test_state_transition_active_to_initializing_fails`
  - Given: A workspace in Active state
  - When: `WorkspaceStateMachine::transition(Active, Initializing)` is called
  - Then: Returns `Err(WorkspaceError::InvalidStateTransition(...))`

- `test_state_transition_initializing_to_locked_fails`
  - Given: A workspace in Initializing state
  - When: `WorkspaceStateMachine::transition(Initializing, Locked)` is called
  - Then: Returns `Err(WorkspaceError::InvalidStateTransition(...))`

- `test_state_transition_deleted_to_active_fails`
  - Given: A workspace in Deleted state
  - When: `WorkspaceStateMachine::transition(Deleted, Active)` is called
  - Then: Returns `Err(WorkspaceError::InvalidStateTransition(...))`

- `test_state_transition_deleted_to_any_fails_except_deleted`
  - Given: A workspace in Deleted state
  - When: `WorkspaceStateMachine::transition(Deleted, Locked)` is called
  - Then: Returns `Err(WorkspaceError::InvalidStateTransition(...))`

## Error Path Tests

### Repository Operation Errors

- `test_get_workspace_by_id_returns_none_when_not_found`
  - Given: A database with no workspaces
  - When: `repository.get(&WorkspaceId::parse("ws-nonexistent".into()).unwrap())` is called
  - Then: Returns `Ok(None)`

- `test_save_workspace_returns_error_when_name_exists`
  - Given: A workspace with name "existing" already saved
  - When: Attempting to save another workspace with same name
  - Then: Returns `Err(WorkspaceError::WorkspaceExists("existing".into()))`

- `test_delete_workspace_returns_error_when_not_found`
  - Given: An empty database
  - When: `repository.delete(&WorkspaceId::parse("ws-nonexistent".into()).unwrap())` is called
  - Then: Returns `Err(WorkspaceError::WorkspaceNotFound("ws-nonexistent".into()))`

### Schema Migration Errors

- `test_migrate_workspaces_returns_error_on_invalid_pool`
  - Given: A SqlitePool with invalid connection (connection string to non-existent dir)
  - When: `migrate_workspaces(pool)` is called
  - Then: Returns `Err(WorkspaceError::OperationFailed(...))`

## Edge Case Tests

### Boundary Conditions

- `test_workspace_id_format_always_starts_with_ws_prefix`
  - Given: A generated WorkspaceId
  - When: The ID is accessed
  - Then: ID starts with "ws-"

- `test_timestamps_are_valid_rfc3339_format`
  - Given: A workspace with created_at and updated_at
  - When: Timestamps are serialized
  - Then: They are valid RFC3339 format strings

- `test_list_active_excludes_all_non_active_states`
  - Given: One Active, one Initializing, one Locked, one Deleted workspace
  - When: `repository.list_active()` is called
  - Then: Returns exactly 1 workspace (the Active one)

### Empty States

- `test_list_returns_empty_vec_when_no_workspaces`
  - Given: An empty database
  - When: `repository.list()` is called
  - Then: Returns `Ok(Vec::new())`

- `test_list_active_returns_empty_vec_when_no_active_workspaces`
  - Given: Database with only Deleted workspaces
  - When: `repository.list_active()` is called
  - Then: Returns `Ok(Vec::new())`

## Property-Based Tests (Proptest)

These tests use proptest to verify properties across many input values.

### WorkspaceId Properties

- `prop_workspace_id_generate_always_produces_ws_prefix`
  - Given: Generating 1000 workspace IDs
  - When: Each ID is generated via `WorkspaceId::generate()`
  - Then: All IDs start with "ws-" prefix
  - Strategy: `prop::collection::vec(any::<()>(), 1000).prop_map(|_| WorkspaceId::generate())`

- `prop_workspace_id_generate_produces_valid_uuid_format`
  - Given: Generating 1000 workspace IDs
  - When: Each ID is parsed
  - Then: All IDs match regex `^ws-[0-9a-f-]{36}$`
  - Strategy: `prop::collection::vec(any::<()>(), 1000).prop_map(|_| WorkspaceId::generate())`

- `prop_workspace_id_generate_produces_unique_ids`
  - Given: Generating 100 workspace IDs
  - When: IDs are collected into a HashSet
  - Then: HashSet size equals number of generated IDs (high probability)
  - Strategy: `prop::collection::vec(any::<()>(), 100).prop_map(|_| WorkspaceId::generate())`

### WorkspaceName Properties

- `prop_workspace_name_accepts_valid_non_empty_strings`
  - Given: Generated non-empty strings (1-255 chars, printable ASCII)
  - When: `WorkspaceName::new(string)` is called
  - Then: Returns `Ok(WorkspaceName)`
  - Strategy: `any::<String>().prop_filter("non-empty", |s| !s.is_empty()).prop_map(WorkspaceName::new)`

- `prop_workspace_name_rejects_empty_string`
  - Given: Empty string
  - When: `WorkspaceName::new("")` is called
  - Then: Returns `Err(WorkspaceError::InvalidWorkspaceName(...))`
  - Strategy: Just("".to_string())

- `prop_workspace_name_accepts_unicode_characters`
  - Given: Valid unicode strings
  - When: `WorkspaceName::new(unicode_string)` is called
  - Then: Returns `Ok(WorkspaceName)` if non-empty

### WorkspacePath Properties

- `prop_workspace_path_accepts_absolute_paths`
  - Given: Generated absolute paths (starting with /)
  - When: `WorkspacePath::new(path)` is called
  - Then: Returns `Ok(WorkspacePath)`
  - Strategy: `"/".prop_union(any::<String>()).prop_map(|s| format!("/{}", s))`

- `prop_workspace_path_rejects_relative_paths`
  - Given: Generated relative paths (not starting with /)
  - When: `WorkspacePath::new(path)` is called
  - Then: Returns `Err(WorkspaceError::InvalidWorkspacePath(...))`
  - Strategy: `any::<String>().prop_filter("relative path", |s| !s.starts_with('/'))`

- `prop_workspace_path_normalizes_trailing_slashes`
  - Given: A path with trailing slashes "/path/to/workspace/"
  - When: `WorkspacePath::new(path)` is called
  - Then: Returns path without trailing slash
  - Strategy: `"./test".prop_map(|s| format!("{}/", s))`

## Mutation Testing Framework

Mutation testing should be integrated to verify test quality and catch code regressions.

### Framework: Mutagen

Add `mutagen` to `Cargo.toml`:
```toml
[dev-dependencies]
mutagen = "0.4"
```

### Mutation Testing Strategy

- **Run mutation tests in CI**: `cargo mutagen --test-threads=1`
- **Expected outcome**: All mutations should be detected by existing tests
- **Mutation operators to enable**:
  - `arithmetic` - Change +, -, *, / to other operators
  - `bitwise` - Change &, |, ^ to other operators
  - `comparison` - Change ==, !=, <, > to other comparisons
  - `logical` - Change &&, || to other logical operators
  - `negation` - Remove ! prefix
  - `constant` - Change constant values
  - `function` - Remove function calls

### Mutation Test Verification

- `mutation_test_arithmetic_operators_are_covered`
  - Verify: Tests catch changes to arithmetic operations in state machine logic
  
- `mutation_test_comparison_operators_are_covered`
  - Verify: Tests catch changes to state comparison logic

- `mutation_test_error_paths_are_covered`
  - Verify: Tests catch missing error handling branches

### Mutation Testing Configuration

Add to `rust-toolchain.toml` or `Cargo.toml`:
```toml
[mutagen]
version = 2
entry = "cargo"
args = ["mutagen", "run", "--test-threads=1"]
```

### Mutation Coverage Goals

- **State transition logic**: Must have 100% branch coverage
- **Error handling**: All error variants must be exercised
- **Type validation**: All validation paths must be tested

## Contract Verification Tests

### Precondition Tests

- `test_precondition_p3_invalid_workspace_id_returns_error`
  - Given: An empty string
  - When: `WorkspaceId::parse("".into())` is called
  - Then: Returns `Err(WorkspaceError::InvalidWorkspaceId("empty id".into()))`

- `test_precondition_p4_empty_workspace_name_returns_error`
  - Given: An empty string
  - When: `WorkspaceName::new("".into())` is called
  - Then: Returns `Err(WorkspaceError::InvalidWorkspaceName(...))`

- `test_precondition_p5_relative_workspace_path_returns_error`
  - Given: A relative path "relative/path"
  - When: `WorkspacePath::new("relative/path".into())` is called
  - Then: Returns `Err(WorkspaceError::InvalidWorkspacePath(...))`

- `test_precondition_p7_delete_nonexistent_workspace_returns_error`
  - Given: A repository with no workspaces
  - When: `repository.delete(&WorkspaceId::parse("ws-123".into()).unwrap())` is called
  - Then: Returns `Err(WorkspaceError::WorkspaceNotFound("ws-123".into()))`

### Postcondition Tests

- `test_postcondition_q1_table_exists_after_migration`
  - Given: A fresh database and valid SqlitePool
  - When: `migrate_workspaces(pool)` completes
  - Then: Query "SELECT name FROM sqlite_master WHERE type='table' AND name='workspaces'" returns the table

- `test_postcondition_q3_saved_workspace_has_generated_id`
  - Given: A Workspace created without an ID
  - When: Workspace is saved
  - Then: Returned workspace has a non-empty ID starting with "ws-"

- `test_postcondition_q5_get_returns_optional_workspace`
  - Given: A saved workspace
  - When: `repository.get(&id)` is called
  - Then: Returns `Ok(Some(Workspace))` with all fields populated

- `test_postcondition_q10_deleted_workspace_not_in_list_active`
  - Given: A workspace that was deleted
  - When: `repository.list_active()` is called
  - Then: Deleted workspace is not in the results

### Invariant Tests

- `test_invariant_i1_workspace_id_format_enforced`
  - Given: Calling `WorkspaceId::generate()`
  - Then: Always produces ID matching regex `^ws-[0-9a-f-]+$`

- `test_invariant_i2_timestamps_are_chronologically_valid`
  - Given: A newly created workspace
  - Then: `created_at <= updated_at`

- `test_invariant_i3_valid_state_transitions_are_allowed`
  - Given: All valid state transition pairs
  - When: `WorkspaceStateMachine::transition(from, to)` is called
  - Then: Returns `Ok(to)`

- `test_invariant_i3_invalid_state_transitions_are_rejected`
  - Given: Invalid state transition pairs (e.g., Active → Initializing)
  - When: `WorkspaceStateMachine::transition(from, to)` is called
  - Then: Returns `Err(WorkspaceError::InvalidStateTransition(...))`

- `test_invariant_i4_lock_holder_only_when_locked`
  - Given: A workspace in Locked state
  - Then: `lock_holder` is `Some(...)`

- `test_invariant_i4_lock_holder_none_when_not_locked`
  - Given: A workspace in Active state
  - Then: `lock_holder` is `None`

## Contract Violation Tests

Per contract-spec.md, every violation example must have a corresponding test:

- `test_violation_p3_empty_workspace_id_produces_invalid_id_error`
  - Given: Empty string as workspace ID
  - When: `WorkspaceId::parse("".into())` is called
  - Then: Returns `Err(WorkspaceError::InvalidWorkspaceId("empty id".into()))`

- `test_violation_p4_empty_workspace_name_produces_invalid_name_error`
  - Given: Empty string as workspace name
  - When: `WorkspaceName::new("".into())` is called
  - Then: Returns `Err(WorkspaceError::InvalidWorkspaceName(...))`

- `test_violation_p5_relative_path_produces_invalid_path_error`
  - Given: Relative path "relative/path"
  - When: `WorkspacePath::new("relative/path".into())` is called
  - Then: Returns `Err(WorkspaceError::InvalidWorkspacePath(...))`

- `test_violation_p7_delete_nonexistent_workspace_produces_not_found_error`
  - Given: Non-existent workspace ID "ws-nonexistent"
  - When: `repository.delete(&WorkspaceId::parse("ws-nonexistent".into()).unwrap())` is called
  - Then: Returns `Err(WorkspaceError::WorkspaceNotFound("ws-nonexistent".into()))`

- `test_violation_q1_invalid_pool_produces_operation_failed_error`
  - Given: Invalid SqlitePool (connection to unreadable path)
  - When: `migrate_workspaces(invalid_pool)` is called
  - Then: Returns `Err(WorkspaceError::OperationFailed(...))`

- `test_violation_q3_duplicate_name_produces_exists_error`
  - Given: Workspace with name "test" already exists
  - When: Attempting to save another workspace with name "test"
  - Then: Returns `Err(WorkspaceError::WorkspaceExists("test".into()))`

- `test_violation_i3_active_to_initializing_produces_invalid_state_transition_error`
  - Given: A workspace in Active state
  - When: `WorkspaceStateMachine::transition(WorkspaceState::Active, WorkspaceState::Initializing)` is called
  - Then: Returns `Err(WorkspaceError::InvalidStateTransition(...))`

- `test_violation_i3_initializing_to_locked_produces_invalid_state_transition_error`
  - Given: A workspace in Initializing state
  - When: `WorkspaceStateMachine::transition(WorkspaceState::Initializing, WorkspaceState::Locked)` is called
  - Then: Returns `Err(WorkspaceError::InvalidStateTransition(...))`

- `test_violation_i3_deleted_to_active_produces_invalid_state_transition_error`
  - Given: A workspace in Deleted state
  - When: `WorkspaceStateMachine::transition(WorkspaceState::Deleted, WorkspaceState::Active)` is called
  - Then: Returns `Err(WorkspaceError::InvalidStateTransition(...))`

## Given-When-Then Scenario Examples

### Scenario 1: Create and Retrieve Workspace

```
Given: A valid SqlitePool with migrated schema
And:   A new Workspace with name "my-workspace", path "/tmp/my-workspace"
When:  Workspace is saved via repository.save()
And:   Workspace is retrieved via repository.get()
Then:  Retrieved workspace matches saved workspace
And:   ID is generated in format "ws-<uuid>"
And:   State is "Initializing"
And:   Timestamps are set to current time
```

### Scenario 2: Update Workspace State Through Delete

```
Given: An Active workspace in the database
When:  repository.delete(workspace_id) is called
Then:  Returns workspace with state = Deleted
And:   repository.list_active() does not include this workspace
And:   repository.list() still includes this workspace
```

### Scenario 3: Migration Idempotency

```
Given: A database where workspaces table already exists
When:  migrate_workspaces(pool) is called
Then:  Returns Ok(()) without error
And:   No duplicate tables are created
```

## End-to-End Workflow Tests

### Complete Workspace Lifecycle

- `test_e2e_complete_workspace_lifecycle`
  - Given: A valid SqlitePool with migrated schema
  - When:
    1. Create new workspace via `Workspace::new("my-workspace", "/path/to/workspace")`
    2. Save via `repository.save(workspace)` → state is Initializing
    3. Transition to Active via `WorkspaceStateMachine::transition(Initializing, Active)`
    4. Save updated workspace
    5. Transition to Locked via `WorkspaceStateMachine::transition(Active, Locked)`
    6. Transition back to Active via `WorkspaceStateMachine::transition(Locked, Active)`
    7. Delete via `repository.delete(&workspace_id)`
  - Then:
    - Final state is Deleted
    - Workspace appears in `repository.list()`
    - Workspace does NOT appear in `repository.list_active()`

- `test_e2e_workspace_from_creation_to_deletion`
  - Given: A valid SqlitePool with migrated schema
  - When:
    1. Create workspace with name "e2e-test", path "/tmp/e2e-test"
    2. Save workspace → returns workspace with generated ID
    3. Get by ID returns the workspace
    4. Get by name returns the workspace
    5. List active includes the workspace
    6. Delete the workspace
    7. List active excludes the workspace
  - Then: Full lifecycle completes successfully

- `test_e2e_concurrent_workspace_operations`
  - Given: Multiple workspaces in database
  - When:
    1. Create and save workspace "workspace-1"
    2. Create and save workspace "workspace-2"
    3. Create and save workspace "workspace-3"
    4. Lock "workspace-1"
    5. Delete "workspace-2"
  - Then:
    - All three workspaces exist in `repository.list()`
    - Only "workspace-3" appears in `repository.list_active()`
    - "workspace-1" state is Locked
    - "workspace-2" state is Deleted

- `test_e2e_state_machine_with_lock_holder`
  - Given: A new workspace
  - When:
    1. Create and save workspace in Active state
    2. Acquire lock: transition to Locked, set lock_holder to "agent-123"
    3. Attempt operations that require unlocked workspace
    4. Release lock: transition to Active, clear lock_holder
  - Then:
    - Lock acquisition returns workspace with lock_holder = Some("agent-123")
    - Operations while locked return `WorkspaceError::WorkspaceLocked`
    - After release, lock_holder = None
    - Workspace is again mutable

## Test Execution Order

1. Schema migration tests (run once per test suite setup)
2. Happy path tests
3. Error path tests
4. Edge case tests
5. Contract verification tests
6. Contract violation tests

## Test Database Strategy

- Use in-memory SQLite with `sqlite://memory:` for unit tests
- Use temp directory with file-based SQLite for integration tests
- Each test should have isolated database state
- Use transactions with rollback for test isolation where supported
