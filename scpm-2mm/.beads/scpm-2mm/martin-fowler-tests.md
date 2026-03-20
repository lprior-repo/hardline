# Martin Fowler Test Plan

## Test Suite: Orchestrator Timeouts and Retry Policies

**Feature**: Phase-level timeout configurations, retry with exponential backoff, circuit breaker state machine
**Bead ID**: scpm-2mm

---

## Happy Path Tests

### test_timeout_policy_creation_with_valid_value
**Given**: A valid timeout value of 5000 milliseconds
**When**: `new_timeout_policy(5000)` is called
**Then**: Returns `Ok(TimeoutPolicy { timeout_ms: Some(5000) })`

### test_retry_policy_creation_with_valid_parameters
**Given**: max_retries=3, base_delay=100, factor=2.0, max_delay=Some(1000)
**When**: `new_retry_policy(3, 100, 2.0, Some(1000), vec!["io".into()])` is called
**Then**: Returns `Ok(RetryPolicy { ... })` with all values correctly stored

### test_circuit_breaker_starts_in_closed_state
**Given**: A newly created CircuitBreaker with default parameters
**When**: CircuitBreaker is instantiated
**Then**: State is `CircuitState::Closed` and `is_execution_allowed()` returns `true`

### test_phase_executes_successfully_within_timeout
**Given**: A phase that completes in 100ms, timeout of 5000ms, no retry policy
**When**: `execute_phase("test-phase", async { Ok(PhaseResult) }, Some(timeout), None, None)` is called
**Then**: Returns `Ok(PhaseResult)`

### test_phase_succeeds_on_first_attempt_no_retry_needed
**Given**: A phase that succeeds immediately, retry policy with max_retries=3
**When**: `execute_phase` is called
**Then**: Phase is executed exactly once, no retries occur

### test_exponential_backoff_delay_increases_monotonically
**Given**: Retry policy with base_delay=100, factor=2.0, max_delay=None
**When**: `compute_backoff_delay` is called for attempts 0, 1, 2, 3
**Then**: Returns [100, 200, 400, 800] respectively (each delay > previous)

### test_exponential_backoff_formula_verification
**Given**: Retry policy with base_delay=100, factor=2.0, max_delay=None
**When**: `compute_backoff_delay` is called for attempts 0, 1, 2, 3
**Then**: Each delay matches the formula `delay = base_delay * factor^attempt` exactly:
  - Attempt 0: 100 * 2^0 = 100ms
  - Attempt 1: 100 * 2^1 = 200ms
  - Attempt 2: 100 * 2^2 = 400ms
  - Attempt 3: 100 * 2^3 = 800ms
**Verification**: This explicitly verifies the stated formula, not just monotonicity

### test_backoff_delay_capped_at_max_delay
**Given**: Retry policy with base_delay=100, factor=2.0, max_delay=Some(500)
**When**: `compute_backoff_delay(10, policy)` is called (would be 102400 without cap)
**Then**: Returns 500 (capped at max_delay)

### test_circuit_breaker_allows_execution_in_closed_state
**Given**: CircuitBreaker in Closed state
**When**: `is_execution_allowed(&cb)` is called
**Then**: Returns `true`

### test_circuit_breaker_transitions_to_halfopen_after_open_duration
**Given**: CircuitBreaker in Open state, open_duration=30s, current time > open_until
**When**: `check_and_transition(&mut cb)` is called
**Then**: State transitions to HalfOpen

---

## Error Path Tests

### test_timeout_error_when_phase_exceeds_timeout
**Given**: A phase that takes 200ms to execute, timeout of 100ms
**When**: `execute_phase` is called
**Then**: Returns `Err(Error::TimeoutExceeded { phase_id: "test", duration_ms: 200, timeout_ms: 100 })`

### test_invalid_timeout_zero_returns_error
**Given**: A timeout value of 0 milliseconds
**When**: `new_timeout_policy(0)` is called
**Then**: Returns `Err(Error::InvalidTimeout("timeout must be greater than 0ms".into()))`

### test_invalid_timeout_negative_returns_error
**Given**: A timeout value that would cause negative (e.g., passed as checked subtraction result)
**When**: `new_timeout_policy(0)` is called (validated before call)
**Then**: Returns `Err(Error::InvalidTimeout(...))` (no panic)

### test_invalid_retry_max_delay_zero_returns_error
**Given**: max_delay of 0 milliseconds
**When**: `new_retry_policy(3, 100, 2.0, Some(0), vec![])` is called
**Then**: Returns `Err(Error::InvalidRetryPolicy("max_delay must be greater than 0ms".into()))`

### test_max_retries_exceeded_returns_last_error
**Given**: A phase that always fails, retry policy with max_retries=2
**When**: `execute_phase` is called
**Then**: Returns `Err(Error::MaxRetriesExceeded { phase_id: "test", attempts: 3, last_error: <final_error> })`

