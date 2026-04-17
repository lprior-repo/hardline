# BLACK HAT REVIEW: orchestrator crate

**Reviewer**: polecat-eta (adversarial audit)
**Date**: 2026-04-17
**Scope**: `crates/orchestrator/` (31 source files, ~6400 LOC production + ~8500 LOC tests)
**Verdict**: **REJECT** — 5 CRITICAL, 7 HIGH, 6 MEDIUM, 4 LOW

---

## PHASE 1: Contract & Bead Parity

### CRITICAL-1: Broken Test — `CleanupResult` field access on non-existent `success`

**File**: `src/cleanup_tests.rs:21,27`
**Severity**: CRITICAL

The test accesses `result.success` but `CleanupResult` has no `success` field — only `status: CleanupStatus` and `cleaned_resources: Vec<ResourceId>`. The correct accessor is `result.success_flag()`.

```rust
// cleanup_tests.rs:19-30 — BROKEN
fn test_cleanup_result() {
    let result = CleanupResult::success();
    assert!(result.success);  // ← DOES NOT COMPILE
    // ...
    assert!(!result.success); // ← DOES NOT COMPILE
}
```

This means either: (a) `cargo test` for this crate fails, or (b) the test file is dead code. Either way, **CI is broken or the test is a lie**.

### CRITICAL-2: Duplicate Type Systems — Two of Everything

The crate maintains **parallel, redundant type hierarchies** that are both publicly exported:

| Concept | Implementation A | Implementation B |
|---------|-----------------|-----------------|
| Circuit Breaker | `policies/circuit.rs` (CircuitBreaker) | `policies/circuit_breaker.rs` (NewCircuitBreaker) |
| Timeout | `policies/timeout.rs` (PhaseTimeout, raw u64) | `policies/timeout_policy.rs` (TimeoutPolicy, NonZeroU64) |
| Error Hierarchy | `policies/errors.rs` (ConfigError + OrchestratorError) | `policies/timeout_error.rs` (TimeoutError + PolicyError) |
| Circuit State | `policies/circuit.rs` (CircuitBreakerState) | `policies/circuit_breaker.rs` (CircuitState) |

**Both** pairs are publicly exported from `lib.rs:33-37`. No deprecation notices. No migration path. A consumer has no idea which to use.

### HIGH-1: Type Mismatch — Metrics Uses Raw Strings Instead of Domain Types

**File**: `src/metrics.rs:20-37`

`PhaseMetrics.pipeline_id` is `String`, not `PipelineId`. `PhaseMetrics.phase` is `String`, not a newtype. `PipelineMetrics.final_state` is `String`, not `PipelineState`.

```rust
pub struct PhaseMetrics {
    pub pipeline_id: String,  // ← should be PipelineId
    pub phase: String,        // ← should be PhaseType or newtype
    // ...
}
pub struct PipelineMetrics {
    pub pipeline_id: String,  // ← should be PipelineId
    pub final_state: String,  // ← should be PipelineState
    // ...
}
```

This means the type system cannot enforce correctness — any arbitrary string can be passed as a pipeline ID or state name.

---

## PHASE 2: Farley Engineering Rigor

### HIGH-2: Functions Exceeding 25-Line Limit

| File | Function | Lines (approx) |
|------|----------|----------------|
| `metrics.rs` | `aggregated()` | ~60 lines |
| `impl_phases.rs` | `spec_review()` | ~35 lines |
| `impl_parallel.rs` | `execute_with_dependency_graph()` | ~35 lines |
| `impl_pipeline.rs` | `handle_validation_decision()` | ~32 lines |
| `state.rs` | `transition_to()` | ~30 lines |
| `persistence.rs` | `mutate_and_persist()` | ~22 lines |

### HIGH-3: I/O Hidden Inside Supposedly Pure Calculation

**File**: `src/policies/timeout.rs:29-38`

`PhaseTimeout::is_expired()` and `elapsed_ms()` call `Utc::now()` — impure wall-clock I/O inside what should be a pure timeout check. Compare with `NewCircuitBreaker` which correctly injects elapsed time via `check_and_transition(elapsed_ms)`.

**File**: `src/policies/circuit.rs:93-107`

