# Martin Fowler Test Plan: cli: Add task management commands

## Happy Path Tests
- `test_task_list_returns_all_tasks_when_repository_has_beads`
  - Given: Repository with 3 beads in various states
  - When: task_list() is called
  - Then: Returns all 3 beads with correct state and assignee info

- `test_task_show_returns_complete_details_for_existing_task`
  - Given: Repository with a bead having all fields populated
  - When: task_show("bead-123") is called
  - Then: Displays id, title, description, state, priority, assignee, created_at, updated_at

- `test_task_claim_assigns_task_to_current_user_and_sets_in_progress`
  - Given: Repository with an Open task, unclaimed
  - When: task_claim("bead-123") is called
  - Then: Task has assignee set to current user, state changed to InProgress

- `test_task_yield_clears_assignee_and_sets_state_to_open`
  - Given: Repository with a task claimed by current user
  - When: task_yield("bead-123") is called
  - Then: Task has assignee cleared (None), state changed to Open

- `test_task_start_transitions_to_in_progress_preserving_assignee`
  - Given: Repository with a task claimed by current user, state Open
  - When: task_start("bead-123") is called
  - Then: Task state is InProgress, assignee remains unchanged

- `test_task_done_closes_task_with_timestamp`
  - Given: Repository with a task claimed by current user, state InProgress
  - When: task_done("bead-123") is called
  - Then: Task state is Closed with closed_at set to current time

## Error Path Tests
- `test_task_show_returns_error_for_invalid_task_id`
  - Given: No specific preconditions
  - When: task_show("") is called
  - Then: Returns `Err(Error::InvalidInput)` with message about empty ID

- `test_task_show_returns_error_for_malformed_task_id`
  - Given: No specific preconditions
  - When: task_show("bad id!") is called
  - Then: Returns `Err(Error::InvalidInput)` with message about invalid characters

- `test_task_show_returns_not_found_for_nonexistent_task`
  - Given: Repository without bead "nonexistent"
  - When: task_show("nonexistent") is called
  - Then: Returns `Err(Error::NotFound)` with task ID in message

- `test_task_claim_returns_error_when_task_already_claimed`
  - Given: Repository with task claimed by "other-user"
  - When: task_claim("bead-123") is called as "current-user"
  - Then: Returns `Err(Error::TaskAlreadyClaimed)` with task ID and current holder

- `test_task_yield_returns_error_when_task_not_claimed`
  - Given: Repository with task that has no assignee (or different assignee)
  - When: task_yield("bead-123") is called as "current-user"
  - Then: Returns `Err(Error::TaskNotClaimed)`

- `test_task_start_returns_error_for_closed_task`
  - Given: Repository with task in Closed state
  - When: task_start("bead-123") is called
  - Then: Returns `Err(Error::InvalidStateTransition)`

- `test_task_done_returns_error_for_already_closed_task`
  - Given: Repository with task already in Closed state
  - When: task_done("bead-123") is called
  - Then: Returns `Err(Error::InvalidStateTransition)`

- `test_task_claim_returns_lock_error_when_lock_unavailable`
  - Given: Lock already held by another process for this task
  - When: task_claim("bead-123") is called
  - Then: Returns `Err(Error::TaskLocked)`

## Edge Case Tests
- `test_task_list_handles_empty_repository_gracefully`
  - Given: Empty repository
  - When: task_list() is called
  - Then: Displays "No tasks found" or empty list message

- `test_task_show_displays_all_fields_when_optional_fields_present`
  - Given: Repository with bead having description, priority, labels
  - When: task_show("bead-123") is called
  - Then: All fields are displayed in output

- `test_task_list_respects_priority_ordering`
  - Given: Repository with beads in priority order P0, P2, P4
  - When: task_list() is called
  - Then: Tasks displayed in priority order (or as configured)

