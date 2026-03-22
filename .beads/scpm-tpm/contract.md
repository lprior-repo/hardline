---
bead_id: scpm-tpm
title: "cli: implement abort command"
type: feature
phase: CONTRACT_SPEC
updated_at: "2026-03-21T02:45:00Z"
---

# Contract Specification: cli abort command

## Context
- **Feature**: Implement `abort` command for workspace termination
- **Domain terms**: Workspace, WorkspaceState, Abandoned, Merged, abort
- **Assumptions**: 
  - Workspaces are managed by JJ (Jujutsu) VCS backend
  - Abort is distinct from merge - aborting destroys workspace, merging integrates changes
  - Main branch is protected and cannot be aborted
- **Open questions**: None

## EARS Requirements

### Ubiquitous Requirements
- THE SYSTEM SHALL treat aborted workspaces as disposable, prioritizing a clean main branch.

### Event-Driven Requirements  
- WHEN `abort` is executed on a workspace, THE SYSTEM SHALL transition the WorkspaceState to 'Abandoned' and physically destroy the workspace directory.

### Unwanted Requirements (Negative Specifications)
- IF a workspace is already in a 'Merged' state, THE SYSTEM SHALL NOT allow an abort operation
- BECAUSE: merged changes are permanent and cannot be aborted, they must be reverted instead.

## Preconditions

| ID | Precondition | Enforcement Level | Type/Pattern |
|----|--------------|-------------------|--------------|
| P1 | WorkspaceName must exist | Runtime-checked | `Result<Workspace, Error::WorkspaceNotFound>` |
| P2 | WorkspaceState must not be 'Merged' | Runtime-checked | `Error::InvalidOperation` variant |
| P3 | Cannot abort 'main' workspace | Runtime-checked | `Error::InvalidOperation("cannot abort main")` |
| P4 | Working copy must be clean | Runtime-checked | `Error::WorkingCopyDirty` |

## Postconditions

| ID | Postcondition | Verification |
|----|--------------|-------------|
| Q1 | WorkspaceState transitions to 'Abandoned' | Event `WorkspaceAborted` emitted |
| Q2 | Physical workspace directory is removed | `jj workspace delete` called |
| Q3 | Main branch remains unaffected | No changes to main branch |

## Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| I1 | No file changes from aborted workspace leak into main | Workspace deletion is atomic with state transition |
| I2 | Only one workspace can be active at a time per session | Session holds exclusive lock |

## Error Taxonomy

| Error | When Raised | Exit Code |
|-------|------------|-----------|
| `Error::WorkspaceNotFound(name)` | Workspace does not exist | 10 |
| `Error::InvalidOperation(msg)` | Cannot abort (merged/main/dirty) | 96 |
| `Error::WorkingCopyDirty` | Uncommitted changes present | 38 |

## Contract Signatures

```rust
/// Abort a workspace - destroy all changes and remove workspace
pub fn abort(name: Option<&str>) -> Result<()>

/// Validate workspace can be aborted (preconditions)
fn validate_abort_preconditions(workspace: &Workspace) -> Result<()>

/// Execute workspace abort (postconditions)
fn execute_workspace_abort(backend: &dyn VcsBackend, name: &str) -> Result<()>
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| Workspace exists | Runtime | `workspace_exists()` returns `Result<bool>` |
| Workspace not merged | Runtime | State check before abort |
| Not main workspace | Runtime | String comparison |
| Working copy clean | Runtime | `backend.status() == VcsStatus::Clean` |

## Violation Examples (REQUIRED)

### Precondition Violations

- **VIOLATES P1**: `abort("nonexistent")` on workspace that doesn't exist
  - Expected: `Err(Error::WorkspaceNotFound("nonexistent"))`
  
- **VIOLATES P2**: `abort()` on workspace already in 'Merged' state
  - Expected: `Err(Error::InvalidOperation("cannot abort merged workspace"))`
  
- **VIOLATES P3**: `abort("main")` attempting to abort main workspace
  - Expected: `Err(Error::InvalidOperation("cannot abort the main workspace"))`
  
- **VIOLATES P4**: `abort()` with dirty working copy
  - Expected: `Err(Error::WorkingCopyDirty)`

### Postcondition Violations

- **VIOLATES Q1**: After successful abort, `WorkspaceAborted` event not emitted
  - This would indicate state tracking failure

- **VIOLATES Q2**: After successful abort, workspace directory still exists
  - This would indicate `jj workspace delete` failure

- **VIOLATES Q3**: Main branch has changes after abort
  - This would indicate improper isolation

## Ownership Contracts

- **No ownership transfer**: All functions borrow references
- **Exclusive borrow**: `&mut VcsBackend` for operations that modify VCS state
- **No clone policy needed**: Operations don't transfer ownership

## Non-goals
- Aborting multiple workspaces at once (single workspace only)
- Preserving workspace files (--keep-workspace is out of scope for this bead)
- Undo of abort operation (not reversible by design)