`CircuitBreaker::can_execute()` calls `Utc::now()` — same violation. The `NewCircuitBreaker` (circuit_breaker.rs) fixes this with `check_and_transition(elapsed_ms: u64)`.

**File**: `src/policies/deadline.rs:34-42`

`Deadline::is_exceeded()` and `remaining_ms()` call `Utc::now()`.

### HIGH-4: Tests Assert Implementation Details (HOW), Not Behavior (WHAT)

**File**: `tests/bdd_orchestrator.rs:313-324`

```rust
fn claim_quality_threshold_not_validated() {
    // Tests that quality_threshold accepts 999 — an implementation detail
    // about what validation is MISSING, not what behavior EXISTS
    assert_eq!(p.quality_threshold, 999);
}
```

Multiple tests assert field values directly instead of testing observable behavior through the public API. Tests like `test_pipeline_hash_and_eq_for_pipeline_id` (state.rs:864) test Rust stdlib behavior (HashSet/Hash), not domain logic.

---

## PHASE 3: NASA-Level Functional Rust (The Big 6)

### CRITICAL-3: Pipeline Struct Allows Illegal States via Public Fields

**File**: `src/state.rs:111-122`

```rust
pub struct Pipeline {
    pub id: PipelineId,
    pub spec_path: String,
    pub state: PipelineState,      // ← pub
    pub iteration: u32,            // ← pub
    pub max_iterations: u32,       // ← pub
    pub quality_threshold: u32,    // ← pub
    pub created_at: DateTime<Utc>, // ← pub
    pub updated_at: DateTime<Utc>, // ← pub
    pub last_error: Option<String>, // ← pub
}
```

All fields are `pub`. Anyone can construct `Pipeline { state: PipelineState::Accepted, iteration: 0, .. }` — a pipeline that is "accepted" but has never run. The `transition_to()` method enforces state machine rules, but **any code can bypass it by setting `pipeline.state = ...` directly**.

The state machine is a suggestion, not a guarantee. This is the opposite of "make illegal states unrepresentable."

### CRITICAL-4: `PipelineConfig` Has No Validation Whatsoever

**File**: `src/state.rs:87-108`

```rust
pub struct PipelineConfig {
    pub max_iterations: u32,       // can be 0, u32::MAX, anything
    pub quality_threshold: u32,    // can be > 100, 0, anything
    pub scenarios_path: String,    // can be empty
    pub linter_path: Option<String>,
}
```

No constructor. No validation. `Default` impl sets `quality_threshold: 80` but nothing prevents `PipelineConfig { quality_threshold: 999999 }`. The tests even explicitly verify this (`claim_quality_threshold_not_validated`).

### MEDIUM-1: Inconsistent Use of Newtypes for Validation

- `TimeoutPolicy` (timeout_policy.rs): Uses `NonZeroU64` — correct
- `PhaseTimeout` (timeout.rs): Uses raw `u64` with runtime validation — inconsistent
- `RetryPolicy` (retry_policy.rs): Uses `NonZeroU64` for `base_delay_ms` — correct
- `CircuitBreaker` (circuit.rs): Uses raw `u32` with runtime validation — inconsistent
- `NewCircuitBreaker` (circuit_breaker.rs): Uses `NonZeroU32` and `NonZeroU64` — correct

The "New" prefix on `NewCircuitBreaker` and `NewCircuitBreakerError` is a code smell — it means the old version should have been deleted.

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

### HIGH-5: Queue Module Is Dead Code — Never Used by Executor

**Files**: `src/queue/types.rs`, `src/queue/processor.rs`, `src/queue/repository.rs`

The `Queue` module (Job, JobProcessor, JobRepository, etc.) is 800+ lines of async job processing infrastructure. The `PipelineExecutor` never uses it. The queue is exported from `lib.rs:38-41` but nothing in the crate consumes it.

This is pure YAGNI — a framework built for hypothetical future use.

### HIGH-6: Duplicate Parallel Execution Systems

**File A**: `src/parallel.rs` — `DependencyGraph`, `ParallelExecutor`, `PhaseGroup`
**File B**: `src/phases/exec/impl_parallel.rs` — `execute_parallel_phases`, `execute_phase_group`

