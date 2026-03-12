# Martin Fowler Test Plan

## Feature: orchestrator: Add timeout and retry policies

This test plan provides comprehensive Given-When-Then scenarios for phase-level timeouts, 
deadline handling, exponential backoff, and circuit breakers.

---

## Happy Path Tests

### test_phase_completes_within_timeout
Given: A phase that takes 50ms to execute
When: Running with a 100ms timeout
Then: Phase completes successfully with `Ok(PhaseResult { success: true })`

### test_retry_succeeds_on_eventual_success
Given: A flaky operation that fails twice then succeeds
When: Running with retry policy (max_retries=3, base_delay=10ms)
Then: Phase succeeds after 2 retries with `Ok(PhaseResult { success: true })`

### test_circuit_breaker_allows_normal_execution
Given: Circuit breaker in Closed state with failure_threshold=3
When: Running 3 successful phase executions
Then: Circuit breaker remains in Closed state, all calls succeed

### test_exponential_backoff_applies_correct_delay
Given: Retry policy with base_delay_ms=100, max_delay_ms=1000
When: Calculating delays for attempts 0, 1, 2, 3
Then: Delays are [100, 200, 400, 800] milliseconds respectively

---

## Error Path Tests

### test_phase_timeout_returns_error
Given: A phase that takes 200ms to execute
When: Running with a 50ms timeout
Then: Returns `Err(OrchestratorError::PhaseTimeout { phase, timeout_ms: 50, elapsed_ms: 200 })`

### test_retries_exhausted_returns_error
Given: A phase that always fails
When: Running with retry policy (max_retries=2)
Then: Returns `Err(OrchestratorError::RetriesExhausted { attempts: 3, last_error: ... })` after 3 attempts (1 initial + 2 retries)

### test_circuit_breaker_open_rejects_calls
Given: Circuit breaker in Open state (after 3 failures)
When: Attempting to execute a phase
Then: Returns `Err(OrchestratorError::CircuitBreakerOpen { failure_count: 3, ... })`

### test_circuit_breaker_transitions_to_half_open_after_timeout
Given: Circuit breaker in Open state, last_failure was 6 seconds ago
When: Recovery timeout is 5 seconds
Then: Circuit breaker state is `HalfOpen`, `can_execute()` returns `true`

---

## Edge Case Tests

### test_zero_timeout_immediately_fails
Given: Timeout configured with duration_ms=0
When: Creating PhaseTimeout
Then: Returns `Err(ConfigError::InvalidTimeout { duration_ms: 0 })`

### test_zero_retries_no_retry_attempts
Given: Retry policy with max_retries=0
When: Running a failing phase
Then: Returns single error without retry, `Err(OrchestratorError::PhaseExecution { ... })`

### test_circuit_breaker_recovery_resets_failure_count
Given: Circuit breaker with failure_count=2 in Open state
When: After transitioning to HalfOpen and recording success
Then: Circuit breaker returns to Closed state with failure_count=0

### test_max_delay_cap_applies
Given: Retry policy with base_delay_ms=100, max_delay_ms=500
When: Calculating delay for attempt 10
Then: Returns capped delay of 500ms (not 102400ms)

### test_pipeline_state_persists_after_timeout_failure
Given: Pipeline in AgentDevelopment state
When: Phase times out and returns error
Then: Pipeline state is persisted with last_error containing timeout message

---

## Contract Verification Tests

### test_precondition_timeout_duration_positive
Given: PhaseTimeout::new(0)
When: Constructor is called
Then: Returns `Err(ConfigError::InvalidTimeout { duration_ms: 0 })`

### test_precondition_base_delay_positive
Given: RetryPolicy::new(3, 0, 1000)
When: Constructor is called
Then: Returns `Err(ConfigError::InvalidBaseDelay { delay_ms: 0 })`

### test_precondition_max_delay_gte_base
Given: RetryPolicy::new(3, 100, 50)
When: Constructor is called
Then: Returns `Err(ConfigError::InvalidMaxDelay { max_delay_ms: 50, base_delay_ms: 100 })`

### test_postcondition_timeout_elapsed_returns_error
Given: PhaseTimeout with duration_ms=1
When: Checking is_expired after 100ms
Then: Returns `true`

### test_postcondition_circuit_breaker_opens_after_threshold
Given: CircuitBreaker with failure_threshold=3
When: Recording 3 failures
Then: Circuit breaker state is `Open`

### test_invariant_pipeline_state_persisted
Given: PipelineExecutor with configured StateStore
When: Running a phase that returns error
Then: Pipeline state is updated in store

---

## Contract Violation Tests

### test_violation_p1_zero_timeout
```rust
PhaseTimeout::new(0) 
// Should produce: Err(ConfigError::InvalidTimeout { duration_ms: 0 })
```

