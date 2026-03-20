# Contract Specification

## Context
- **Feature**: Orchestrator timeouts and retry policies
- **Bead ID**: scpm-2mm
- **Domain Terms**:
  - Phase: A discrete unit of execution within a workflow
  - TimeoutPolicy: Configuration for phase execution time limits
  - RetryPolicy: Configuration for retry behavior with exponential backoff
  - CircuitBreaker: Failure mitigation state machine (Closed, HalfOpen, Open)
  - TimeoutError: Error variant for exceeded timeouts
- **Assumptions**:
  - Timeout durations are specified in milliseconds
  - Exponential backoff follows the formula: `delay = base_delay * factor^attempt`
  - Circuit breaker failure rate thresholds are configurable
- **Open Questions**: None

---

## Preconditions
1. **P1**: A phase timeout value, if configured, MUST be strictly greater than 0 milliseconds.
2. **P2**: A retry policy `max_retries` value MUST be a non-negative integer (>= 0).
3. **P3**: A retry policy `max_delay` value, if configured, MUST be strictly greater than 0 milliseconds.
4. **P4**: A circuit breaker `failure_threshold` for transitioning Open->HalfOpen MUST be a positive integer.
5. **P5**: A retry policy `factor` value MUST be strictly greater than 1.0 (exponential backoff multiplier).

---

## Postconditions
1. **Q1**: A phase execution total time MUST NOT exceed its configured timeout plus scheduling overhead.
2. **Q2**: If a phase exceeds its `max_retries` limit, the final result MUST be the last encountered error.
3. **Q3**: A successfully completed phase MUST NOT be retried.
4. **Q4**: Backoff delays MUST monotonically increase according to the exponential factor up to the configured `max_delay`.
5. **Q5**: When circuit breaker is Open, phase execution MUST NOT be attempted.

---

## Invariants
1. **I1**: The circuit breaker state (Closed, HalfOpen, Open) MUST accurately reflect the failure rate.
2. **I2**: Backoff delays MUST NOT exceed the configured `max_delay`.
3. **I3**: The circuit breaker MUST transition from Open to HalfOpen only after the configured `open_duration` has elapsed.
4. **I4**: A phase that returns a non-retryable error MUST NOT be retried regardless of retry policy.

---

## Error Taxonomy
- `Error::InvalidTimeout(String)` - when timeout value is invalid (<=0ms)
- `Error::InvalidRetryPolicy(String)` - when retry policy parameters are invalid
- `Error::TimeoutExceeded { phase_id: String, duration_ms: u64, timeout_ms: u64 }` - when phase exceeds its timeout
- `Error::MaxRetriesExceeded { phase_id: String, attempts: u32, last_error: Box<Error> }` - when max retries exhausted
- `Error::CircuitBreakerOpen { phase_id: String, open_until: Instant }` - when circuit breaker prevents execution
- `Error::NonRetryableError { phase_id: String, cause: String }` - when error cannot be retried
- `Error::PreconditionViolation(String)` - when a precondition is violated

---

## Contract Signatures

### TimeoutPolicy
```rust
/// Configuration for phase timeout
struct TimeoutPolicy {
    timeout_ms: Option<NonZeroU64>,  // None = no timeout
}

/// Creates a TimeoutPolicy if the value is valid (> 0ms)
fn new_timeout_policy(timeout_ms: u64) -> Result<TimeoutPolicy, Error::InvalidTimeout>;

/// Returns the effective timeout in milliseconds
fn get_timeout_ms(policy: &TimeoutPolicy) -> Option<u64>;
```

### RetryPolicy
```rust
/// Configuration for retry behavior with exponential backoff
struct RetryPolicy {
    max_retries: u32,           // Non-negative
    base_delay_ms: u64,         // > 0
    factor: f64,               // > 1.0 (exponential multiplier)
    max_delay_ms: Option<u64>,  // None = no cap
    retryable_errors: Vec<String>, // Error types that can be retried
}

/// Creates a RetryPolicy if all values are valid
fn new_retry_policy(
    max_retries: u32,
    base_delay_ms: u64,
    factor: f64,
    max_delay_ms: Option<u64>,
    retryable_errors: Vec<String>,
) -> Result<RetryPolicy, Error::InvalidRetryPolicy>;

/// Computes the backoff delay for a given attempt number
fn compute_backoff_delay(attempt: u32, policy: &RetryPolicy) -> u64;
```

### CircuitBreaker
```rust
/// Circuit breaker states
enum CircuitState {
    Closed,     // Normal operation, failures tracked
    HalfOpen,   // Testing if downstream has recovered
    Open,       // Blocking execution, failures exceeded threshold
}

/// Circuit breaker configuration and state
struct CircuitBreaker {
    state: CircuitState,
    failure_threshold: u32,    // Failures to trip the breaker
    success_threshold: u32,    // Successes to close from HalfOpen
    open_duration: Duration,   // Time to wait before HalfOpen
    failure_count: u32,
    last_failure_time: Option<Instant>,
}

/// Records a successful phase execution
fn record_success(cb: &mut CircuitBreaker) -> Result<(), Error>;

/// Records a failed phase execution
fn record_failure(cb: &mut CircuitBreaker) -> Result<(), Error>;

/// Checks if execution is allowed under the current circuit state
fn is_execution_allowed(cb: &CircuitBreaker) -> bool;
```