### test_circuit_breaker_blocks_execution_when_open
**Given**: CircuitBreaker in Open state
**When**: `execute_phase` is called with the circuit breaker
**Then**: Returns `Err(Error::CircuitBreakerOpen { phase_id: "test", open_until: <instant> })` without executing phase

### test_non_retryable_error_not_retried
**Given**: A phase that returns a non-retryable error, retry policy with max_retries=3
**When**: `execute_phase` is called
**Then**: Phase is attempted exactly once, returns the non-retryable error

### test_invalid_circuit_breaker_failure_threshold_zero
**Given**: A failure_threshold of 0
**When**: `new_circuit_breaker(0, 3, Duration::from_secs(30))` is called
**Then**: Returns `Err(Error::InvalidRetryPolicy("failure_threshold must be positive".into()))`

---

## Edge Case Tests

### test_timeout_policy_none_means_no_timeout
**Given**: TimeoutPolicy with `timeout_ms: None`
**When**: `get_timeout_ms(&policy)` is called
**Then**: Returns `None` (no timeout enforcement)

### test_zero_max_retries_means_no_retries
**Given**: Retry policy with max_retries=0
**When**: Phase fails once
**Then**: Returns `Err(Error::MaxRetriesExceeded { attempts: 1, max_retries: 0, ... })` immediately

### test_circuit_breaker_halfopen_allows_one_execution
**Given**: CircuitBreaker in HalfOpen state
**When**: `is_execution_allowed(&cb)` is called
**Then**: Returns `true` (allows probe execution)

### test_backoff_delay_first_attempt_uses_base_delay
**Given**: Retry policy with base_delay=500
**When**: `compute_backoff_delay(0, policy)` is called
**Then**: Returns 500 (first attempt uses base_delay, not base * factor^0)

### test_multiple_successful_executions_decrease_failure_count
**Given**: CircuitBreaker in Closed state with failure_count=2
**When**: `record_success()` is called twice
**Then**: failure_count becomes 0

### test_exponential_backoff_factor_of_1_returns_constant_delay
**Given**: Retry policy with factor=1.0 (constant backoff)
**When**: `compute_backoff_delay` is called for multiple attempts
**Then**: Returns same delay as base_delay for all attempts

### test_empty_retryable_errors_list_means_no_errors_retryable
**Given**: Retry policy with empty retryable_errors
**When**: Any error occurs
**Then**: Error is treated as non-retryable

### test_very_large_timeout_value_handled
**Given**: Timeout of u64::MAX milliseconds
**When**: `new_timeout_policy(u64::MAX)` is called
**Then**: Returns `Ok(TimeoutPolicy { timeout_ms: Some(u64::MAX) })`

---

## Contract Verification Tests

### test_precondition_p1_timeout_greater_than_zero
**Given**: Any timeout value passed to `new_timeout_policy`
**When**: Value is <= 0
**Then**: Returns `Err(Error::InvalidTimeout(...))` -- compile-time enforcement via NonZeroU64

### test_precondition_p2_max_retries_non_negative
**Given**: Any u32 value for max_retries
**When**: Value is >= 0 (always true for u32)
**Then**: Accepted without error -- u32 type enforces at compile-time

### test_precondition_p3_max_delay_greater_than_zero_if_configured
**Given**: A retry policy with max_delay=Some(0)
**When**: `new_retry_policy` is called
**Then**: Returns `Err(Error::InvalidRetryPolicy(...))` -- enforced via Option<NonZeroU64>

### test_precondition_p4_failure_threshold_positive
**Given**: A circuit breaker with failure_threshold=0
**When**: `new_circuit_breaker(0, ...)` is called
**Then**: Returns `Err(Error::InvalidRetryPolicy(...))` -- enforced via NonZeroU32

### test_postcondition_q1_phase_execution_respects_timeout
**Given**: A phase that runs longer than configured timeout
**When**: `execute_phase` is called
**Then**: Returns `Err(Error::TimeoutExceeded { ... })` within timeout + overhead margin

### test_postcondition_q2_max_retries_exceeded_returns_last_error
**Given**: A phase that fails, max_retries=2
**When**: All 3 attempts fail
**Then**: Final error is `Error::MaxRetriesExceeded` containing the last encountered error

### test_postcondition_q4_backoff_delay_monotonically_increases
**Given**: Retry policy with valid base and factor
**When**: Delays are computed for consecutive attempts
**Then**: Each delay is > previous delay (until max_delay cap)

### test_postcondition_q5_circuit_breaker_open_prevents_execution
**Given**: CircuitBreaker in Open state
**When**: `execute_phase` is called
**Then**: Phase is not executed, returns `Err(Error::CircuitBreakerOpen { ... })`