### test_violation_p3_zero_base_delay
```rust
RetryPolicy::new(3, 0, 1000) 
// Should produce: Err(ConfigError::InvalidBaseDelay { delay_ms: 0 })
```

### test_violation_p4_max_less_than_base
```rust
RetryPolicy::new(3, 100, 50) 
// Should produce: Err(ConfigError::InvalidMaxDelay { max_delay_ms: 50, base_delay_ms: 100 })
```

### test_violation_p5_zero_failure_threshold
```rust
CircuitBreaker::new(0, 1000) 
// Should produce: Err(ConfigError::InvalidFailureThreshold { threshold: 0 })
```

### test_violation_q1_timeout_elapsed
```rust
let timeout = PhaseTimeout::new(50);
let started = Utc::now() - Duration::milliseconds(100);
timeout.is_expired(started) 
// Should produce: true
```

### test_violation_q3_circuit_opens
```rust
let mut cb = CircuitBreaker::new(3, 5000);
cb.record_failure();
cb.record_failure();
cb.record_failure();
// Should produce: cb.state == CircuitBreakerState::Open
```

### test_violation_q5_circuit_open_rejects
```rust
let cb = CircuitBreaker::new(3, 5000); // Assume already opened
cb.can_execute() 
// Should produce: false
```

### test_violation_p6_zero_recovery_timeout
```rust
CircuitBreaker::new(3, 0) 
// Should produce: Err(ConfigError::InvalidRecoveryTimeout { timeout_ms: 0 })
```

### test_violation_q2_retries_exhausted_returns_error
```rust
let policy = RetryPolicy::new(2, 10, 1000);
// Run 3 failing attempts (1 initial + 2 retries)
// Should produce: Err(OrchestratorError::RetriesExhausted { attempts: 3, ... })
```

### test_violation_q4_half_open_after_recovery_timeout
```rust
let mut cb = CircuitBreaker::new(2, 100); // 100ms recovery timeout
cb.record_failure();
cb.record_failure(); // Now Open
// Wait 150ms
// Should produce: cb.state == CircuitBreakerState::HalfOpen
```

---

## Additional Edge Case Tests

### test_circuit_breaker_record_success_clears_failures
Given: CircuitBreaker with failure_count=2
When: Recording a success
Then: failure_count resets to 0, state remains Closed

### test_circuit_breaker_half_open_allows_execution
Given: CircuitBreaker in HalfOpen state
When: Calling can_execute()
Then: Returns true, allowing the test request through

### test_deadline_exceeded_returns_error
Given: Deadline that expired 1 second ago
When: Checking is_exceeded()
Then: Returns true

### test_deadline_remaining_time_calculates_correctly
Given: Deadline set for 5 seconds from now
When: Checking remaining_ms()
Then: Returns approximately 5000 (within 100ms margin)

---

## Given-When-Then Scenarios

### Scenario 1: Phase completes successfully within timeout
**Given**: PipelineExecutor configured with 500ms timeout for spec_review phase
**When**: Running spec_review phase which takes 200ms
**Then**:
- Returns `Ok(PhaseResult { success: true, ... })`
- Pipeline state transitions to UniverseSetup

### Scenario 2: Phase times out, retries exhausted
**Given**: PipelineExecutor with 100ms timeout, 2 max retries, 10ms base delay
**When**: Running a phase that takes 150ms (always times out)
**Then**:
- First attempt: Err(PhaseTimeout)
- Second attempt: Err(PhaseTimeout)
- Third attempt: Err(PhaseTimeout)
- Returns `Err(OrchestratorError::RetriesExhausted { attempts: 3, ... })`

### Scenario 3: Circuit breaker prevents cascading failures
**Given**: Circuit breaker with failure_threshold=2, recovery_timeout=5000ms
**When**: Recording 2 failures then attempting execution
**Then**:
- State transitions to Open after second failure
- Next call returns `Err(OrchestratorError::CircuitBreakerOpen)`
- After 5 seconds, state transitions to HalfOpen
- Next call succeeds, state returns to Closed

### Scenario 4: Deadline handling with global pipeline timeout
**Given**: Pipeline with global deadline of 10 seconds, 3 phases each taking 4 seconds
**When**: Running phases sequentially
**Then**:
- First 2 phases complete
- Third phase fails with `OrchestratorError::DeadlineExceeded`
- Pipeline marked as Failed

### Scenario 5: Exponential backoff prevents thundering herd
**Given**: Retry policy with base_delay=100ms, max_retries=4
**When**: Running flaky operation that fails 3 times then succeeds
**Then**:
- Attempt 1: immediate failure
- Wait 100ms: Attempt 2
- Wait 200ms: Attempt 3
- Wait 400ms: Attempt 4
- Total wait time: 700ms
- Final result: success
