# Martin Fowler Test Plan: Queue Service Wiring

## Test Metadata

- **Bead ID**: scpm-uzl
- **Bead Title**: queue: wire application services
- **Phase**: Contract Specification
- **Updated**: 2026-03-21

## Happy Path Tests

### Scenario 1: Enqueue creates pending job
**test_enqueue_creates_pending_job**
- Given: A valid `InMemoryQueueRepository` and `QueueService`
- When: `enqueue("session-1", Some("bead-1"), Priority::default())` is called
- Then:
  - Returns `Ok(QueueEntry)` with status `Pending`
  - Entry is persisted in repository
  - Entry has generated ID

### Scenario 2: Dequeue returns claimed job
**test_dequeue_returns_claimed_job**
- Given: A `QueueService` with a pending entry in repository
- When: `dequeue()` is called
- Then:
  - Returns `Ok(Some(QueueEntry))` with status `Claimed`
  - Entry is removed from pending queue

### Scenario 3: Complete job success path
**test_complete_job_success_path**
- Given: A claimed entry
- When: `complete_job(id, true)` is called
- Then:
  - Entry transitions through: Claimed → Rebasing → Testing → ReadyToMerge → Merging → Merged
  - Final status is `Merged`

### Scenario 4: Complete job failure path
**test_complete_job_failure_path**
- Given: A claimed entry
- When: `complete_job(id, false)` is called
- Then:
  - Entry transitions to `FailedRetryable`
  - `retry_count` is incremented
  - `error_message` is set

## Error Path Tests

### Scenario 5: Dequeue empty queue
**test_dequeue_empty_queue_returns_none**
- Given: An empty `QueueService`
- When: `dequeue()` is called
- Then: Returns `Ok(None)`

### Scenario 6: Get non-existent job
**test_get_nonexistent_job_returns_none**
- Given: A `QueueService` with existing jobs
- When: `get_job(&non_existent_id)` is called
- Then: Returns `Ok(None)`

### Scenario 7: Enqueue with empty session ID
**test_enqueue_empty_session_id_returns_error**
- Given: A valid `QueueService`
- When: `enqueue("", None, Priority::default())` is called
- Then: Returns `Err(QueueError::InvalidQueueEntryId("empty id"))`

### Scenario 8: Enqueue with whitespace session ID
**test_enqueue_whitespace_session_id_returns_error**
- Given: A valid `QueueService`
- When: `enqueue("   ", None, Priority::default())` is called
- Then: Returns `Err(QueueError::InvalidQueueEntryId("empty id"))`

### Scenario 9: Claim already claimed job
**test_claim_already_claimed_returns_error**
- Given: A job with status `Claimed`
- When: `claim_job(id)` is called again
- Then: Returns `Err(QueueError::InvalidStateTransition { from: "Claimed", to: "Claimed" })`

### Scenario 10: Complete job not in valid state
**test_complete_invalid_state_returns_error**
- Given: A job with status `Pending`
- When: `complete_job(id, true)` is called
- Then: Returns `Err(QueueError::InvalidStateTransition { from: "Pending", to: ... })`

### Scenario 11: Cancel already merged job
**test_cancel_merged_job_returns_error**
- Given: A job with status `Merged`
- When: `cancel_job(id)` is called
- Then: Returns `Err(QueueError::InvalidStateTransition { from: "Merged", to: "Cancelled" })`

### Scenario 12: Update non-existent job
**test_update_nonexistent_job_returns_error**
- Given: A `QueueService`
- When: `update_job(QueueEntry{...})` is called with non-existent ID
- Then: Returns `Err(RepositoryError("entry not found"))`

## Edge Case Tests

### Scenario 13: Priority ordering preserved
**test_priority_ordering_preserved**
- Given: Multiple entries with different priorities enqueued
- When: `list_pending()` is called
- Then: Returns entries sorted by priority (Critical > High > Default > Low)

### Scenario 14: Concurrent dequeue safety
**test_concurrent_dequeue_is_safe**
- Given: A `QueueService` with entries
- When: Multiple threads call `dequeue()` simultaneously
- Then: No race conditions; each call gets unique entry or None

### Scenario 15: Repository clone independence
**test_repository_clone_is_independent**
- Given: A `QueueService` with a repository
- When: Repository is cloned and modified
- Then: Original service's repository is unaffected

### Scenario 16: Retry count limit
**test_retry_count_limit_respected**
- Given: A `FailedRetryable` entry with `retry_count >= 3`
- When: `can_retry()` is called
- Then: Returns `false`

### Scenario 17: Terminal state immutability
**test_terminal_states_are_immutable**
- Given: Entries in `Merged`, `FailedTerminal`, `Cancelled` states
- When: State transition attempted
- Then: All return `InvalidStateTransition` errors

## Contract Verification Tests

### Scenario 18: State machine validates transitions
**test_state_machine_validates_all_transitions**
- Given: All combinations of `QueueStatus`
- When: `QueueStateMachine::can_transition(from, to)` is called
- Then: Returns true only for valid transitions per state diagram

### Scenario 19: Enqueue persists entry
**test_enqueue_persists_entry**
- Given: A fresh `QueueService`
- When: `enqueue()` is called
- Then: Entry appears in `list_all()`

### Scenario 20: Dequeue removes from pending
**test_dequeue_removes_from_pending**
- Given: A `QueueService` with pending entries
- When: `dequeue()` returns `Some(entry)`
- Then: `list_pending()` does not include that entry

### Scenario 21: Update modifies entry
**test_update_modifies_entry**
- Given: A `QueueService` with an existing entry
- When: `update_job(modified_entry)` is called
- Then: `get_job(id)` returns the modified entry

### Scenario 22: Remove deletes entry
**test_remove_deletes_entry**
- Given: A `QueueService` with an existing entry
- When: `remove_job(id)` is called
- Then: `get_job(id)` returns `None`

## Given-When-Then Scenarios

### Scenario: Full job lifecycle (end-to-end)
**test_full_job_lifecycle**
- Given: A `QueueService` with empty queue
- When:
  1. `enqueue("session-1", Some("bead-1"), Priority::high())` → entry1
  2. `dequeue()` → claimed_entry
  3. `complete_job(claimed_entry.id, true)` → merged_entry
- Then:
  - entry1.status = `Pending`
  - claimed_entry.status = `Claimed` then `Rebasing` then `Testing` then `ReadyToMerge` then `Merging` then `Merged`
  - merged_entry.status = `Merged`
  - merged_entry.error_message = None

### Scenario: Failed job retry lifecycle
**test_failed_job_retry_lifecycle**
- Given: A `QueueService`
- When:
  1. `enqueue("session-1", None, Priority::default())` → entry1
  2. `dequeue()` → claimed
  3. `complete_job(claimed.id, false)` → failed (retry_count = 1)
  4. `retry_entry(failed)` → requeued
- Then:
  - failed.status = `FailedRetryable`
  - requeued.status = `Pending`
  - `can_retry()` returns `true` for failed

### Scenario: Cancel before processing
**test_cancel_before_processing**
- Given: A `QueueService` with a pending entry
- When:
  1. `claim_job(entry.id)` → claimed
  2. `cancel_job(claimed.id)` → cancelled
- Then:
  - cancelled.status = `Cancelled`
  - `is_terminal()` returns `true` for cancelled