- `test_task_claim_idempotent_when_already_claimed_by_same_user`
  - Given: Repository with task already claimed by current user
  - When: task_claim("bead-123") is called again
  - Then: Returns success (idempotent), state remains InProgress

## Contract Verification Tests
- `test_precondition_p1_empty_id_rejected_at_type_level`
  - Given: Input ""
  - When: BeadId::new("") is called
  - Then: Returns Err(BeadError::InvalidId("ID cannot be empty"))

- `test_precondition_p1_malformed_id_rejected_at_type_level`
  - Given: Input "bad id!"
  - When: BeadId::new("bad id!") is called
  - Then: Returns Err with invalid character message

- `test_precondition_p2_nonexistent_task_returns_not_found`
  - Given: Repository without task "nonexistent"
  - When: get_task(&repo, &BeadId::new("nonexistent").unwrap())
  - Then: Returns Err(Error::NotFound)

- `test_precondition_p3_already_claimed_prevents_claim`
  - Given: Task claimed by "other-user"
  - When: claim_task with holder="current-user"
  - Then: Returns Err(Error::TaskAlreadyClaimed)

- `test_precondition_p4_must_be_claimed_before_yield`
  - Given: Task not claimed
  - When: yield_task with holder="current-user"
  - Then: Returns Err(Error::TaskNotClaimed)

- `test_postcondition_q1_list_returns_all_tasks`
  - Given: Repository with N tasks
  - When: list_tasks() is called
  - Then: Returns exactly N tasks

- `test_postcondition_q3_claim_sets_assignee_and_in_progress`
  - Given: Open task
  - When: claim_task completes successfully
  - Then: Resulting bead has assignee=current_user AND state=InProgress

- `test_postcondition_q4_yield_clears_assignee_and_sets_open`
  - Given: InProgress task with assignee
  - When: yield_task completes successfully
  - Then: Resulting bead has assignee=None AND state=Open

- `test_postcondition_q6_done_sets_closed_with_timestamp`
  - Given: InProgress task
  - When: complete_task completes successfully
  - Then: Resulting bead has state=Closed{closed_at} with timestamp

- `test_invariant_i3_lock_released_even_on_error`
  - Given: Lock is acquired, operation fails mid-way
  - When: Error occurs after lock acquired
  - Then: Lock is released (via Drop or explicit release)

## Given-When-Then Scenarios

### Scenario 1: User claims an available task
**Scenario**: User claims an unclaimed task to start working on it
- Given: A task "bead-123" exists with state Open and no assignee
- When: User runs `scp task claim bead-123`
- Then: Task "bead-123" now has assignee="current-user" and state=InProgress
- And: Success message "Task bead-123 claimed" is displayed

### Scenario 2: User yields an assigned task
**Scenario**: User releases their claim on a task
- Given: A task "bead-123" has assignee="current-user" and state=InProgress
- When: User runs `scp task yield bead-123`
- Then: Task "bead-123" now has assignee=None and state=Open
- And: Success message "Task bead-123 yielded" is displayed

### Scenario 3: User tries to claim another user's task
**Scenario**: Attempt to claim a task already assigned to someone else
- Given: A task "bead-123" has assignee="other-user" and state=InProgress
- When: User runs `scp task claim bead-123`
- Then: Error "Task bead-123 is already claimed by other-user" is displayed
- And: Task state remains unchanged

### Scenario 4: User completes a task
**Scenario**: User finishes working on a task and marks it done
- Given: A task "bead-123" has assignee="current-user" and state=InProgress
- When: User runs `scp task done bead-123`
- Then: Task "bead-123" now has state=Closed{closed_at}
- And: Success message "Task bead-123 completed" is displayed

### Scenario 5: Lock acquisition with TTL
**Scenario**: Task operations acquire TTL lock to prevent race conditions
- Given: No lock held for task "bead-123"
- When: User runs `scp task start bead-123`
- Then: Lock is acquired with TTL before state transition
- And: Lock is released after operation completes
