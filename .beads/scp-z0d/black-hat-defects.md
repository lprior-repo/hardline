# Black Hat Review - Defects

## Metadata
- bead_id: scp-z0d
- status: REJECTED
- reviewed_at: 2026-03-11T00:00:00Z

---

## CRITICAL DEFECTS (Must Fix)

### D1: Contract Signature Mismatch - PhaseError vs anyhow::Error
**Location**: `phases.rs:139`
**Contract**: 
```rust
pub fn run_pipeline(&mut self, pipeline_id: &PipelineId) -> Result<Decision, PhaseError>;
```
**Actual**:
```rust
pub fn run_pipeline(&mut self, pipeline_id: &PipelineId) -> Result<Decision>
```
**Violation**: Contract specifies `PhaseError` but implementation uses `anyhow::Error`. The contract's error taxonomy (PhaseError::SpecReviewFailed, CleanupFailed, etc.) is not implemented.

**Fix**: Define `PhaseError` enum matching contract and return `Result<Decision, PhaseError>`

---

### D2: Q4 Violation - State Persisted BEFORE Cleanup
**Location**: `phases.rs:397-410` (handle_spec_failure), `412-428` (handle_setup_failure), `430-446` (handle_dev_failure)

**Contract Q4**: "Pipeline state is persisted after cleanup completes"

**Actual order in handle_spec_failure**:
```rust
// Line 399-405: FIRST - Persist state
let pipeline_opt = self.store.get_mut(id).ok().map(|p| {
    let _ = p.transition_to(PipelineState::Failed);  // State change
    p.set_error(message);
    p.clone()
});
if let Some(pipeline) = pipeline_opt {
    let _ = self.store.update(pipeline);  // PERSISTED HERE
}

// Line 408: SECOND - Call cleanup
let _ = self.cleanup_after_failure(id);  // CLEANUP AFTER
```

**Violation**: State is persisted BEFORE cleanup, not after. Same pattern in handle_setup_failure and handle_dev_failure.

**Fix**: Reorder to call cleanup FIRST, THEN persist state after cleanup completes successfully.

---

### D3: Function Length Violation - run_pipeline() 76 Lines
**Location**: `phases.rs:139-214`
**Constraint**: Functions MUST be <25 lines (Farley Hard Constraint)
**Actual**: `run_pipeline()` is 76 lines - exceeds limit by 3x

**Fix**: Decompose into smaller functions:
- `run_spec_review_phase()`
- `run_universe_setup_phase()`
- `run_agent_development_phase()`
- `run_validation_phase()`

---

## HIGH SEVERITY DEFECTS

### D4: Silent Error Ignorance - 11 instances of `let _ =`
**Locations**:
- Line 400: `let _ = p.transition_to(PipelineState::Failed);`
- Line 405: `let _ = self.store.update(pipeline);`
- Line 408: `let _ = self.cleanup_after_failure(id);`
- Line 415: `let _ = p.transition_to(PipelineState::Escalated);`
- Line 420: `let _ = self.store.update(pipeline);`
- Line 423: `let _ = self.cleanup_after_failure(id);`
- Line 424-425: `if let Ok(pipeline) = self.store.get(id) { let _ = self.rollback_phase(pipeline, PhaseType::UniverseSetup); }`
- Line 433: `let _ = p.transition_to(PipelineState::Escalated);`
- Line 438: `let _ = self.store.update(pipeline);`
- Line 441: `let _ = self.cleanup_after_failure(id);`
- Line 442-443: `if let Ok(pipeline) = self.store.get(id) { let _ = self.rollback_phase(pipeline, PhaseType::AgentDevelopment); }`

**Contract Q3**: "On phase failure, any in-progress side effects are reverted or released"

**Violation**: Cleanup and rollback failures are silently ignored. If cleanup fails, the code proceeds as if it succeeded.

**Fix**: Propagate cleanup/rollback errors or at minimum return a compound error that includes cleanup failures.

---

### D5: CleanupResult.success Never Checked
**Location**: `phases.rs:85-109` (cleanup_after_failure)

**Contract Q2**: "On phase failure, cleanup handlers are invoked for the failed phase"

The code invokes handlers but never checks if they succeeded:
```rust
let result = self.cleanup_manager.cleanup(&context);

if !result.success {
    warn!("Cleanup had errors...");  // Only logs, doesn't propagate
}
```

**Fix**: If cleanup fails (result.success == false), should return Err(PhaseError::CleanupFailed) to satisfy Q3.

---

## MEDIUM SEVERITY DEFECTS

### D6: Precondition P2 Not Enforced
**Location**: `phases.rs:139-214`

**Contract P2**: "Phase execution must have started (state >= SpecReview)"

**Actual**: `run_pipeline()` accepts Pending state and starts spec_review() from scratch. While spec_review() does transition to SpecReview, the contract implies the phase should already be in progress.

**Note**: This may be acceptable depending on interpretation - Pending → SpecReview is a valid transition. But the contract wording suggests requiring state >= SpecReview.

---

## LOW SEVERITY DEFECTS

### D7: Unnecessary Clone in Hot Path
**Location**: `phases.rs:142`
```rust
let mut pipeline = self.store.get(pipeline_id)?.clone();
```
Pipeline is cloned on every run. Could work with reference if StateStore API supports it.

---

### D8: Unused Methods - Never Called
**Location**: `cleanup.rs`
- `CleanupContext::add_resource()` - never called by any caller
- `CleanupContext::set_rollback_data()` - never called by any caller
- `CleanupManager::register_handler()` - never called

These are defined but not used, violating YAGNI.

---

## VERIFICATION REQUIRED

After fixes, verify:
```bash
cargo clippy -p orchestrator -- -D warnings
cargo test -p orchestrator
```

---

## STATUS: REJECTED

**Required Actions**:
1. Define and use `PhaseError` enum (D1)
2. Reorder cleanup to happen BEFORE state persistence (D2)
3. Decompose `run_pipeline()` into <25 line functions (D3)
4. Propagate cleanup/rollback errors instead of silencing (D4, D5)

STATUS: REJECTED
