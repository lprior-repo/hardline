# Contract Specification

## Context
- **Feature**: orchestrator: Add timeout and retry policies
- **Bead ID**: scp-wgv
- **Description**: Add phase-level timeouts, deadline handling, exponential backoff, circuit breakers
- **Domain terms**:
  - `PhaseTimeout`: Maximum duration a phase can run before being cancelled
  - `RetryPolicy`: Configuration for exponential backoff between retries
  - `CircuitBreaker`: State machine to prevent cascading failures
  - `Deadline`: Absolute time by which an operation must complete
- **Assumptions**:
  - PipelineExecutor is the main entry point for phase execution
  - Timeouts are configurable per phase type
  - Circuit breaker tracks failure counts per pipeline execution
- **Open questions**:
  - Should timeouts be configurable at runtime or at pipeline creation?
  - How many consecutive failures should trigger circuit breaker open state?

---

## Preconditions

- **P1**: `timeout.duration_ms > 0` - Timeout duration must be positive
- **P2**: `retry_policy.max_retries >= 0` - Max retries cannot be negative
- **P3**: `retry_policy.base_delay_ms > 0` - Base delay must be positive
- **P4**: `retry_policy.max_delay_ms >= retry_policy.base_delay_ms` - Max delay >= base delay
- **P5**: `circuit_breaker.failure_threshold > 0` - Failure threshold must be positive
- **P6**: `circuit_breaker.recovery_timeout_ms > 0` - Recovery timeout must be positive

---

## Postconditions

- **Q1**: After timeout elapses, phase returns `Err(OrchestratorError::PhaseTimeout)` 
- **Q2**: After max retries exhausted, returns last error or `OrchestratorError::RetriesExhausted`
- **Q3**: Circuit breaker transitions to `Open` after `failure_threshold` consecutive failures
- **Q4**: Circuit breaker transitions to `HalfOpen` after `recovery_timeout_ms` elapses
- **Q5**: After circuit breaker opens, immediate calls return `Err(OrchestratorError::CircuitBreakerOpen)`
- **Q6**: Exponential backoff applies `base_delay_ms * 2^attempt` delay between retries

---

## Invariants

- **I1**: Pipeline state is persisted after each phase completion or failure
- **I2**: Circuit breaker state is scoped to single pipeline execution
- **I3**: Timeout applies to individual phase, not entire pipeline
- **I4**: Retry count resets on successful phase execution

---

