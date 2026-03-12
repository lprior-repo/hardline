# Martin Fowler Test Plan

## Metadata
- bead_id: scp-z0d
- bead_title: "orchestrator: Add rollback and cleanup on failure"
- phase: CONTRACT_SYNTHESIS
- updated_at: 2026-03-11T00:00:00Z

## Happy Path Tests
- `test_runs_full_pipeline_successfully`
  - Given: A valid pipeline in Pending state with all phases configured
  - When: `run_pipeline()` is called
  - Then: All phases execute sequentially, pipeline ends in Accepted state

- `test_pipeline_transitions_through_all_phases`
  - Given: Pipeline at Pending state
  - When: Phases execute in order: SpecReview → UniverseSetup → AgentDevelopment → Validation
  - Then: Pipeline state progresses correctly through each phase

- `test_creates_pipeline_with_spec_path`
  - Given: Valid spec path
  - When: `create_pipeline()` is called
  - Then: Pipeline is created with Pending state and correct spec_path

## Error Path Tests
- `test_fails_pipeline_on_spec_review_failure`
  - Given: Pipeline in Pending state with invalid spec
  - When: SpecReview phase fails
  - Then: Pipeline transitions to Failed, cleanup is invoked

- `test_fails_pipeline_on_setup_failure`
  - Given: Pipeline in UniverseSetup state
  - When: UniverseSetup phase fails
  - Then: Pipeline transitions to Escalated, cleanup is invoked

- `test_fails_pipeline_on_development_failure`
  - Given: Pipeline in AgentDevelopment state
  - When: Development phase fails
  - Then: Pipeline transitions to Escalated, cleanup is invoked

- `test_rollback_on_phase_failure`
  - Given: Pipeline with a failed phase
  - When: `rollback_phase()` is called
  - Then: All side effects from that phase are reverted

- `test_cleanup_after_failure`
  - Given: Pipeline that failed mid-execution
  - When: `cleanup_after_failure()` is called
  - Then: All resources created during failed execution are released

## Edge Case Tests
- `test_handles_duplicate_cleanup_calls`
  - Given: Cleanup handler already invoked
  - When: Cleanup is called again
  - Then: Returns Ok (idempotent), no error

- `test_handles_cleanup_failure_gracefully`
  - Given: Cleanup handler fails
  - When: Cleanup is invoked after phase failure
  - Then: Error is logged but pipeline still transitions to terminal state

- `test_handles_empty_resource_list`
  - Given: Failed phase with no created resources
  - When: Cleanup is invoked
  - Then: Returns Ok immediately

- `test_handles_invalid_state_transition`
  - Given: Pipeline in Accepted (terminal) state
  - When: Attempting to run pipeline
  - Then: Returns `Err(PhaseError::InvalidStateTransition)`

## Contract Verification Tests

### Precondition Tests
- `test_precondition_p1_pipeline_not_terminal`
  - Given: Pipeline in Failed state
  - When: `run_pipeline()` is called
  - Then: Returns `Err(PhaseError::InvalidStateTransition)`

- `test_precondition_p2_phase_started`
  - Given: Pipeline in Pending state
  - When: `run_pipeline()` is called
  - Then: Transitions to SpecReview before execution

- `test_precondition_p3_valid_failed_phase`
  - Given: Pipeline with invalid phase indicator
  - When: Cleanup is invoked
  - Then: Returns appropriate error

### Postcondition Tests
- `test_postcondition_q1_terminal_state_on_failure`
  - Given: Phase fails during execution
  - When: Phase completes with failure
  - Then: Pipeline state is Failed or Escalated

- `test_postcondition_q2_cleanup_invoked`
  - Given: Phase fails
  - When: Failure is detected
  - Then: Cleanup handler for that phase is invoked

- `test_postcondition_q3_effects_reverted`
  - Given: Failed phase with side effects
  - When: Rollback completes
  - Then: Side effects are undone

- `test_postcondition_q4_state_persisted`
  - Given: Cleanup completes
  - When: State store is checked
  - Then: Pipeline state reflects post-cleanup status

- `test_postcondition_q5_error_recorded`
  - Given: Phase fails
  - When: Failure handler processes error
  - Then: pipeline.last_error contains error details

### Invariant Tests
- `test_invariant_i1_valid_state_at_all_times`
  - Given: Any pipeline operation
  - When: State transitions occur
  - Then: State is always a valid PipelineState variant

- `test_invariant_i2_terminal_after_failure`
  - Given: Any failure in phase execution
  - When: Processing completes
  - Then: Pipeline is in Failed or Escalated state

- `test_invariant_i3_cleanup_idempotent`
  - Given: Cleanup handler
  - When: Called multiple times
  - Then: Each call succeeds without error

- `test_invariant_i4_no_resource_leaks`
  - Given: Pipeline with cleanup
  - When: All cleanup completes
  - Then: No resources remain from failed phase

## Contract Violation Tests

- `test_p1_violation_terminal_pipeline_rejected`
  - Given: Pipeline in Accepted state
  - When: `run_pipeline(pipeline_id)` is called
  - Then: Returns `Err(PhaseError::InvalidStateTransition)`

- `test_p2_violation_pending_pipeline_requires_start`
  - Given: Pipeline in Pending state
  - When: Direct transition to Validation attempted
  - Then: Returns `Err(PhaseError::InvalidStateTransition)`

- `test_q1_violation_nonterminal_after_failure`
  - Given: Failed phase
  - When: State is checked after failure handling
  - Then: State is terminal (Failed or Escalated)

- `test_q2_violation_no_cleanup_on_failure`
  - Given: Failed phase with cleanup handler registered
  - When: Phase failure occurs
  - Then: Cleanup is invoked (can verify via mock/capture)

- `test_q3_violation_resources_remain_after_cleanup`
  - Given: Failed phase created resources
  - When: Cleanup runs
  - Then: Resources are released (verify via resource check)

## Given-When-Then Scenarios

### Scenario 1: Spec Review Failure
**Given**: Pipeline is created with path to invalid spec file  
**When**: `run_pipeline()` executes SpecReview phase  
**Then**: 
- SpecReview fails
- Cleanup handler is invoked
- Pipeline transitions to Failed state
- last_error contains failure message
- State is persisted

### Scenario 2: Universe Setup Failure with Rollback
**Given**: Pipeline in UniverseSetup that creates temp resources  
**When**: UniverseSetup fails mid-execution  
**Then**:
- Pipeline transitions to Escalated
- Rollback undoes resource creation
- Cleanup releases any remaining handles
- State persists with error details

### Scenario 3: Development Phase Failure Recovery
**Given**: Pipeline at AgentDevelopment with iteration in progress  
**When**: Agent development fails  
**Then**:
- Current iteration state is rolled back
- Cleanup releases spawned processes
- Pipeline transitions to Escalated
- Recovery can restart from AgentDevelopment state

### Scenario 4: Idempotent Cleanup
**Given**: Pipeline that already had cleanup run  
**When**: Cleanup is invoked again (e.g., recovery scenario)  
**Then**:
- Returns Ok without error
- No duplicate resource release attempts
- Maintains idempotent contract
