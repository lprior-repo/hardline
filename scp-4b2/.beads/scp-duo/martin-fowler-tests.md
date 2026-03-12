# Martin Fowler Test Plan

## Happy Path Tests

### Workspace Happy Path Tests
- `test_workspace_create_returns_initializing_state`
  Given: Valid workspace name and path
  When: Workspace::create is called
  Then: Returns workspace with state = Initializing

- `test_workspace_activate_transitions_to_active`
  Given: Workspace in Initializing state
  When: workspace.activate() is called
  Then: Returns workspace with state = Active

- `test_workspace_lock_sets_lock_holder`
  Given: Workspace in Active state
  When: workspace.lock("agent-1") is called
  Then: Returns workspace with state = Locked and lock_holder = Some("agent-1")

- `test_workspace_unlock_clears_lock_holder`
  Given: Workspace in Locked state with lock_holder
  When: workspace.unlock() is called
  Then: Returns workspace with state = Active and lock_holder = None

- `test_workspace_delete_transitions_to_deleted`
  Given: Workspace in Active state
  When: workspace.delete() is called
  Then: Returns workspace with state = Deleted

- `test_workspace_mark_corrupted_transitions_to_corrupted`
  Given: Workspace in Active state
  When: workspace.mark_corrupted() is called
  Then: Returns workspace with state = Corrupted

### Bead Happy Path Tests
- `test_bead_create_returns_open_state`
  Given: Valid BeadId and BeadTitle
  When: Bead::create is called
  Then: Returns bead with state = BeadState::Open

- `test_bead_transition_to_inprogress`
  Given: Bead in Open state
  When: bead.transition(InProgress) is called
  Then: Returns bead with state = InProgress

- `test_bead_transition_to_blocked`
  Given: Bead in InProgress state
  When: bead.transition(Blocked) is called
  Then: Returns bead with state = Blocked

- `test_bead_transition_to_closed_sets_timestamp`
  Given: Bead in any non-Closed state
  When: bead.transition(Closed) is called
  Then: Returns bead with state = Closed and closed_at = Some(timestamp)

- `test_bead_with_priority_sets_priority`
  Given: A newly created Bead
  When: bead.with_priority(P1) is called
  Then: Returns bead with priority = Some(P1)

- `test_bead_add_dependency_adds_to_list`
  Given: A Bead
  When: bead.add_dependency("dep-1") is called
  Then: Returns bead with depends_on containing "dep-1"

- `test_bead_add_blocker_adds_to_blocked_by`
  Given: A Bead
  When: bead.add_blocker("blocker-1") is called
  Then: Returns bead with blocked_by containing "blocker-1"

## Error Path Tests

### Workspace Error Path Tests
- `test_workspace_activate_when_already_active_returns_error`
  Given: Workspace in Active state
  When: workspace.activate() is called
  Then: Returns Err(InvalidStateTransition { from: "Active", to: "Active" })

- `test_workspace_lock_when_not_active_returns_error`
  Given: Workspace in Initializing state
  When: workspace.lock("holder") is called
  Then: Returns Err(InvalidStateTransition { from: "Initializing", to: "Locked" })

- `test_workspace_unlock_when_not_locked_returns_error`
  Given: Workspace in Active state
  When: workspace.unlock() is called
  Then: Returns Err(InvalidStateTransition { from: "Active", to: "Active" })

- `test_workspace_delete_when_already_deleted_returns_error`
  Given: Workspace in Deleted state
  When: workspace.delete() is called
  Then: Returns Err(InvalidStateTransition { from: "Deleted", to: "Deleted" })

- `test_workspace_lock_with_empty_holder_returns_error`
  Given: Workspace in Active state
  When: workspace.lock("") is called
  Then: Returns Err (empty holder not allowed - implementation detail)

### Bead Error Path Tests
- `test_bead_create_with_empty_id_returns_error`
  Given: Empty BeadId string
  When: BeadId::new("") is called
  Then: Returns Err(InvalidId("ID cannot be empty"))

- `test_bead_create_with_invalid_characters_returns_error`
  Given: BeadId with invalid characters (e.g., "bead!")
  When: BeadId::new("bead!") is called
  Then: Returns Err(InvalidId containing "alphanumeric")

- `test_bead_create_with_empty_title_returns_error`
  Given: Empty BeadTitle string
  When: BeadTitle::new("") is called
  Then: Returns Err(InvalidTitle("Title cannot be empty"))

- `test_bead_transition_from_closed_returns_error`
  Given: Bead in Closed state
  When: bead.transition(Open) is called
  Then: Returns Err(InvalidStateTransition)

- `test_bead_transition_invalid_state_returns_error`
  Given: Bead in Open state
  When: Attempting invalid transition (if any defined)
  Then: Returns appropriate error

## Edge Case Tests

### Workspace Edge Case Tests
- `test_workspace_create_with_very_long_name`
  Given: Workspace name at maximum allowed length
  When: Workspace::create is called
  Then: Returns Ok with workspace

- `test_workspace_updated_at_changes_on_transition`
  Given: Workspace with known created_at
  When: Any state transition occurs
  Then: updated_at > created_at

- `test_workspace_is_locked_returns_correct_value`
  Given: Workspaces in different states
  When: is_locked() is called
  Then: Returns true only for Locked state

- `test_workspace_is_active_returns_correct_value`
  Given: Workspaces in different states
  When: is_active() is called
  Then: Returns true only for Active state