### PhaseExecutor
```rust
/// Executes a phase with timeout, retry, and circuit breaker protection
async fn execute_phase<P>(
    phase_id: String,
    phase: P,
    timeout: Option<TimeoutPolicy>,
    retry: Option<RetryPolicy>,
    circuit_breaker: Option<&mut CircuitBreaker>,
) -> Result<PhaseResult, Error>
where
    P: FnOnce() -> Future<Output = Result<PhaseResult, Error>>;
```

---

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| P1: timeout_ms > 0 | Compile-time (strongest) | `NonZeroU64` wrapper |
| P2: max_retries >= 0 | Compile-time | `u32` (already non-negative) |
| P3: max_delay > 0 | Compile-time | `Option<NonZeroU64>` |
| P4: failure_threshold > 0 | Compile-time | `NonZeroU32` |
| P5: factor > 1.0 | Runtime-checked constructor | `RetryPolicy::new()` validates |

---

## Violation Examples (REQUIRED)

### Precondition Violations

- **VIOLATES P1**: `new_timeout_policy(0)` -- should produce `Err(Error::InvalidTimeout("timeout must be greater than 0ms".into()))`
- **VIOLATES P1**: `new_timeout_policy(-1)` -- should produce `Err(Error::InvalidTimeout("timeout must be greater than 0ms".into()))` (u64 underflow panic avoided via checked arithmetic)
- **VIOLATES P2**: `new_retry_policy(u32::MAX, 100, 2.0, None, vec![])` with `u32::MAX` as max_retries -- This is allowed (non-negative), but subsequent calls may overflow when computing backoff. Error detected at runtime.
- **VIOLATES P3**: `new_retry_policy(3, 100, 2.0, Some(0), vec![])` -- should produce `Err(Error::InvalidRetryPolicy("max_delay must be greater than 0ms".into()))`
- **VIOLATES P4**: `new_circuit_breaker(0, 3, Duration::from_secs(30))` -- should produce `Err(Error::InvalidRetryPolicy("failure_threshold must be positive".into()))`
- **VIOLATES P5**: `new_retry_policy(3, 100, 1.0, None, vec![])` -- should produce `Err(Error::InvalidRetryPolicy("factor must be greater than 1.0".into()))`
- **VIOLATES P5**: `new_retry_policy(3, 100, 0.5, None, vec![])` -- should produce `Err(Error::InvalidRetryPolicy("factor must be greater than 1.0".into()))`

### Postcondition Violations

- **VIOLATES Q1**: Phase with 100ms timeout that takes 150ms due to scheduling overhead -- should produce `Err(Error::TimeoutExceeded { phase_id: "test", duration_ms: 150, timeout_ms: 100 })` if overhead exceeds allowed margin (implementation should account for scheduling overhead in timeout calculation)
- **VIOLATES Q2**: Phase with max_retries=2 that fails 3 times -- should produce `Err(Error::MaxRetriesExceeded { phase_id: "test", attempts: 3, last_error: <final_error> })`
- **VIOLATES Q4**: Backoff delay computed as 500ms when it should be 400ms (factor=2.0, base=100ms, attempt=2: expected 100*2^2=400) -- implementation bug, delay exceeds expected monotonic increase

### Invariant Violations

- **VIOLATES I1**: Circuit breaker in Closed state after 5 consecutive failures when threshold is 3 -- state machine bug
- **VIOLATES I2**: Backoff delay of 600ms returned when max_delay is 500ms -- delay exceeded cap
- **VIOLATES I3**: Circuit breaker transitions to HalfOpen before open_duration has elapsed -- premature transition
- **VIOLATES I4**: Phase with NonRetryableError being retried -- retry policy not respected

---

## Ownership Contracts (Rust-specific)

### PhaseExecutor
- **Exclusive borrow of circuit breaker**: `circuit_breaker: Option<&mut CircuitBreaker>` -- mutates `failure_count`, `state`, `last_failure_time`
- **Ownership of phase closure**: Phase closure is owned and consumed by `execute_phase`
- **No ownership transfer of timeout/retry policies**: Policies are borrowed (read-only)

### Newtypes for Validation
- `TimeoutMs(pub NonZeroU64)` -- compile-time guarantee of > 0
- `MaxRetries(pub u32)` -- documentation that >= 0 is required
- `CircuitFailureThreshold(pub NonZeroU32)` -- compile-time guarantee of > 0

---

## Non-goals
- Implementation of specific phase business logic
- Persistence of circuit breaker state across process restarts
- Global shared circuit breaker across multiple phase executors
- CancellationToken-based cancellation (future work)
