# Martin Fowler Test Plan: Task Claim and Yield Commands

## Overview

This test plan follows the Given-When-Then pattern from Martin Fowler's "Skills Matter" approach. Each test is executable documentation that specifies behavior.

---

## Happy Path Tests

### Scenario 1: Agent claims available task successfully
**ID:** test_happy_claim_available_task  
**Given:** A task "task-001" exists in Open state with no assignee  
**When:** Agent "agent-a" executes `task claim task-001`  
**Then:**
- Lock is acquired with TTL
- Task assignee is set to "agent-a"
- Task state is InProgress
- Success message is printed

### Scenario 2: Agent yields claimed task successfully
**ID:** test_happy_yield_claimed_task  
**Given:** A task "task-001" is claimed by "agent-a" (state: InProgress, assignee: "agent-a")  
**When:** Agent "agent-a" executes `task yield task-001`  
**Then:**
- Lock is released
- Task assignee is cleared (None)
- Task state is Open
- Success message is printed

### Scenario 3: Agent starts work on claimed task
**ID:** test_happy_start_claimed_task  
**Given:** A task "task-001" is claimed by "agent-a" (state: Open, assignee: "agent-a")  
**When:** Agent "agent-a" executes `task start task-001`  
**Then:**
- Task state transitions to InProgress
- Assignee remains "agent-a"
- Success message is printed

### Scenario 4: Agent completes task
**ID:** test_happy_complete_task  
**Given:** A task "task-001" is in InProgress state, claimed by "agent-a"  
**When:** Agent "agent-a" executes `task done task-001`  
**Then:**
- Task state transitions to Closed with closed_at timestamp
- Lock is released
- Success message is printed

---

## Error Path Tests

### Scenario 5: Claim fails for nonexistent task
**ID:** test_error_claim_nonexistent_task  
**Given:** No task with ID "nonexistent" exists  
**When:** Agent "agent-a" executes `task claim nonexistent`  
**Then:**
- Returns Err(Error::TaskNotFound("nonexistent"))
- Exit code: 60

### Scenario 6: Claim fails when already claimed by another agent
**ID:** test_error_claim_already_claimed  
**Given:** Task "task-001" is claimed by "agent-a"  
**When:** Agent "agent-b" executes `task claim task-001`  
**Then:**
- Returns Err(Error::TaskAlreadyClaimed("task-001", "agent-a"))
- Exit code: 61

### Scenario 7: Yield fails when task not claimed
**ID:** test_error_yield_not_claimed  
**Given:** Task "task-001" is in Open state (no assignee)  
**When:** Agent "agent-a" executes `task yield task-001`  
**Then:**
- Returns Err(Error::TaskNotClaimed("task-001"))
- Exit code: 62

### Scenario 8: Yield fails when claimed by different agent
**ID:** test_error_yield_not_owner  
**Given:** Task "task-001" is claimed by "agent-a"  
**When:** Agent "agent-b" executes `task yield task-001`  
**Then:**
- Returns Err(Error::TaskNotClaimed("task-001"))
- Exit code: 62

### Scenario 9: Claim fails for invalid task ID format
**ID:** test_error_claim_invalid_id_empty  
**Given:** N/A  
**When:** Agent "agent-a" executes `task claim ""`  
**Then:**
- Returns Err(Error::InvalidTaskId("Task ID cannot be empty"))
- Exit code: 64

### Scenario 10: Claim fails for malformed task ID
**ID:** test_error_claim_invalid_id_malformed  
**Given:** N/A  
**When:** Agent "agent-a" executes `task claim "bad id!"`  
**Then:**
- Returns Err(Error::InvalidTaskId("Task ID must be alphanumeric with - or _"))
- Exit code: 64

### Scenario 11: Done fails for closed task
**ID:** test_error_done_already_closed  
**Given:** Task "task-001" is in Closed state  
**When:** Agent "agent-a" executes `task done task-001`  
**Then:**
- Returns Err(Error::InvalidTaskStateTransition("task-001", "Task is already closed"))
- Exit code: 65

### Scenario 12: Lock acquisition fails (concurrent claim)
**ID:** test_error_lock_acquisition_failure  
**Given:** Task "task-001" lock is held by "agent-a"  
**When:** Agent "agent-b" attempts to claim (lock conflict)  
**Then:**
- Returns Err(Error::TaskLocked("task-001"))
- Exit code: 63

---

## Edge Case Tests

### Scenario 13: Idempotent claim by same agent
**ID:** test_edge_idempotent_claim_same_agent  
**Given:** Task "task-001" is already claimed by "agent-a" (state: InProgress, assignee: "agent-a")  
**When:** Agent "agent-a" executes `task claim task-001` again  
**Then:**
- Returns success (idempotent)
- State remains InProgress
- Assignee remains "agent-a"

### Scenario 14: Task state transitions are atomic
**ID:** test_edge_atomic_state_transition  
**Given:** Task "task-001" in Open state  
**When:** Multiple concurrent claim attempts  
**Then:**
- Only one agent succeeds
- Others receive TaskAlreadyClaimed or TaskLocked error
- Task is never in inconsistent state

### Scenario 15: Lock released on panic (Drop guarantee)
**ID:** test_edge_lock_released_on_drop  
**Given:** Agent "agent-a" acquires lock on "task-001"  
**When:** LockGuard is dropped  
**Then:**
- Lock is released
- Other agents can claim

### Scenario 16: Empty task list
**ID:** test_edge_empty_task_list  
**Given:** No tasks exist  
**When:** Agent "agent-a" executes `task list`  
**Then:**
- Prints "No tasks found"
- Initializes demo tasks
- Returns Ok

---

## Contract Verification Tests

