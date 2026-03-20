# Martin Fowler Test Plan: task start and done commands

## Test Overview

This test plan covers the `task start` and `task done` CLI commands for task lifecycle management.

## Happy Path Tests

### Scenario: Start transitions Open task to InProgress

**test_task_start_transitions_to_in_progress_preserving_assignee**
- Given: Task claimed by "current-user", state Open
- When: `transition_to_started(task)` is called
- Then:
  - Task state is `InProgress`
  - Assignee remains "current-user"
  - `updated_at` is set to current time

**test_task_done_closes_task_with_timestamp**
- Given: Task claimed by "current-user", state InProgress
- When: `transition_to_done(task)` is called
- Then:
  - Task state is `Closed { closed_at }` with timestamp
  - `updated_at` is set to current time

### Scenario: Full task lifecycle

**test_task_claim_assigns_task_to_current_user_and_sets_in_progress**
- Given: Open task, unclaimed
- When: `transition_to_claimed(task, "current-user")` is called
- Then:
  - Task has assignee set to "current-user"
  - State changed to `InProgress`

**test_task_yield_clears_assignee_and_sets_state_to_open**
- Given: Task claimed by "current-user"
- When: `transition_to_yielded(task)` is called
- Then:
  - Assignee is None
  - State is `Open`

## Error Path Tests

### Scenario: Invalid task ID

**test_task_show_returns_error_for_invalid_task_id**
- Given: Empty string as task ID
- When: `TaskId::new("")` is called
- Then: Returns `Err(Error::InvalidTaskId("Task ID cannot be empty"))`

**test_task_show_returns_error_for_malformed_task_id**
- Given: "bad id!" as task ID
- When: `TaskId::new("bad id!")` is called
- Then: Returns `Err(Error::InvalidTaskId(...))` containing "alphanumeric"

### Scenario: Task not found

**test_task_show_returns_not_found_for_nonexistent_task**
- Given: Nonexistent task ID "nonexistent"
- When: `validate_task_exists(None, "nonexistent")` is called
- Then: Returns `Err(Error::TaskNotFound("nonexistent"))`

### Scenario: Task already claimed by another user

**test_task_claim_returns_error_when_task_already_claimed**
- Given: Task claimed by "other-user"
- When: `validate_not_claimed_by_other(&task, "current-user")` is called
- Then: Returns `Err(Error::TaskAlreadyClaimed(_, _))`

### Scenario: Task not claimed (cannot yield/start)

**test_task_yield_returns_error_when_task_not_claimed**
- Given: Task with no assignee (Open state)
- When: `validate_claimed_by_user(&task, "current-user")` is called
- Then: Returns `Err(Error::TaskNotClaimed(_))`

### Scenario: Task already closed

**test_task_start_returns_error_for_closed_task**
- Given: Task in Closed state
- When: `validate_not_closed(&task)` is called
- Then: Returns `Err(Error::InvalidTaskStateTransition(_, _))`

**test_task_done_returns_error_for_already_closed_task**
- Given: Task already in Closed state
- When: `validate_not_closed(&task)` is called
- Then: Returns `Err(Error::InvalidTaskStateTransition(_, _))`

## Edge Case Tests

### Scenario: Idempotent claim

**test_task_claim_idempotent_when_already_claimed_by_same_user**
- Given: Task already claimed by "current-user"
- When: `transition_to_claimed(task, "current-user")` is called again
- Then: Returns success, state remains `InProgress`, assignee unchanged

### Scenario: Valid task ID formats

**test_invariant_i1_valid_task_ids_are_accepted**
- Given: Valid task IDs: ["task-001", "bead_123", "ABC-123_xyz", "a", "1-2_3"]
- When: `TaskId::new(id)` is called for each
- Then: All return `Ok`

## Contract Verification Tests

### Precondition Tests

**test_precondition_p1_empty_id_rejected_at_type_level**
- Given: Empty string ""
- When: `TaskId::new("")` is called
- Then: Returns `Err` with message containing "empty"

**test_precondition_p1_malformed_id_rejected_at_type_level**
- Given: "bad id!" (contains space and !)
- When: `TaskId::new("bad id!")` is called
- Then: Returns `Err` with message containing "alphanumeric"

