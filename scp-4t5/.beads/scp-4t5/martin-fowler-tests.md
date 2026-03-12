# Martin Fowler Test Plan

## Bead ID: scp-4t5
## Feature: cli: Add task management commands with TTL locking

## Happy Path Tests

### Scenario: List tasks when store is empty
- **test_list_tasks_initializes_demo_tasks**
  Given: Task store is empty (no tasks)
  When: `list()` is called
  Then: Initializes demo tasks and displays them
  Then: Returns `Ok(())`

### Scenario: List tasks when store has tasks
- **test_list_tasks_returns_all_tasks**
  Given: Task store contains task-001, task-002, task-003
  When: `list()` is called
  Then: Returns all tasks in list format
  Then: Returns `Ok(())`

### Scenario: Show existing task
- **test_show_returns_task_details**
  Given: Task "task-001" exists with title "Implement auth"
  When: `show("task-001")` is called
  Then: Displays task ID, title, state, assignee, timestamps
  Then: Returns `Ok(())`

### Scenario: Claim unclaimed task
- **test_claim_unclaimed_task_succeeds**
  Given: Task "task-001" exists with state Open, no assignee
  When: `claim("task-001")` is called by "user-a"
  Then: Task state becomes InProgress
  Then: Task assignee becomes "user-a"
  Then: Prints "Task task-001 claimed"
  Then: Returns `Ok(())`

### Scenario: Claim task already claimed by self (idempotent)
- **test_claim_already_claimed_by_self_succeeds**
  Given: Task "task-001" is claimed by "user-a"
  When: `claim("task-001")` is called by "user-a"
  Then: Returns `Ok(())` (idempotent)
  Then: Prints "Task task-001 claimed"

### Scenario: Yield claimed task
- **test_yield_releases_assignment**
  Given: Task "task-001" is claimed by "user-a" with state InProgress
  When: `yield_task("task-001")` is called by "user-a"
  Then: Task state becomes Open
  Then: Task assignee becomes None
  Then: Prints "Task task-001 yielded"
  Then: Returns `Ok(())`

### Scenario: Start work on claimed task
- **test_start_transitions_to_in_progress**
  Given: Task "task-001" is claimed by "user-a" with state Open
  When: `start("task-001")` is called by "user-a"
  Then: Task state becomes InProgress
  Then: Returns `Ok(())`
  Then: Prints "Task task-001 started"

### Scenario: Complete claimed task
- **test_done_closes_task**
  Given: Task "task-001" is claimed by "user-a" with state InProgress
  When: `done("task-001")` is called by "user-a"
  Then: Task state becomes Closed { closed_at: <now> }
  Then: Returns `Ok(())`
  Then: Prints "Task task-001 completed"

## Error Path Tests

### Scenario: Show task with empty ID
- **test_show_empty_id_returns_error**
  Given: Valid task store
  When: `show("")` is called
  Then: Returns `Err(Error::InvalidTaskId("Task ID cannot be empty"))`

### Scenario: Show non-existent task
- **test_show_nonexistent_returns_error**
  Given: Valid task store without "nonexistent"
  When: `show("nonexistent")` is called
  Then: Returns `Err(Error::TaskNotFound("nonexistent"))`

### Scenario: Claim task locked by another user
- **test_claim_locked_task_returns_error**
  Given: Task "task-001" is locked by "user-a"
  When: `claim("task-001")` is called by "user-b"
  Then: Returns `Err(Error::TaskLocked("task-001"))`

### Scenario: Claim task already claimed by another
- **test_claim_already_claimed_by_other_returns_error**
  Given: Task "task-001" is claimed by "user-a"
  When: `claim("task-001")` is called by "user-b"
  Then: Returns `Err(Error::TaskAlreadyClaimed("task-001", "user-a"))`

### Scenario: Yield task not claimed
- **test_yield_unclaimed_returns_error**
  Given: Task "task-001" has no assignee
  When: `yield_task("task-001")` is called
  Then: Returns `Err(Error::TaskNotClaimed("task-001"))`