### Precondition P1: Valid Task ID format
**test_contract_p1_empty_id_rejected_at_type_level**
- Given: Empty string ""
- When: TaskId::new() is called
- Then: Returns Err containing "empty"

**test_contract_p1_malformed_id_rejected_at_type_level**
- Given: String "bad id!"
- When: TaskId::new() is called
- Then: Returns Err containing "alphanumeric"

### Precondition P2: Task existence
**test_contract_p2_nonexistent_task_returns_not_found**
- Given: None (task not found)
- When: validate_task_exists(None, "nonexistent") is called
- Then: Returns Err(Error::TaskNotFound(_))

### Precondition P3: Not claimed by other
**test_contract_p3_already_claimed_prevents_claim**
- Given: Task claimed by "other-user"
- When: validate_not_claimed_by_other(&task, "current-user") is called
- Then: Returns Err(Error::TaskAlreadyClaimed(_, "other-user"))

**test_contract_p3_claim_succeeds_for_same_user**
- Given: Task claimed by "current-user"
- When: validate_not_claimed_by_other(&task, "current-user") is called
- Then: Returns Ok

### Precondition P4: Claimed by user
**test_contract_p4_must_be_claimed_before_yield**
- Given: Task not claimed (Open state, no assignee)
- When: validate_claimed_by_user(&task, "current-user") is called
- Then: Returns Err(Error::TaskNotClaimed(_))

**test_contract_p4_yield_succeeds_when_claimed**
- Given: Task claimed by "current-user"
- When: validate_claimed_by_user(&task, "current-user") is called
- Then: Returns Ok

### Postcondition Q2/Q3: Claim sets assignee and InProgress
**test_contract_postcondition_claim_sets_assignee_and_in_progress**
- Given: Open task
- When: transition_to_claimed(task, "agent-a") is called
- Then:
  - assignee == Some(Assignee("agent-a"))
  - state == InProgress

### Postcondition Q5/Q6: Yield clears assignee and sets Open
**test_contract_postcondition_yield_clears_assignee_and_sets_open**
- Given: InProgress task with assignee
- When: transition_to_yielded(task) is called
- Then:
  - assignee == None
  - state == Open

### Postcondition Q6: Done sets Closed with timestamp
**test_contract_postcondition_done_sets_closed_with_timestamp**
- Given: InProgress task
- When: transition_to_done(task) is called
- Then:
  - state == Closed { closed_at: DateTime<Utc> }
  - closed_at is within expected range

### Invariant I1: Exclusive claim
**test_invariant_exclusive_claim**
- Given: Two different agents attempting to claim same task
- When: Both call claim simultaneously
- Then: Only one succeeds, other gets TaskAlreadyClaimed

### Invariant I3: TTL prevents indefinite blocking
**test_invariant_ttl_prevents_indefinite_blocking**
- Given: Lock acquired with TTL
- When: TTL expires (300 seconds pass)
- Then: Lock is automatically released

---

## End-to-End Scenario Tests

### Scenario E2E 1: Complete task lifecycle
**test_e2e_complete_task_lifecycle**
1. Agent "agent-a" creates/lists tasks → finds "task-001" in Open state
2. Agent "agent-a" claims "task-001" → succeeds, state becomes InProgress
3. Agent "agent-a" starts "task-001" → state remains InProgress
4. Agent "agent-a" completes "task-001" → state becomes Closed
5. Verify: Task is Closed, lock is released

### Scenario E2E 2: Agent takeover after timeout
**test_e2e_agent_takeover_after_timeout**
1. Agent "agent-a" claims "task-001" → succeeds
2. (TTL expires - 300 seconds)
3. Agent "agent-b" claims "task-001" → succeeds (lock was released)
4. Verify: "agent-a" can no longer yield/start/complete "task-001"

### Scenario E2E 3: Concurrent claim conflict
**test_e2e_concurrent_claim_conflict**
1. Agent "agent-a" and "agent-b" both see "task-001" in Open state
2. Agent "agent-a" claims "task-001" → succeeds (lock acquired first)
3. Agent "agent-b" claims "task-001" → fails with TaskAlreadyClaimed
4. Verify: "agent-a" is assignee, "agent-b" cannot modify

---

## Given-When-Then Summary Table

| Test ID | Given | When | Then |
|---------|-------|------|------|
| test_happy_claim_available_task | Open task, no assignee | claim | assignee=agent-a, state=InProgress |
| test_happy_yield_claimed_task | InProgress task, assignee=agent-a | yield | assignee=None, state=Open |
| test_happy_start_claimed_task | Open task, assignee=agent-a | start | state=InProgress |
| test_happy_complete_task | InProgress task, assignee=agent-a | done | state=Closed |
| test_error_claim_nonexistent_task | Nonexistent task | claim | TaskNotFound |
| test_error_claim_already_claimed | Claimed by agent-a | claim (agent-b) | TaskAlreadyClaimed |
| test_error_yield_not_claimed | Open task, no assignee | yield | TaskNotClaimed |
| test_error_yield_not_owner | Claimed by agent-a | yield (agent-b) | TaskNotClaimed |
| test_error_claim_invalid_id_empty | N/A | claim "" | InvalidTaskId |
| test_error_claim_invalid_id_malformed | N/A | claim "bad!" | InvalidTaskId |
| test_error_done_already_closed | Closed task | done | InvalidTaskStateTransition |
| test_error_lock_acquisition_failure | Lock held | claim | TaskLocked |
| test_edge_idempotent_claim_same_agent | Claimed by agent-a | claim (agent-a) | success |
| test_edge_atomic_state_transition | Open task | concurrent claims | one succeeds, others fail |
| test_edge_lock_released_on_drop | LockGuard exists | drop | lock released |
| test_edge_empty_task_list | No tasks | list | init demo, return Ok |
