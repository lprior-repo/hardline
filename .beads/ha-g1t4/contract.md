bead_id: ha-g1t4
bead_title: Test: CircuitBreaker — closed→open→half-open→closed cycle
phase: p3
updated_at: 2026-04-06T04:00:00Z

# CircuitBreaker Test Contract

## Two Implementations

### 1. `circuit_breaker.rs` — Explicit state machine (primary target)
- `CircuitState`: Closed, HalfOpen, Open
- `CircuitBreaker::new(failure_threshold, success_threshold, open_duration_ms)`
- Explicit `check_and_transition(elapsed_ms)` for Open→HalfOpen
- `record_success()` / `record_failure()` drive transitions
- `is_execution_allowed()` guards execution

### 2. `circuit.rs` — Chrono-based with implicit transitions
- `CircuitBreakerState`: Closed, Open, HalfOpen (Serializable)
- `CircuitBreaker::new(failure_threshold, recovery_timeout_ms)`
- `can_execute()` implicitly checks timeout for Open→HalfOpen
- `try_transition_to_half_open()` explicit transition
- `record_success()` / `record_failure()` drive transitions

## State Machine Invariants

### I1: Closed State
- Starts in Closed
- `is_execution_allowed()` / `can_execute()` always true
- `record_failure()` increments failure_count
- `record_success()` resets failure_count to 0
- Transitions to Open when failure_count >= failure_threshold

### I2: Open State
- `is_execution_allowed()` returns false
- `record_success()` and `record_failure()` are no-ops (state unchanged, counts unchanged)
- Only exits via `check_and_transition(elapsed >= open_duration)` → HalfOpen

### I3: HalfOpen State
- `is_execution_allowed()` returns true
- `record_success()` increments success_count
  - If success_count >= success_threshold → Closed (resets both counts)
- `record_failure()` → Open immediately (resets success_count)

### I4: Transition Constraints
- Closed → Open: only via failure threshold
- Open → HalfOpen: only via check_and_transition with sufficient elapsed time
- HalfOpen → Closed: only via success threshold
- HalfOpen → Open: only via single failure
- No other transitions exist

## Test Gaps (identified from existing coverage)

### circuit_breaker.rs gaps:
1. No proptest for arbitrary operation sequences maintaining state invariants
2. No test for exact boundary: elapsed_ms == open_duration (only tested >= and <)
3. No test for multiple complete lifecycle cycles (closed→open→halfopen→closed × N)
4. No proptest for full lifecycle with random parameters
5. No test verifying failure_count clamps at threshold (doesn't exceed it)

### circuit.rs gaps:
1. No proptests at all
2. No test for repeated lifecycle cycles
3. No test for can_execute in closed state always returning true
4. Missing: failure_count increments correctly without premature opening
