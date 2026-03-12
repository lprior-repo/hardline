# Contract Specification

## Context
- **Feature**: Add spawn, done, abort workflow commands from isolate
- **Bead ID**: scp-pmw
- **Domain terms**:
  - Workspace: An isolated development environment (Jujutsu workspace)
  - Spawn: Create a new isolated workspace
  - Done: Complete workspace and merge changes to main
  - Abort: Abandon workspace without merging changes
- **Assumptions**:
  - VCS backend is Jujutsu (jj) - checked at runtime
  - Current directory is a valid jj repository
  - Commands are invoked from the CLI
- **Open questions**:
  - Should spawn create a bead automatically?
  - Should done/abort update bead status?

## Preconditions
- [P1] Workspace name must be valid identifier (non-empty, alphanumeric + dash/underscore)
- [P2] Workspace name must not already exist (for spawn)
- [P3] Workspace must exist (for done, abort)
- [P4] Working copy must be clean before switch/done/abort operations
- [P5] Target workspace must not be "main" for abort operation

## Postconditions
- [Q1] After spawn: New workspace exists in VCS backend
- [Q2] After spawn with sync: Workspace is switched to and rebased on main
- [Q3] After done: Working copy is rebased on main and pushed to remote
- [Q4] After abort: Workspace is deleted from VCS backend

## Invariants
- [I1] No workspace operations should leave repository in inconsistent state
- [I2] All VCS operations must complete atomically or rollback

## Error Taxonomy
- `Error::WorkspaceExists(name)` - when workspace name already exists
- `Error::WorkspaceNotFound(name)` - when workspace does not exist
- `Error::WorkingCopyDirty` - when uncommitted changes exist
- `Error::VcsConflict(op, msg)` - when VCS operation fails
- `Error::VcsPushFailed(msg)` - when push to remote fails
- `Error::VcsRebaseFailed(msg)` - when rebase onto main fails
- `Error::InvalidIdentifier(name)` - when workspace name is invalid

## Contract Signatures
```rust
// Spawn: Create new workspace
fn spawn(name: &str, sync: bool) -> Result<()>;

// Done: Complete workspace and merge to main  
fn done(name: Option<&str>) -> Result<()>;

// Abort: Abandon workspace without merge
fn abort(name: Option<&str>) -> Result<()>;
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| name non-empty | Runtime-checked constructor | ValidateString::new() |
| name valid identifier | Runtime regex | `^[a-zA-Z][a-zA-Z0-9_-]*$` |
| workspace not exists | Result error variant | `Result<(), Error::WorkspaceExists>` |
| workspace exists | Result error variant | `Result<(), Error::WorkspaceNotFound>` |
| working copy clean | Runtime check | `Result<(), Error::WorkingCopyDirty>` |
| not aborting main | Runtime check | `Result<(), Error::InvalidOperation>` |

## Violation Examples
- VIOLATES P1: `spawn("", false)` -- should produce `Err(Error::InvalidIdentifier(""))`
- VIOLATES P1: `spawn("123invalid", false)` -- should produce `Err(Error::InvalidIdentifier("123invalid"))`
- VIOLATES P2: `spawn("existing-workspace", false)` -- should produce `Err(Error::WorkspaceExists("existing-workspace"))`
- VIOLATES P3: `done(Some("nonexistent"))` -- should produce `Err(Error::WorkspaceNotFound("nonexistent"))`
- VIOLATES P3: `abort(Some("nonexistent"))` -- should produce `Err(Error::WorkspaceNotFound("nonexistent"))`
- VIOLATES P4: `switch("workspace")` with dirty working copy -- should produce `Err(Error::WorkingCopyDirty)`
- VIOLATES P5: `abort(Some("main"))` -- should produce `Err(Error::InvalidOperation("cannot abort main"))`

## Ownership Contracts
- All functions take string slices (`&str`) - no ownership transfer
- No mutable borrows - functions are pure commands
- Clone decisions: Backend is created fresh per invocation (intentional - stateless)

## Non-goals
- [ ] Auto-create beads on spawn
- [ ] Auto-update bead status on done/abort
- [ ] Remote workspace operations
- [ ] Workspace templates