- `test_workspace_is_terminal_returns_correct_value`
  Given: Workspaces in different states
  When: is_terminal() is called
  Then: Returns true only for Deleted and Corrupted states

### Bead Edge Case Tests
- `test_bead_id_max_length_boundary`
  Given: BeadId with exactly 100 characters
  When: BeadId::new is called
  Then: Returns Ok

- `test_bead_id_exceeds_max_length_returns_error`
  Given: BeadId with 101 characters
  When: BeadId::new is called
  Then: Returns Err(InvalidId containing "maximum length")

- `test_bead_title_max_length_boundary`
  Given: BeadTitle with exactly 200 characters
  When: BeadTitle::new is called
  Then: Returns Ok

- `test_bead_is_blocked_returns_correct_value`
  Given: Beads with empty and non-empty blocked_by lists
  When: is_blocked() is called
  Then: Returns true only when blocked_by is non-empty

- `test_bead_can_transition_to_closed_from_any_state`
  Given: Beads in all states (Open, InProgress, Blocked, Deferred, Closed)
  When: can_transition_to(Closed) is called
  Then: Returns true for all states

- `test_bead_cannot_transition_from_closed_to_any_state`
  Given: Bead in Closed state
  When: can_transition_to(Open) is called
  Then: Returns false

## Contract Verification Tests

### Workspace Contract Tests
- `test_contract_p2_workspace_activate_requires_initializing`
  Given: Workspace in Active state
  When: workspace.activate() is called
  Then: Returns Err(InvalidStateTransition)

- `test_contract_p3_workspace_lock_requires_active`
  Given: Workspace in Initializing state
  When: workspace.lock("holder") is called
  Then: Returns Err(InvalidStateTransition)

- `test_contract_p4_workspace_unlock_requires_locked`
  Given: Workspace in Active state
  When: workspace.unlock() is called
  Then: Returns Err(InvalidStateTransition)

- `test_contract_q1_workspace_create_sets_initializing`
  Given: Valid name and path
  When: Workspace::create is called
  Then: Resulting workspace has state = Initializing

- `test_contract_q2_workspace_activate_sets_active`
  Given: Workspace in Initializing
  When: workspace.activate() is called
  Then: Resulting workspace has state = Active

- `test_contract_q3_workspace_lock_sets_lock_holder`
  Given: Workspace in Active
  When: workspace.lock("agent") is called
  Then: Resulting workspace has lock_holder = Some("agent")

- `test_contract_q6_workspace_delete_sets_deleted`
  Given: Workspace in Active
  When: workspace.delete() is called
  Then: Resulting workspace has state = Deleted

### Bead Contract Tests
- `test_contract_p7_bead_id_validation`
  Given: Various invalid ID strings
  When: BeadId::new is called
  Then: Returns Err for empty, exceeds length, invalid chars

- `test_contract_p7_bead_title_validation`
  Given: Various invalid title strings
  When: BeadTitle::new is called
  Then: Returns Err for empty, exceeds length

- `test_contract_p8_bead_transition_validation`
  Given: Bead in Closed state
  When: bead.transition(Open) is called
  Then: Returns Err(InvalidStateTransition)

- `test_contract_q11_bead_create_sets_open`
  Given: Valid id and title
  When: Bead::create is called
  Then: Resulting bead has state = Open

- `test_contract_q12_bead_transition_to_closed_sets_timestamp`
  Given: Bead in InProgress
  When: bead.transition(Closed) is called
  Then: Resulting bead has state = Closed { closed_at }

- `test_contract_q14_bead_is_blocked_logic`
  Given: Bead with empty blocked_by
  When: is_blocked() is called
  Then: Returns false

- `test_contract_q15_bead_closed_cannot_transition`
  Given: Bead in Closed state
  When: can_transition_to(Open) is called
  Then: Returns false

## Given-When-Then Scenarios

### Scenario 1: Workspace Full Lifecycle
Given: A new workspace with valid name "my-workspace" and path "/tmp/my-workspace"
When: 
  - Workspace is created (state = Initializing)
  - Workspace is activated (state = Active)
  - Workspace is locked by "agent-1" (state = Locked)
  - Workspace is unlocked (state = Active)
  - Workspace is deleted (state = Deleted)
Then:
- Each transition produces the correct state
- Timestamps are properly updated
- is_terminal returns true only at the end

### Scenario 2: Bead Full Lifecycle
Given: A new bead with id "scp-123" and title "Implement feature"
When:
  - Bead is created (state = Open)
  - Bead is transitioned to InProgress
  - Bead is transitioned to Blocked
  - Bead is transitioned back to InProgress
  - Bead is transitioned to Closed
Then:
- Each transition produces the correct state
- Closed state contains timestamp
- is_blocked returns true during Blocked state

### Scenario 3: Blocked Bead Cannot Close Prematurely (if applicable)
Given: A bead with a blocker in blocked_by list
When:
  - Attempting to close the bead
Then:
- Consider whether this should be allowed or blocked by contract

### Scenario 4: Dependency Management
Given: Two beads - "parent-bead" and "child-bead"
When:
  - child-bead.add_dependency(parent-bead.id) is called
Then:
- child-bead.depends_on contains parent-bead.id
- No self-reference in depends_on

## Implementation Notes

- All tests should be in the `#[cfg(test)]` module
- Use expressive test names following the pattern: `test_<subject>_<condition>_<expected_result>`
- Each test should be self-contained and not depend on execution order
- Use `#[must_use]` annotations where appropriate
- All fallible operations must return `Result<T, Error>` - no unwrap/panic in production code
