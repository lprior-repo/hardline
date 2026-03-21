# Martin Fowler Test Plan: scpm-7vg - Queue Domain Models

## Happy Path Tests

### test_creates_job_with_valid_inputs
Given: Valid JobId "j-1", valid JSON payload "{\"key\":\"value\"}", valid priority 100
When: `Job::new(id, payload, priority)` is called
Then: Returns `Ok(Job)` with id "j-1", status `Pending`, payload contains valid JSON

### test_job_status_defaults_to_pending
Given: A newly created Job
When: `job.status()` is called
Then: Returns `JobStatus::Pending`

### test_job_transitions_from_pending_to_processing
Given: A Job with status `Pending`
When: `job.transition_to(Processing)` is called
Then: Returns `Ok(Job)` with status `Processing`

### test_job_transitions_from_processing_to_completed
Given: A Job with status `Processing`
When: `job.transition_to(Completed)` is called
Then: Returns `Ok(Job)` with status `Completed`

### test_job_transitions_from_processing_to_failed
Given: A Job with status `Processing`
When: `job.transition_to(Failed)` is called
Then: Returns `Ok(Job)` with status `Failed`

### test_queue_enqueue_adds_job_at_correct_priority
Given: An empty Queue, Job with priority 100
When: `queue.enqueue(job)` is called
Then: Returns new Queue where job is present at priority position

### test_queue_dequeue_returns_lowest_priority_job
Given: A Queue with jobs of priorities [50, 100, 150]
When: `queue.dequeue()` is called
Then: Returns `(Queue, Some(Job))` where job has priority 50

### test_queue_dequeue_on_empty_queue
Given: An empty Queue
When: `queue.dequeue()` is called
Then: Returns `(Queue, None)`

## Error Path Tests

### test_returns_invalid_payload_error_when_payload_empty
Given: Empty string payload
When: `Job::new(JobId::new("j-1"), Payload::from(""), 100)` is called
Then: Returns `Err(JobCreationError::InvalidPayload(PayloadError::Empty))`

### test_returns_invalid_payload_error_when_payload_malformed_json
Given: Malformed JSON string "{{not json"
When: `Job::new(JobId::new("j-1"), Payload::from("{{not json"), 100)` is called
Then: Returns `Err(JobCreationError::InvalidPayload(PayloadError::MalformedJson))`

### test_returns_invalid_priority_error_when_priority_overflows
Given: Priority value 256
When: `Job::new(JobId::new("j-1"), Payload::from("{}"), 256)` is called
Then: Returns `Err(JobCreationError::InvalidPriority(256))`

### test_returns_invalid_priority_error_when_priority_negative
Given: Priority value 0 (valid), but test with 256+ handled above
Note: u8 is 0-255, so negative not possible in type system

### test_returns_invalid_transition_error_for_same_status
Given: A Job with status `Pending`
When: `job.transition_to(Pending)` is called
Then: Returns `Err(QueueError::InvalidTransition { from: Pending, to: Pending })`

### test_returns_invalid_transition_error_from_completed
Given: A Job with status `Completed`
When: `job.transition_to(Processing)` is called
Then: Returns `Err(QueueError::InvalidTransition { from: Completed, to: Processing })`

### test_returns_invalid_transition_error_from_failed
Given: A Job with status `Failed`
When: `job.transition_to(Processing)` is called
Then: Returns `Err(QueueError::InvalidTransition { from: Failed, to: Processing })`

## Edge Case Tests

### test_handles_empty_payload_string
Given: Payload is an empty string ""
When: `Job::new()` is called with this payload
Then: Returns appropriate error

### test_handles_whitespace_only_payload
Given: Payload is only whitespace "   "
When: `Job::new()` is called
Then: Should this be valid or invalid? (Decision: Invalid - whitespace-only is not valid JSON)

### test_handles_max_priority_value
Given: Priority value 255
When: `Job::new(JobId::new("j-1"), Payload::from("{}"), 255)` is called
Then: Returns `Ok(Job)` with priority 255

### test_handles_min_priority_value
Given: Priority value 0
When: `Job::new(JobId::new("j-1"), Payload::from("{}"), 0)` is called
Then: Returns `Ok(Job)` with priority 0

### test_handles_complex_nested_json_payload
Given: Complex nested JSON payload
When: `Job::new(JobId::new("j-1"), Payload::from("{\"nested\":{\"deep\":{\"value\":123}}}"), 50)` is called
Then: Returns `Ok(Job)` with payload containing nested structure

### test_queue_maintains_priority_order_after_multiple_enqueues
Given: A Queue and jobs with priorities [200, 50, 150, 25]
When: All jobs are enqueued
Then: Dequeuing returns jobs in order [25, 50, 150, 200]

## Contract Verification Tests

### test_precondition_payload_non_empty
Given: Empty payload
When: Constructor is invoked
Then: Returns Error (not a panic)

### test_precondition_payload_valid_json
Given: Invalid JSON payload
When: Constructor is invoked
Then: Returns Error (not a panic)

### test_precondition_priority_valid_range
Given: Priority outside 0-255
When: Constructor is invoked
Then: Returns Error (not a panic)

### test_postcondition_job_status_valid_after_creation
Given: A successfully created Job
When: `job.status()` is called
Then: Returns `JobStatus::Pending`

### test_invariant_queue_jobs_sorted_by_priority
Given: A Queue with multiple jobs
When: Jobs are dequeued
Then: Jobs are returned in ascending priority order

### test_invariant_unique_job_ids_in_queue
Given: A Queue
When: Multiple jobs are enqueued
Then: All job IDs remain unique

## Given-When-Then Scenarios

### Scenario 1: Successful Job Lifecycle
Given: A newly created Job with valid payload
When: Job transitions through Pending -> Processing -> Completed
Then: Each transition succeeds and status is correctly updated

### Scenario 2: Job Fails During Processing
Given: A Job in Processing state
When: Processing encounters an error and Job transitions to Failed
Then: Job status is Failed and can no longer transition to other states

### Scenario 3: Reject Malformed Job at Creation
Given: An invalid payload (not JSON)
When: Attempting to create a Job with this payload
Then: Job creation fails with InvalidPayload error, no Job instance exists

### Scenario 4: Queue Orders by Priority
Given: A Queue with multiple jobs at different priorities
When: Jobs are dequeued
Then: Jobs are returned in ascending priority order (lowest number = highest priority)

### Scenario 5: Cannot Re-transition Completed Job
Given: A Job that has reached Completed state
When: Attempting to transition back to Processing
Then: Returns InvalidTransition error

### Scenario 6: Cannot Re-transition Failed Job
Given: A Job that has reached Failed state
When: Attempting to transition to any other state
Then: Returns InvalidTransition error