### test_invariant_i1_circuit_state_reflects_failure_rate
**Given**: CircuitBreaker with failure_threshold=3
**When**: `record_failure()` is called 3 times
**Then**: State becomes `CircuitState::Open`

### test_invariant_i2_backoff_never_exceeds_max_delay
**Given**: Retry policy with max_delay=Some(1000)
**When**: `compute_backoff_delay` is called for any attempt
**Then**: Result is always <= 1000

### test_invariant_i3_open_to_halfopen_requires_open_duration
**Given**: CircuitBreaker just entered Open state at T=0, open_duration=30s
**When**: `check_and_transition` called at T=20s
**Then**: State remains Open (duration not elapsed)

### test_invariant_i4_non_retryable_error_not_retried
**Given**: Error that is not in retryable_errors list
**When**: Phase returns that error
**Then**: Returns immediately without retry attempts

---

## Contract Violation Tests

### test_violates_p1_zero_timeout_returns_invalid_timeout_error
**Given**: timeout_ms = 0
**When**: `new_timeout_policy(0)` is called
**Then**: Returns `Err(Error::InvalidTimeout("timeout must be greater than 0ms".into()))`
**Verification**: This is NOT a panic -- returns proper error variant

### test_violates_p3_zero_max_delay_returns_invalid_retry_policy_error
**Given**: max_delay_ms = Some(0)
**When**: `new_retry_policy(3, 100, 2.0, Some(0), vec![])` is called
**Then**: Returns `Err(Error::InvalidRetryPolicy("max_delay must be greater than 0ms".into()))`
**Verification**: Returns error variant, not panic

### test_violates_p4_zero_failure_threshold_returns_error
**Given**: failure_threshold = 0
**When**: `new_circuit_breaker(0, 3, Duration::from_secs(30))` is called
**Then**: Returns `Err(Error::InvalidRetryPolicy("failure_threshold must be positive".into()))`
**Verification**: NonZeroU32 type would prevent compilation for literal 0

### test_violates_q1_phase_exceeds_timeout_returns_timeout_error
**Given**: phase takes 200ms, timeout is 100ms
**When**: `execute_phase` is called
**Then**: Returns `Err(Error::TimeoutExceeded { phase_id: "test", duration_ms: 200, timeout_ms: 100 })`
**Verification**: Error contains actual duration and configured timeout

### test_violates_q2_max_retries_exceeded_contains_last_error
**Given**: Phase fails with "connection refused", max_retries=1
**When**: Both attempts fail
**Then**: Returns `Err(Error::MaxRetriesExceeded { attempts: 2, last_error: "connection refused" })`
**Verification**: Error contains the last error, not the first

### test_violates_q4_backoff_not_monotonic_is_bug
**Given**: Retry policy with base=100, factor=2.0
**When**: `compute_backoff_delay(1)` returns <= `compute_backoff_delay(0)`
**Then**: This indicates implementation bug -- delays must increase

### test_violates_i1_circuit_state_wrong_after_failures
**Given**: CircuitBreaker with failure_threshold=2
**When**: `record_failure()` called once
**Then**: State remains Closed (not Open yet)

### test_violates_i2_backoff_exceeds_max_delay_is_bug
**Given**: max_delay=500ms
**When**: `compute_backoff_delay(10)` returns > 500
**Then**: This indicates implementation bug -- delay must be capped

### test_violates_i4_non_retryable_error_retried_is_bug
**Given**: Error "validation_failed" not in retryable_errors
**When**: Phase returns this error with max_retries=3
**Then**: Phase is attempted exactly once, not retried
**Verification**: If retried, implementation violates invariant I4

---

## Given-When-Then Scenarios

### Scenario 1: Phase completes successfully within timeout
**Given**: A TimeoutPolicy with 5000ms timeout
**And**: A phase that returns `Ok(PhaseResult)` in 100ms
**When**: `execute_phase` is called
**Then**: Returns `Ok(PhaseResult)` immediately
**And**: No retry is attempted
**And**: Circuit breaker state remains unchanged

### Scenario 2: Phase times out
**Given**: A TimeoutPolicy with 50ms timeout
**And**: A phase that takes 100ms to complete
**When**: `execute_phase` is called
**Then**: Returns `Err(Error::TimeoutExceeded { ... })`
**And**: No retry is attempted
**And**: Error contains phase_id, actual duration, and configured timeout

### Scenario 3: Phase fails and is retried with exponential backoff
**Given**: A RetryPolicy with max_retries=3, base_delay=100ms, factor=2.0
**And**: A phase that fails twice then succeeds
**When**: `execute_phase` is called
**Then**: Phase is attempted up to 4 times (1 initial + 3 retries)
**And**: Delay before retry 1 is ~100ms
**And**: Delay before retry 2 is ~200ms
**And**: Delay before retry 3 is ~400ms
**And**: Final result is `Ok(PhaseResult)` if phase eventually succeeds

