# Contract Specification

## Metadata
- bead_id: scp-z0d
- bead_title: "orchestrator: Add rollback and cleanup on failure"
- phase: CONTRACT_SYNTHESIS
- updated_at: 2026-03-11T00:00:00Z

## Context
- **Feature**: Add cleanup/rollback when phases fail mid-pipeline
- **Domain terms**:
  - Pipeline: A sequential workflow with phases (SpecReview, UniverseSetup, AgentDevelopment, Validation)
  - Phase: A discrete step in the pipeline that can succeed or fail
  - Rollback: The act of undoing side effects from a failed phase
  - Cleanup: Releasing resources (files, processes, state) created during execution
- **Assumptions**:
  - Each phase may create side effects that need cleanup
  - The orchestrator tracks which phase is currently executing
  - Pipeline state is persisted between phase transitions
  - Rollback is needed for SpecReview, UniverseSetup, and AgentDevelopment phases
- **Open questions**:
  - What external resources does UniverseSetup create that need cleanup?
  - Does AgentDevelopment spawn subprocesses that need killing?
  - Do we need a transaction log for rollback?

## Preconditions
- [P1] Pipeline must be in a non-terminal state (Pending, SpecReview, UniverseSetup, AgentDevelopment, Validation)
- [P2] Phase execution must have started (state >= SpecReview)
- [P3] Failed phase must be one that creates cleanup-relevant side effects

## Postconditions
- [Q1] On phase failure, orchestrator transitions pipeline to Failed/Escalated state
- [Q2] On phase failure, cleanup handlers are invoked for the failed phase
- [Q3] On phase failure, any in-progress side effects are reverted or released
- [Q4] Pipeline state is persisted after cleanup completes
- [Q5] Error details are recorded in pipeline.last_error

## Invariants
- [I1] Pipeline state is always valid (matches defined state machine)
- [I2] If a phase fails, the pipeline ends in a terminal state (Failed or Escalated)
- [I3] Cleanup is idempotent - multiple cleanup calls don't cause errors
- [I4] No resources remain after cleanup that were created during failed phase

## Error Taxonomy
- `PhaseError::SpecReviewFailed` - Spec linting or validation failed
- `PhaseError::SetupFailed` - Universe/twin setup failed
- `PhaseError::DevelopmentFailed` - Agent development failed
- `PhaseError::ValidationFailed` - Scenario validation failed
- `PhaseError::CleanupFailed` - Cleanup/rollback of previous phase failed
- `PhaseError::PersistenceFailed` - State store write failed
- `PhaseError::InvalidStateTransition` - Attempted invalid state change

## Contract Signatures

### PipelineExecutor
```rust
// Execute pipeline with automatic cleanup on failure
pub fn run_pipeline(&mut self, pipeline_id: &PipelineId) -> Result<Decision, PhaseError>;

// Rollback side effects from a specific phase
pub fn rollback_phase(&self, pipeline: &Pipeline, phase: PhaseType) -> Result<(), PhaseError>;

// Cleanup resources after phase failure
pub fn cleanup_after_failure(&self, pipeline: &Pipeline) -> Result<(), PhaseError>;
```

### CleanupTrait (new)
```rust
pub trait CleanupHandler: Send + Sync {
    fn cleanup(&self, context: &CleanupContext) -> Result<(), CleanupError>;
    fn phase_type(&self) -> PhaseType;
}

pub struct CleanupContext {
    pub pipeline_id: PipelineId,
    pub failed_phase: PhaseType,
    pub created_resources: Vec<ResourceId>,
    pub rollback_data: Vec<u8>,
}
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| Pipeline non-terminal | Compile-time | `PipelineState: is_terminal()` guard |
| Phase started | Runtime check | `PipelineState >= SpecReview` |
| Valid phase for cleanup | Enum + match | `enum PhaseType { ... }` matching current state |

## Violation Examples

- VIOLATES P1: `run_pipeline()` with pipeline in `Accepted` state -- should return `Err(PhaseError::InvalidStateTransition)`
- VIOLATES P2: `run_pipeline()` with pipeline in `Pending` state without starting -- should return `Err(PhaseError::InvalidStateTransition)`
- VIOLATES Q1: Phase fails but pipeline state remains non-terminal -- should produce `Err(PhaseError::PersistenceFailed)` or panic
- VIOLATES Q2: Phase fails but no cleanup handler invoked -- state leak
- VIOLATES Q3: Cleanup called but resources still exist -- should produce `Err(PhaseError::CleanupFailed)`

## Ownership Contracts

- `PipelineExecutor` owns `StateStore` and `Metrics` - both read/write
- `CleanupContext` contains borrowed data from pipeline - no ownership transfer
- `CleanupHandler` implementations are owned by the executor but invoked externally
- No clone operations needed - pipeline and state are moved through transitions

## Non-goals
- [ ] Automatic retry logic (handled elsewhere)
- [ ] Human intervention workflow for Escalated state
- [ ] Distributed transactions across multiple pipelines
- [ ] Partial rollback (either all-or-nothing)
