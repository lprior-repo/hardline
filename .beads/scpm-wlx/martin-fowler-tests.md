# Martin Fowler Test Plan: Job Processing Loop

## Test Naming Convention
All tests follow: `test_<scenario>_<when>_<then>`

## Happy Path Tests

### test_processor_initializes_successfully_given_valid_repository
- Given: A valid JobRepository implementation
- When: JobProcessor::new is called with valid params
- Then: Returns Ok(JobProcessor) with correct configuration

### test_poll_returns_highest_priority_job_first
- Given: Multiple pending jobs with different priorities (P0, P2, P4)
- When: poll_pending_jobs is called
- Then: Returns jobs ordered P0, P2, P4 (descending priority)

### test_execute_job_transitions_to_completed
- Given: A pending job exists
- When: execute_job is called with the job
- Then: Job state transitions to Completed with timestamp

### test_processor_stops_cleanly_on_shutdown_signal
- Given: JobProcessor is running
- When: ShutdownToken is signaled
- Then: run() returns Ok(()) after current job completes

### test_empty_queue_returns_none
- Given: No pending jobs exist
- When: poll_pending_jobs is called
- Then: Returns Ok(None)

## Error Path Tests

### test_poll_returns_error_when_repository_unavailable
- Given: Repository is disconnected/failed
- When: poll_pending_jobs is called
- Then: Returns Err(Error::NoRepository)

### test_execute_job_returns_error_when_job_not_found
- Given: A job ID that doesn't exist
- When: execute_job is called with invalid ID
- Then: Returns Err(Error::JobNotFound)

### test_execute_job_returns_error_for_invalid_state
- Given: A job in Completed state
- When: execute_job is called
- Then: Returns Err(Error::InvalidJobState)

### test_processor_returns_error_on_execution_failure
- Given: A job that will fail during execution
- When: execute_job is called
- Then: Job state transitions to Failed with error message

## Edge Case Tests

### test_poll_respects_concurrency_limit
- Given: 10 jobs pending, concurrency_limit of 3
- When: Multiple poll_calls happen rapidly
- Then: No more than 3 jobs are running simultaneously

### test_poll_returns_single_job_from_multiple_same_priority
- Given: 5 jobs all with P1 priority
- When: poll_pending_jobs is called with limit=1
- Then: Returns exactly 1 job (FIFO among same priority)

### test_handles_job_payload_deserialization_error
- Given: A job with corrupted payload
- When: execute_job is called
- Then: Job transitions to Failed with deserialization error

### test_priority_ordering_stable_under_load
- Given: 100 jobs with random priorities added rapidly
- When: poll_pending_jobs is called 100 times
- Then: All jobs returned in correct priority order

## Contract Verification Tests

### test_precondition_p1_repository_validation
- Given: A null repository reference
- When: JobProcessor::new is called
- Then: Returns Err(Error::NoRepository)

### test_precondition_p2_poll_interval_validation
- Given: Zero duration poll interval
- When: JobProcessor::new is called with Duration::ZERO
- Then: Returns Err(Error::InvalidConfiguration)

### test_postcondition_q1_single_job_returned
- Given: Repository returns 3 jobs
- When: poll_pending_jobs is called with limit=1
- Then: Returns exactly 1 job (not 3)

### test_postcondition_q2_priority_ordering
- Given: Jobs with priorities P4, P0, P2
- When: poll_pending_jobs is called
- Then: Returns in order P0, P2, P4 (not P4, P0, P2)

### test_postcondition_q3_state_transition
- Given: A job in Pending state
- When: execute_job completes successfully
- Then: Job is in Completed state (not Pending)

### test_invariant_i1_concurrency_enforcement
- Given: concurrency_limit of 2
- When: 5 concurrent execute_job calls are made
- Then: Only 2 jobs run simultaneously

### test_invariant_i2_no_duplicate_execution
- Given: A job already running
- When: execute_job is called for same job again
- Then: Returns Err(Error::InvalidJobState)

### test_invariant_i3_priority_stability
- Given: Repeated polls over 1000 iterations
- When: Jobs maintain consistent priority ordering
- Then: No priority inversion occurs

## Contract Violation Tests (One per violation example)

### test_violates_p1_null_repository
- Given: Repository is null
- When: JobProcessor::new(nullptr, interval)
- Then: Err(Error::NoRepository)

### test_violates_p2_zero_interval
- Given: poll_interval is Duration::ZERO
- When: JobProcessor::new(repo, Duration::ZERO)
- Then: Err(Error::InvalidConfiguration)

### test_violates_p3_polling_non_pending_job
- Given: A job in Blocked state
- When: poll_job(BlockedJob)
- Then: Err(Error::InvalidJobState)

### test_violates_q1_multiple_jobs_returned
- Given: Repository returns 2 jobs
- When: poll_pending_job() (single job variant)
- Then: Returns at most 1 job (enforce single retrieval)

### test_violates_q2_priority_inversion
- Given: Jobs with P0 and P1
- When: poll_pending_job()
- Then: P0 returned before P1 (not reversed)

### test_violates_q3_state_not_transitioned
- Given: Job execution completes
- When: Checking job.state after execute_job
- Then: State is Completed or Failed (not Pending)

### test_violates_i1_exceeds_concurrency
- Given: concurrency_limit = 1
- When: 5 execute_job calls
- Then: Maximum 1 job running at any time

### test_violates_i2_duplicate_execution
- Given: Job already running
- When: Second execute_job call same job
- Then: Err(Error::InvalidJobState)

### test_violates_i3_priority_corruption
- Given: Sequential polls
- When: Observing returned priority order
- Then: Always descending by priority value

## Given-When-Then Scenarios

### Scenario 1: Continuous polling with jobs arriving
Given: JobProcessor is running with 100ms poll interval
And: No jobs exist initially
When: A new P1 job is added to the queue
Then: The job is polled and executed within 200ms

### Scenario 2: Priority preemption
Given: A P2 job is running
And: A P0 job arrives in the queue
When: The next poll occurs
Then: P0 job is queued and will execute after/in parallel with P2 (based on concurrency settings)

### Scenario 3: Graceful shutdown with in-flight jobs
Given: JobProcessor is running with 2 in-flight jobs
And: Shutdown signal is received
When: shutdown is called
Then: Completes both in-flight jobs
And: Transitions processor to stopped state
And: Returns Ok(())

### Scenario 4: Repository failure during execution
Given: A job is executing
And: Repository becomes unavailable
When: Job execution completes
Then: Job result is cached locally
And: Retry mechanism attempts repository reconnection
And: On reconnection, result is persisted