**test_precondition_p2_nonexistent_task_returns_not_found**
- Given: `None` as task option, "nonexistent" as ID
- When: `validate_task_exists(None, "nonexistent")` is called
- Then: Returns `Err(Error::TaskNotFound(_))`

**test_precondition_p3_already_claimed_prevents_claim**
- Given: Task claimed by "other-user"
- When: `validate_not_claimed_by_other(&task, "current-user")` is called
- Then: Returns `Err(Error::TaskAlreadyClaimed(_, _))`

**test_precondition_p3_claim_succeeds_for_same_user**
- Given: Task claimed by "current-user"
- When: `validate_not_claimed_by_other(&task, "current-user")` is called
- Then: Returns `Ok`

**test_precondition_p4_must_be_claimed_before_yield**
- Given: Task not claimed (Open state, no assignee)
- When: `validate_claimed_by_user(&task, "current-user")` is called
- Then: Returns `Err(Error::TaskNotClaimed(_))`

**test_precondition_p4_yield_succeeds_when_claimed**
- Given: Task claimed by "current-user"
- When: `validate_claimed_by_user(&task, "current-user")` is called
- Then: Returns `Ok`

### Postcondition Tests

**test_postcondition_q3_claim_sets_assignee_and_in_progress**
- Given: Open task
- When: `transition_to_claimed(task, "current-user")` is called
- Then:
  - assignee is `Some(Assignee("current-user"))`
  - state is `TaskState::InProgress`

**test_postcondition_q4_yield_clears_assignee_and_sets_open**
- Given: InProgress task with assignee
- When: `transition_to_yielded(task)` is called
- Then:
  - assignee is `None`
  - state is `TaskState::Open`

**test_postcondition_q6_done_sets_closed_with_timestamp**
- Given: InProgress task
- When: `transition_to_done(task)` is called between `before` and `after` timestamps
- Then:
  - state is `TaskState::Closed { closed_at }` where `closed_at >= before && closed_at <= after`

### Invariant Tests

**test_invariant_i1_valid_task_ids_are_accepted**
- Given: Various valid task ID formats
- When: `TaskId::new()` is called for each
- Then: All return `Ok`

**test_invariant_i2_cannot_close_already_closed_task**
- Given: Closed task
- When: `validate_not_closed(&task)` is called
- Then: Returns `Err(Error::InvalidTaskStateTransition(_, _))`

## Given-When-Then Scenarios

### Scenario 1: Agent starts a claimed task

**Scenario: Agent starts an Open task that was previously claimed**
- Given: Task "bead-123" exists, is claimed by "agent-1", state is Open
- And: Agent "agent-1" is authenticated
- When: User executes `scp task start bead-123 --user agent-1`
- Then:
  - Command returns exit code 0
  - Task "bead-123" state is now `InProgress`
  - Task "bead-123" assignee remains "agent-1"
  - Output shows "Task bead-123 started"

### Scenario 2: Agent completes an InProgress task

**Scenario: Agent marks an InProgress task as done**
- Given: Task "bead-123" exists, is claimed by "agent-1", state is `InProgress`
- And: Agent "agent-1" is authenticated
- When: User executes `scp task done bead-123 --user agent-1`
- Then:
  - Command returns exit code 0
  - Task "bead-123" state is now `Closed` with `closed_at` timestamp
  - Output shows "Task bead-123 completed"

### Scenario 3: Agent tries to start unclaimed task

**Scenario: Agent cannot start a task they haven't claimed**
- Given: Task "bead-123" exists, has no assignee, state is Open
- And: Agent "agent-1" is authenticated
- When: User executes `scp task start bead-123 --user agent-1`
- Then:
  - Command returns non-zero exit code
  - Error message indicates task not claimed
  - Task "bead-123" state remains `Open`

### Scenario 4: Agent tries to complete already closed task

**Scenario: Agent cannot close an already closed task**
- Given: Task "bead-123" exists, is claimed by "agent-1", state is `Closed { closed_at }`
- And: Agent "agent-1" is authenticated
- When: User executes `scp task done bead-123 --user agent-1`
- Then:
  - Command returns non-zero exit code
  - Error message indicates task already closed
  - Task "bead-123" state remains `Closed`

## Test Execution

All tests are located in:
- `crates/cli/src/commands/task_validation.rs` (lines 102-447)

Run with:
```bash
cargo test --package scp-cli --lib commands::task_validation
```