### Scenario 4: Phase fails after max retries exhausted
**Given**: A RetryPolicy with max_retries=2, base_delay=100ms, factor=2.0
**And**: A phase that always returns `Err(Error::IoError("connection refused"))`
**When**: `execute_phase` is called
**Then**: Phase is attempted 3 times (1 initial + 2 retries)
**And**: Returns `Err(Error::MaxRetriesExceeded { attempts: 3, last_error: Error::IoError(...) })`

### Scenario 5: Circuit breaker opens after failure threshold
**Given**: A CircuitBreaker with failure_threshold=3, success_threshold=2
**And**: CircuitBreaker is in Closed state
**When**: `record_failure()` is called 3 times
**Then**: State transitions to Open
**And**: `is_execution_allowed()` returns `false`

### Scenario 6: Circuit breaker prevents execution when open
**Given**: CircuitBreaker in Open state with open_until in the future
**When**: `execute_phase` is called
**Then**: Phase is NOT executed
**And**: Returns `Err(Error::CircuitBreakerOpen { phase_id: "test", open_until: <future_instant> })`

### Scenario 7: Circuit breaker half-open allows probe
**Given**: CircuitBreaker in Open state
**And**: open_duration has elapsed
**When**: `check_and_transition()` is called
**Then**: State becomes HalfOpen
**And**: `is_execution_allowed()` returns `true`

### Scenario 8: Circuit breaker closes after success threshold in half-open
**Given**: CircuitBreaker in HalfOpen state with success_threshold=2
**When**: `record_success()` is called 2 times
**Then**: State transitions to Closed
**And**: failure_count resets to 0

### Scenario 8b: Circuit breaker returns to open from half-open after probe failure
**Given**: CircuitBreaker in HalfOpen state with failure_threshold=3
**And**: A probe execution fails
**When**: `record_failure()` is called while in HalfOpen state
**Then**: State transitions back to Open
**And**: failure_count resets to 0
**And**: open_duration timer restarts
**Note**: This completes the HalfOpen→Open→HalfOpen cycle

### Scenario 9: Non-retryable error is not retried
**Given**: RetryPolicy with max_retries=3, retryable_errors=["io", "network"]
**And**: A phase that returns `Err(Error::ValidationFailed("invalid input"))`
**When**: `execute_phase` is called
**Then**: Phase is attempted exactly once
**And**: Returns `Err(Error::NonRetryableError { cause: "validation_failed", ... })`

### Scenario 10: Total timeout spans all retry attempts
**Given**: TimeoutPolicy with 200ms, RetryPolicy with max_retries=3
**And**: A phase that takes 50ms per attempt and fails 3 times
**And**: Total time including backoff delays is 50 + 100 + 200 + 300 = 650ms
**When**: `execute_phase` is called
**Then**: The total accumulated time (execution + backoff delays) MUST NOT exceed the timeout
**And**: If total time exceeds timeout, returns TimeoutError
**And**: Retries are aborted when timeout fires
**Note**: Contract Q1 specifies "total time MUST NOT exceed configured timeout" — this is a TOTAL timeout, not per-attempt

### Scenario 10b: Timeout fires during backoff delay
**Given**: TimeoutPolicy with 150ms, RetryPolicy with max_retries=3, base_delay=100ms, factor=2.0
**And**: A phase that fails on first attempt (50ms execution)
**When**: Total elapsed time (50ms execution + 100ms backoff = 150ms) equals timeout
**Then**: Timeout fires immediately upon backoff completion, before retry attempt 2
**And**: Returns TimeoutError without attempting second retry
**Note**: This verifies Q1: total time includes backoff delays in the timeout calculation

---

## End-to-End Integration Test

### test_end_to_end_phase_with_full_timeout_retry_circuit_breaker
**Given**:
- TimeoutPolicy: 1000ms
- RetryPolicy: max_retries=2, base_delay=50ms, factor=2.0, max_delay=200ms, retryable_errors=["io", "network"]
- CircuitBreaker: failure_threshold=3, success_threshold=2, open_duration=5s
- A phase that fails twice with "io" error then succeeds

**When**: `execute_phase("integration-test", phase_fn, Some(timeout), Some(retry), Some(&mut cb))` is called

**Then**:
1. Attempt 1: Phase fails with "io" -> record_failure, cb.state = Closed
2. Delay ~50ms (base_delay)
3. Attempt 2: Phase fails with "io" -> record_failure, cb.state = Closed
4. Delay ~100ms (50 * 2^1)
5. Attempt 3: Phase succeeds -> record_success, cb.state = Closed
6. Returns `Ok(PhaseResult)`

**Verification**:
- Total time < 1000ms (no timeout)
- Exactly 3 attempts made
- Backoff delays: 50ms, 100ms
- Circuit breaker remained Closed (successes exceeded failures)