## Error Taxonomy

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigError {
    InvalidTimeout { duration_ms: u64 },
    InvalidBaseDelay { delay_ms: u64 },
    InvalidMaxDelay { max_delay_ms: u64, base_delay_ms: u64 },
    InvalidFailureThreshold { threshold: u32 },
    InvalidRecoveryTimeout { timeout_ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestratorError {
    /// Phase execution exceeded timeout duration
    PhaseTimeout { 
        phase: String, 
        timeout_ms: u64, 
        elapsed_ms: u64 
    },
    /// All retry attempts exhausted
    RetriesExhausted { 
        phase: String, 
        attempts: u32, 
        last_error: Box<OrchestratorError> 
    },
    /// Circuit breaker is open, request rejected
    CircuitBreakerOpen { 
        phase: String, 
        failure_count: u32,
        recovery_timeout_ms: u64 
    },
    /// Global pipeline deadline exceeded
    DeadlineExceeded {
        deadline: DateTime<Utc>,
        elapsed_ms: u64,
    },
    /// Generic phase execution error
    PhaseExecution { 
        phase: String, 
        message: String 
    },
}
```

---

## Contract Signatures

### Timeout Configuration
```rust
#[derive(Debug, Clone)]
pub struct PhaseTimeout {
    pub duration_ms: u64,
}

impl PhaseTimeout {
    pub fn new(duration_ms: u64) -> Result<Self, ConfigError>;
    pub fn is_expired(&self, started_at: DateTime<Utc>) -> bool;
}
```

### Retry Policy
```rust
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl RetryPolicy {
    pub fn new(max_retries: u32, base_delay_ms: u64, max_delay_ms: u64) -> Result<Self, ConfigError>;
    pub fn calculate_delay(&self, attempt: u32) -> u64;
}
```

### Circuit Breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,  // Normal operation
    Open,   // Failing, reject calls
    HalfOpen, // Testing if recovery possible
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub failure_threshold: u32,
    pub recovery_timeout_ms: u64,
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub last_failure_at: Option<DateTime<Utc>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout_ms: u64) -> Result<Self, ConfigError>;
    pub fn record_success(&mut self);
    pub fn record_failure(&mut self);
    pub fn can_execute(&self) -> bool;
}

/// Deadline configuration for global pipeline timeout
#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    pub deadline_at: DateTime<Utc>,
}

impl Deadline {
    pub fn from_now(duration_ms: u64) -> Self {
        Self {
            deadline_at: Utc::now() + chrono::Duration::milliseconds(duration_ms as i64),
        }
    }

    pub fn is_exceeded(&self) -> bool {
        Utc::now() > self.deadline_at
    }

    pub fn remaining_ms(&self) -> i64 {
        let remaining = self.deadline_at.signed_duration_since(Utc::now());
        remaining.num_milliseconds().max(0)
    }
}
```

### Orchestrator with Policies
```rust
pub struct PolicyConfig {
    pub timeout: PhaseTimeout,
    pub retry: RetryPolicy,
    pub circuit_breaker: CircuitBreaker,
}

impl PipelineExecutor {
    pub fn with_policies(mut self, config: PolicyConfig) -> Self;
    pub fn run_phase_with_timeout(&mut self, phase: &str) -> Result<PhaseResult, OrchestratorError>;
    pub fn run_phase_with_retry(&mut self, phase: &str) -> Result<PhaseResult, OrchestratorError>;
    pub fn run_phase_with_circuit_breaker(&mut self, phase: &str) -> Result<PhaseResult, OrchestratorError>;
}
```

---

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| timeout.duration_ms > 0 | Runtime-checked constructor | `PhaseTimeout::new() -> Result` |
| retry_policy.max_retries >= 0 | Compile-time | `u32` (unsigned, min 0) |
| retry_policy.base_delay_ms > 0 | Runtime-checked constructor | `RetryPolicy::new() -> Result` |
| retry_policy.max_delay_ms >= base | Runtime-checked constructor | `RetryPolicy::new() -> Result` |
| circuit_breaker.failure_threshold > 0 | Runtime-checked constructor | `CircuitBreaker::new() -> Result` |
| circuit_breaker.recovery_timeout_ms > 0 | Runtime-checked constructor | `CircuitBreaker::new() -> Result` |

---

## Violation Examples (REQUIRED)

### Precondition Violations
- **VIOLATES P1**: `PhaseTimeout::new(0)` → returns `Err(ConfigError::InvalidTimeout { duration_ms: 0 })`
- **VIOLATES P2**: `RetryPolicy::new(0, 100, 1000)` → This is valid (0 retries allowed)
- **VIOLATES P3**: `RetryPolicy::new(3, 0, 1000)` → returns `Err(ConfigError::InvalidBaseDelay { delay_ms: 0 })`
- **VIOLATES P4**: `RetryPolicy::new(3, 100, 50)` → returns `Err(ConfigError::InvalidMaxDelay { max_delay_ms: 50, base_delay_ms: 100 })`
- **VIOLATES P5**: `CircuitBreaker::new(0, 1000)` → returns `Err(ConfigError::InvalidFailureThreshold { threshold: 0 })`
- **VIOLATES P6**: `CircuitBreaker::new(3, 0)` → returns `Err(ConfigError::InvalidRecoveryTimeout { timeout_ms: 0 })`

### Postcondition Violations
- **VIOLATES Q1**: Run phase with 1ms timeout on 100ms operation → returns `Err(OrchestratorError::PhaseTimeout { phase: "test", timeout_ms: 1, elapsed_ms: 100 })`
- **VIOLATES Q3**: Record 5 failures with threshold=3 → state is `CircuitBreakerState::Open`
- **VIOLATES Q5**: Call `can_execute()` when Open → returns `false`

---

## Ownership Contracts (Rust-specific)

- `PipelineExecutor::with_policies()`: Takes `PolicyConfig` by value, clones internally for storage
- `PhaseTimeout`: Copy type, no ownership transfer
- `RetryPolicy`: Copy type, no ownership transfer  
- `CircuitBreaker`: Mutable state required, exclusive borrow `&mut self` for state transitions

---

## Non-goals

- Distributed circuit breakers across multiple nodes
- Dynamic timeout adjustment at runtime
- Retry policies with jitter (randomization)
- Integration with external monitoring systems