### Scenario: Yield task claimed by another
- **test_yield_other_users_task_returns_error**
  Given: Task "task-001" is claimed by "user-a"
  When: `yield_task("task-001")` is called by "user-b"
  Then: Returns `Err(Error::TaskNotClaimed("task-001"))`

### Scenario: Start task not claimed
- **test_start_unclaimed_returns_error**
  Given: Task "task-001" has no assignee
  When: `start("task-001")` is called
  Then: Returns `Err(Error::TaskNotClaimed("task-001"))`

### Scenario: Start task claimed by another
- **test_start_other_users_task_returns_error**
  Given: Task "task-001" is claimed by "user-a"
  When: `start("task-001")` is called by "user-b"
  Then: Returns `Err(Error::TaskNotClaimed("task-001"))`

### Scenario: Done task not claimed
- **test_done_unclaimed_returns_error**
  Given: Task "task-001" has no assignee
  When: `done("task-001")` is called
  Then: Returns `Err(Error::TaskNotClaimed("task-001"))`

### Scenario: Done task already closed
- **test_done_already_closed_returns_error**
  Given: Task "task-001" is in Closed state
  When: `done("task-001")` is called
  Then: Returns `Err(Error::InvalidTaskStateTransition("task-001", "Task is already closed"))`

## Edge Case Tests

### Scenario: List with many tasks
- **test_list_handles_many_tasks**
  Given: Task store has 100 tasks
  When: `list()` is called
  Then: Returns all 100 tasks
  Then: Returns `Ok(())`

### Scenario: Concurrent claim attempts
- **test_concurrent_claim_only_one_succeeds**
  Given: Task "task-001" is unclaimed
  When: Two concurrent `claim("task-001")` calls by different users
  Then: One succeeds, one fails with TaskLocked
  Then: Task is claimed by exactly one user

### Scenario: Lock expires after TTL
- **test_lock_expires_after_ttl**
  Given: Lock acquired with 1 second TTL
  When: Wait 2 seconds
  Then: Lock is automatically released
  Then: Another user can claim the task

## Contract Verification Tests

### Precondition: Task ID non-empty
- **test_precondition_task_id_not_empty**
  Given: Empty string as task_id
  When: `show("")` is called
  Then: Returns error (not panic)
  Then: Error variant is `Error::InvalidTaskId`

### Precondition: Task exists
- **test_precondition_task_exists**
  Given: Task "nonexistent" does not exist
  When: `show("nonexistent")` is called
  Then: Returns `Err(Error::TaskNotFound("nonexistent"))`

### Precondition: Lock acquire
- **test_precondition_lock_not_held**
  Given: Task "task-001" is locked by "user-a"
  When: `claim("task-001")` called by "user-b"
  Then: Returns `Err(Error::TaskLocked("task-001"))`

### Postcondition: State transition on claim
- **test_postcondition_claim_sets_in_progress**
  Given: Task "task-001" is Open, unclaimed
  When: `claim("task-001")` succeeds
  Then: Task state is InProgress
  Then: Task assignee is set

### Postcondition: State transition on done
- **test_postcondition_done_sets_closed**
  Given: Task "task-001" is InProgress, claimed
  When: `done("task-001")` succeeds
  Then: Task state is Closed with closed_at timestamp

### Invariant: Assignee implies InProgress
- **test_invariant_assignee_implies_in_progress**
  Given: Task has Some assignee
  When: Query task state
  Then: State is InProgress

## Given-When-Then Scenarios

### Scenario: Complete task workflow
Given: User "alice" wants to work on a task
When: She calls `claim("task-001")`
Then: Task is assigned to her, state is InProgress

Given: She starts working
When: She calls `start("task-001")`
Then: State remains InProgress (already in progress)

Given: She finishes the work
When: She calls `done("task-001")`
Then: Task state becomes Closed
Then: closed_at is set to current time

### Scenario: Abandon task workflow
Given: User "bob" claimed "task-002"
When: He decides to yield the task
Then: He calls `yield_task("task-002")`
Then: Task is unassigned, state is Open

Given: Later, user "carol" wants the task
When: She calls `claim("task-002")`
Then: Task is now assigned to her
