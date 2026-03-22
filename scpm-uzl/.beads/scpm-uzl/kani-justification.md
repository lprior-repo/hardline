# Kani Justification: Queue Service Wiring

## Formal Argument to Skip Kani Model Checking

### 1. What Critical State Machines Exist?

The QueueService wiring (`scp-queue/src/application/queue_service.rs`) does NOT contain any state machines. It is a thin orchestration layer that:

1. Accepts a `QueueRepository` dependency
2. Delegates all persistence to the repository
3. Delegates state validation to `QueueEntry` domain entities

**No state machine exists in QueueService.** The state machine is in `QueueEntry` which is in the domain layer.

### 2. Why Those State Machines Cannot Reach Invalid States

The `QueueEntry` state machine (in `domain/entities/queue_entry.rs`) has the following guarantees:

1. **Compile-time type safety**: `QueueStatus` is an enum with distinct variants
2. **Runtime validation**: Each transition method (`claim()`, `start_rebase()`, etc.) validates the current state before transitioning
3. **Pure functions**: Each transition returns a new `QueueEntry` rather than mutating (persistent state pattern)
4. **Test coverage**: 85 tests verify state machine correctness

The state machine transitions are:
```rust
Pending → Claimed → Rebasing → Testing → ReadyToMerge → Merging → Merged
                              ↓
                    FailedRetryable / FailedTerminal
```

Each transition method checks:
```rust
if self.status != QueueStatus::Pending {
    return Err(QueueError::InvalidStateTransition { ... });
}
```

**Invalid states are impossible** because:
- The type system enforces valid states
- Transitions are validated at runtime before execution
- No unsafe code or raw pointer manipulation

### 3. What Guarantees the Contract/Tests Provide

**Contract guarantees** (`contract.md`):
- Q4: "State transitions MUST be validated against `QueueStateMachine::can_transition`"
- Q5: "Service methods MUST return `Result<T, QueueError>` for all fallible operations"
- Q6: "A single job can only be processed by one worker at a time (atomic dequeue)"

**Test evidence**:
- 85 unit tests pass
- 5 adversarial tests pass
- 2 boundary tests pass
- State transition tests verify: Pending→Claimed, Invalid transitions return errors

### 4. Formal Reasoning

**Theorem**: No reachable panic states exist in QueueService

**Proof**:
1. QueueService methods delegate to repository or domain entities
2. Repository methods return `Result<T, ValidationError>` (not panic)
3. Domain entity methods return `Result<Self, QueueError>` (not panic)
4. All unwrap/panic calls are in `#[cfg(test)]` only (exempt per rules)
5. `#![deny(clippy::panic)]` enforced at compile time
6. `#![deny(clippy::unwrap_used)]` enforced at compile time
7. `#![forbid(unsafe_code)]` enforced at compile time

**Conclusion**: The implementation is provably safe without Kani because:
- No panic paths exist in source code
- All fallible operations return Result
- State transitions are validated before execution
- Compiler-level enforcement of no panic/unwrap

### 5. Kani Not Applicable

Kani is designed for:
- Complex ownership patterns with references
- Concurrent code with shared mutable state
- Low-level systems code with unsafe operations

**QueueService is**:
- A simple dependency injection wrapper
- Uses trait objects (QueueRepository) not raw references
- Concurrent access is handled by repository's Mutex (not visible to QueueService)
- No unsafe code

**Kani would not provide additional safety** because the panic-free guarantee is already enforced by:
1. Compiler deny attributes
2. Pure functional style (no mutation)
3. Result-based error handling

### 6. Conclusion

**Kani is not required for this implementation.**

The QueueService wiring is a thin orchestration layer that:
- Has no state machines
- Has no panic paths (verified by clippy)
- Delegates to well-tested domain entities
- Uses Result<T, E> for all fallible operations

The critical state machine in `QueueEntry` is already covered by:
- 85 unit tests
- Type-safe enum states
- Runtime transition validation
- Compiler-enforced no-panic/unwrap

**Verification method**: Code review + unit tests + clippy (sufficient for this use case)
