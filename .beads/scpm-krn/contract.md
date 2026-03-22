# Contract Specification: Parallel Phase Execution

## Context
- Feature: Orchestrator parallel phase execution
- Domain terms:
  - Phase: A distinct stage in the pipeline (SpecReview, UniverseSetup, AgentDevelopment, Validation)
  - Dependency: A prerequisite relationship between phases
  - Parallel execution: Running independent phases concurrently using async/await
- Assumptions:
  - Phases within the same pipeline can be parallelized if they have no data dependencies
  - AgentDevelopment phase supports parallel iterations when multiple agents are involved
  - tokio runtime is available for async operations
- Open questions:
  - What is the maximum degree of parallelism?
  - How do we handle shared resources during parallel execution?

## Preconditions
- P1: Pipeline must be in non-terminal state before execution
- P2: Pipeline state must be persisted before parallel phases launch
- P3: All phases in the ready set must have their dependencies satisfied

## Postconditions
- Q1: All phases complete execution (success or failure)
- Q2: Pipeline state is correctly updated after parallel execution
- Q3: Phase ordering constraints are respected (no phase completes before its dependencies)
- Q4: If any required phase fails, the pipeline transitions to appropriate error state
- Q5: Metrics are recorded for all executed phases

## Invariants
- I1: A phase cannot run before its dependencies complete
- I2: Terminal states (Accepted, Escalated, Failed) cannot transition further
- I3: Only one phase can hold exclusive access to shared state at a time

## Error Taxonomy
- `PhaseError::DependencyNotMet` - Phase dependencies not satisfied before execution
- `PhaseError::StateTransitionFailed` - Invalid state transition during parallel execution
- `PhaseError::ParallelExecutionFailed` - General parallel execution error
- `PhaseError::InvalidPhaseConfiguration` - Phase configuration is invalid

## Contract Signatures
```rust
// Phase dependency representation
fn resolve_parallel_phases(pipeline: &Pipeline) -> Result<Vec<PhaseGroup>, PhaseError>;
fn execute_phase_group(phases: PhaseGroup) -> Result<Vec<PhaseResult>, PhaseError>;
fn validate_dependency_order(phases: &[PhaseType]) -> Result<(), PhaseError>;
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| P1: non-terminal state | Compile-time | `PipelineState::is_terminal()` check |
| P2: persistence before launch | Runtime check | `Result<()>` before spawn |
| P3: dependencies satisfied | Runtime check | `DependencyGraph` validation |

## Violation Examples
- VIOLATES P1: `execute_pipeline(Pipeline { state: Accepted })` → `Err(PhaseError::InvalidStateTransition)`
- VIOLATES P2: `spawn_phase_without_persistence()` → `Err(PhaseError::ParallelExecutionFailed)`
- VIOLATES P3: `execute_phase(Validation, missing UniverseSetup)` → `Err(PhaseError::DependencyNotMet)`

## Ownership Contracts
- `Pipeline`: Shared reference during parallel execution, exclusive access for state transitions
- `PhaseGroup`: Owned by the parallel executor, spawned as async tasks
- `StateStore`: Thread-safe via internal synchronization, accessed by multiple workers

## Non-goals
- Implementing cross-pipeline parallelism (each pipeline is independent)
- Dynamic rebalancing of parallelism degree at runtime
- Automatic phase dependency inference (dependencies are explicit)
