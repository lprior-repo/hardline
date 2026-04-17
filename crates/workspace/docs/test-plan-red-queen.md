# Red Queen Test Plan — scp-workspace

## Verdict: CROWN DEFENDED

**Champion**: scp-workspace v0.5.0
**Generations**: 3
**Lineage**: 3 done_when checks (base: test, clippy, fmt)
**Survivors**: 0
**Crown**: DEFENDED (equilibrium reached after 3 consecutive zero-survivor generations)

## Fitness Landscape

| Dimension | Tests | Survivors | Fitness | Status |
|-----------|-------|-----------|---------|--------|
| state-inconsistency | 1 | 0 | 0 | EXHAUSTED |
| typestate-bypass | 1 | 0 | 0 | EXHAUSTED |
| value-object-gaps | 1 | 0 | 0 | EXHAUSTED |
| invariant-violation | 1 | 0 | 0 | EXHAUSTED |
| concurrency | 1 | 0 | 0 | EXHAUSTED |
| edge-case-transitions | 1 | 0 | 0 | EXHAUSTED |
| service-filter-correctness | 1 | 0 | 0 | EXHAUSTED |
| mutation-style | 1 | 0 | 0 | DORMANT |
| boundary | 1 | 0 | 0 | DORMANT |
| serialization | 1 | 0 | 0 | DORMANT |
| service-edge | 1 | 0 | 0 | DORMANT |
| repository-stress | 1 | 0 | 0 | DORMANT |
| proptest-invariants | 1 | 0 | 0 | COOLING |
| stress-concurrent | 1 | 0 | 0 | COOLING |
| edge-semantics | 1 | 0 | 0 | COOLING |

## Observations (not bugs, but design debt)

### State Machine vs Service Inconsistency
- `WorkspaceStateMachine` allows `Locked→Deleted` but `WorkspaceService::delete_workspace` rejects it
- `WorkspaceStateMachine` allows `Corrupted→Deleted` and entity supports it, but `WorkspaceService` rejects it
- This is intentional service-level policy (protect locked workspaces) but creates a **semantic gap** between the state machine's truth and the service's enforcement

### Typestate Bypass in WorkspaceService
- `WorkspaceService` methods manually construct `Workspace` structs with `PhantomData::<Initializing>` regardless of actual runtime state
- This defeats the compile-time typestate guarantees of the entity layer
- The service layer effectively operates on untyped `Workspace` while the entity layer has rich typestate

### Value Object Validation Gaps
- `BranchName` only rejects empty and null — newlines, tabs, control chars pass through
- `LockHolder` only rejects empty — whitespace-only, newlines allowed
- `WorkspaceId::parse` only rejects empty — path traversal strings, XSS-like content accepted
- `WorkspaceName` allows separator-only names (`---`, `___`)

## Test Coverage Summary

### Generation 1 (22 tests) — Structural Adversarial
- State machine vs service consistency checks
- Typestate bypass verification
- Value object validation gap documentation
- Invariant preservation through transitions
- Concurrent thread safety (8 threads)
- Full lifecycle path coverage

### Generation 2 (30 tests) — Mutation + Boundary
- Exhaustive pairwise state transition validation
- Terminal/lockable mutation resistance
- Triple-boundary length checks (254/255/256/257)
- Invalid character enumeration
- Null character position sweep
- JSON serialization roundtrips for all types
- Service edge case coverage

### Generation 3 (19 tests) — Property + Stress
- Reflexivity/symmetry/transitivity properties
- 1000-unique ID generation test
- 100-thread concurrent save stress test
- 50-thread concurrent read/write/delete stress test
- 1000-cycle lock/unlock stress test
- Timestamp monotonicity verification
- Identity preservation through all transitions
- Send + Sync + 'static trait bounds