Both provide dependency graph resolution and parallel phase execution. The `impl_parallel.rs` version is used by the executor; the `parallel.rs` version is used by the BDD tests. Two implementations of the same concept.

### MEDIUM-2: Stub Implementations Disguised as Production Code

**File**: `src/phases/exec/impl_phases.rs:66-70`
```rust
#[must_use]
fn run_linter(&self, _spec_path: &str) -> u32 {
    debug!("Running linter on spec");
    85  // ← HARDCODED
}
```

**File**: `src/phases/exec/impl_phases.rs:174-191`
```rust
fn run_scenarios(&self, _pipeline: &Pipeline) -> Vec<ScenarioResult> {
    vec![
        ScenarioResult { name: "happy_path".into(), passed: true, ... },
        ScenarioResult { name: "edge_case".into(), passed: true, ... },
    ]  // ← HARDCODED
}
```

**File**: `src/cleanup.rs:184-213`
```rust
impl CleanupHandler for UniverseSetupCleanupHandler {
    fn cleanup(&self, context: &CleanupContext) -> CleanupResult {
        // Placeholder: In production, actually clean up resources
        for resource in &context.created_resources {
            result = result.with_resource(resource.clone());  // ← NOOP
        }
    }
}
```

The linter always returns 85. Scenarios always pass. Cleanup handlers are no-ops that just acknowledge resources. The entire `run_pipeline` happy path succeeds not because the code works, but because every phase is a stub.

### MEDIUM-3: `#[allow(dead_code)]` on PipelineExecutor Fields

**File**: `src/phases/exec/executor.rs:14,18-19`
```rust
#[allow(dead_code)]
pub struct PipelineExecutor {
    #[allow(dead_code)]
    scenarios_path: PathBuf,
    #[allow(dead_code)]
    linter_path: Option<PathBuf>,
```

If fields are dead code, delete them. If they're needed, use them. Don't suppress the linter.

### MEDIUM-4: `CleanupResult` Builder Creates Unnecessary Clones

**File**: `src/cleanup.rs:118-131`
```rust
pub fn with_error(mut self, error: String) -> Self {
    let errors = match &mut self.status {
        CleanupStatus::Success => { vec![error] }
        CleanupStatus::Failed(errs) => {
            errs.push(error);
            errs.clone()  // ← UNNECESSARY CLONE
        }
    };
    self.status = CleanupStatus::Failed(errors);
    self
}
```

`errs.clone()` allocates a new Vec just to assign it back. Could use `std::mem::take` or restructure.

### LOW-1: `PipelineId(String)` Tuple Struct with No Invariant Enforcement

**File**: `src/state.rs:7-8`

`PipelineId` wraps `String` but accepts anything — empty strings, path traversal (`"../../../etc/passwd"`), etc. The BDD test even verifies path traversal works (`claim_pipeline_id_special_chars`). A newtype should enforce a format.

---

## PHASE 5: The Bitter Truth (Velocity & Legibility)

### MEDIUM-5: `anyhow` Imported But Not Used in Persistence

**File**: `src/persistence.rs:9`
```rust
use anyhow::Result;
```

`Result` here resolves to `anyhow::Result<T>` which is `std::result::Result<T, anyhow::Error>`. But all functions return `Result<T, StoreError>`, making the `anyhow` import misleading dead weight. The `?` operator works because `StoreError` has `#[from]` conversions, not because of anyhow.

### LOW-2: Overly Granular Module Splitting

`phases/exec/` contains 7 files for a state machine runner that's essentially:
1. Call phase function
2. If success, transition state
3. If failure, cleanup and escalate

This doesn't need `types.rs`, `executor.rs`, `impl_phases.rs`, `impl_pipeline.rs`, `impl_failure.rs`, `impl_state.rs`, and `impl_parallel.rs`. That's 7 files for ~600 LOC of production code. A single `phases.rs` would be clearer.

### LOW-3: `im::Vector` Used Where `Vec` Would Suffice

**File**: `src/metrics.rs:7,55`
```rust
use im::Vector;
// ...
phase_metrics: Vector<PhaseMetrics>,
```

`im::Vector` is a persistent (immutable) data structure. `Metrics` is already `Clone` and all methods take `&mut self`. The `im::Vector` provides no benefit over `Vec` here — it's just slower and adds a dependency.

### LOW-4: `PipelineConfig` Has `Default` but Also `Pipeline::new()` Duplicates Defaults

**File**: `src/state.rs:99-108` vs `src/state.rs:126-139`

`PipelineConfig::default()` sets `max_iterations: 10, quality_threshold: 80`. `Pipeline::new()` hardcodes the same values inline. If defaults change, two places must be updated.

---

## Summary Table

| ID | Severity | Phase | Finding | File |
|----|----------|-------|---------|------|
| C-1 | CRITICAL | 1 | Broken test: `result.success` field doesn't exist | cleanup_tests.rs:21,27 |
| C-2 | CRITICAL | 1 | Duplicate type systems: 2 circuit breakers, 2 timeouts, 2 error hierarchies | policies/ |
| C-3 | CRITICAL | 3 | Pipeline struct: all fields pub, state machine bypassable | state.rs:111-122 |
| C-4 | CRITICAL | 3 | PipelineConfig: zero validation on any field | state.rs:87-108 |
| C-5 | CRITICAL | 4 | Entire pipeline is stubs — linter returns 85, scenarios hardcoded | impl_phases.rs:67,174 |
| H-1 | HIGH | 1 | Metrics uses `String` instead of domain types | metrics.rs:20-37 |
| H-2 | HIGH | 2 | 6 functions exceed 25-line Farley limit | multiple |
| H-3 | HIGH | 2 | I/O (Utc::now) hidden in pure timeout/circuit checks | timeout.rs:29, circuit.rs:93, deadline.rs:34 |
| H-4 | HIGH | 2 | Tests assert implementation details, not behavior | bdd_orchestrator.rs, state.rs |
| H-5 | HIGH | 4 | Queue module: 800+ LOC dead code, never used by executor | queue/ |
| H-6 | HIGH | 4 | Duplicate parallel execution systems | parallel.rs, impl_parallel.rs |
| H-7 | HIGH | 5 | Placeholder cleanup handlers that do nothing | cleanup.rs:184-249 |
| M-1 | MEDIUM | 3 | Inconsistent newtype usage (NonZeroU64 vs raw u64) | policies/ |
| M-2 | MEDIUM | 4 | Stub implementations disguised as production code | impl_phases.rs |
| M-3 | MEDIUM | 4 | `#[allow(dead_code)]` on executor fields | executor.rs:14,18 |
| M-4 | MEDIUM | 4 | Unnecessary clone in CleanupResult builder | cleanup.rs:125 |
| M-5 | MEDIUM | 5 | Misleading `anyhow` import in persistence | persistence.rs:9 |
| M-6 | MEDIUM | 5 | PipelineConfig defaults duplicated in Pipeline::new() | state.rs |
| L-1 | LOW | 3 | PipelineId accepts any string (path traversal, empty) | state.rs:7 |
| L-2 | LOW | 5 | Overly granular module splitting (7 files for ~600 LOC) | phases/exec/ |
| L-3 | LOW | 5 | `im::Vector` provides no benefit over `Vec` | metrics.rs:55 |
| L-4 | LOW | 5 | CleanupStatus::Failed clones unnecessarily | cleanup.rs:125 |

---

## VERDICT: **REJECT**

5 CRITICAL findings. The orchestrator crate is in a **transitional state** — it has both old and new implementations of the same concepts, the core pipeline is entirely stubbed, the state machine is bypassable via public fields, and there's a broken test that suggests CI may not be passing.

**Required actions before resubmission:**

1. **Delete** the old implementations: `circuit.rs`, `timeout.rs`, `timeout_error.rs`. Keep only `circuit_breaker.rs`, `timeout_policy.rs`, `errors.rs`.
2. **Make Pipeline fields private** with constructor-only access. Use typestate pattern or builder.
3. **Add validation to PipelineConfig** or use the "New" pattern consistently.
4. **Replace String with domain types** in Metrics (`PipelineId`, `PhaseType`, `PipelineState`).
5. **Delete the queue module** or wire it into the executor. Dead code is technical debt.
6. **Fix or delete the broken test** in cleanup_tests.rs.
7. **Remove `#[allow(dead_code)]`** — delete the dead code instead.
