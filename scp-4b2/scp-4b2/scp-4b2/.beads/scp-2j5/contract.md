# Contract Specification

## Context
- **Feature**: cli: Add switch and context commands
- **Description**: Add switch (workspace switching), context/whereami (location detection) commands
- **Domain terms**:
  - `switch` - Top-level command to switch between workspaces
  - `context` / `whereami` - Command to detect and display current location (workspace, branch, VCS state)
- **Assumptions**:
  - The VCS backend (jj/git) is already initialized in the current directory
  - The workspace to switch to already exists
  - Working copy must be clean before switching
- **Open questions**:
  - Should `context` show additional metadata beyond workspace/branch?

## Preconditions

- **P1**: For `switch`: Workspace name must be provided as a non-empty string
- **P2**: For `switch`: Target workspace must exist in the VCS backend
- **P3**: For `switch`: Working copy must be clean (no uncommitted changes)
- **P4**: For `context`: Command can run in any directory (no preconditions)

## Postconditions

- **Q1**: For successful `switch`: Current working directory's VCS workspace is changed to the target workspace
- **Q2**: For successful `switch`: Output confirms the switch was successful
- **Q3**: For successful `context`: Output shows current workspace name, branch name, and VCS status
- **Q4**: For failed `switch` (workspace not found): Error message suggests `scp workspace list`
- **Q5**: For failed `switch` (dirty working copy): Error message suggests committing or stashing

## Invariants

- **I1**: After `switch` completes (success or failure), the process exits with appropriate exit code
- **I2**: `context` always outputs human-readable format by default
- **I3**: Both commands handle VCS backend errors gracefully without panicking

## Error Taxonomy

- `Error::WorkspaceNotFound(String)` - When target workspace does not exist
- `Error::WorkingCopyDirty` - When uncommitted changes exist and switch is attempted
- `Error::VcsNotInitialized` - When VCS is not initialized in current directory
- `Error::VcsConflict(String, String)` - When VCS operation fails
- `Error::IoError(String)` - When filesystem operations fail

## Contract Signatures

```rust
// switch command
fn switch_to_workspace(name: &str) -> Result<(), Error>

// context/whereami command  
fn show_context() -> Result<(), Error>
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| workspace name non-empty | Runtime-checked | String with length > 0 check |
| workspace exists | Runtime-checked | Query VCS backend, return Error::WorkspaceNotFound |
| working copy clean | Runtime-checked | Check VCS status, return Error::WorkingCopyDirty |
| VCS initialized | Runtime-checked | Return Error::VcsNotInitialized if not |

## Violation Examples

- **VIOLATES P1**: `switch_to_workspace("")` -- should produce `Err(Error::InvalidIdentifier("workspace name cannot be empty".into()))`
- **VIOLATES P2**: `switch_to_workspace("nonexistent")` -- should produce `Err(Error::WorkspaceNotFound("nonexistent".into()))`
- **VIOLATES P3**: `switch_to_workspace("existing")` with dirty working copy -- should produce `Err(Error::WorkingCopyDirty)`

## Ownership Contracts

- Both functions take `&str` parameters (borrowed, no ownership transfer)
- No `&mut` parameters - no mutation contracts needed
- Functions may mutate stdout (printing output), but this is side-effect only

## Non-goals

- Creating new workspaces (handled by `scp workspace spawn`)
- Deleting or managing workspaces (handled by `scp workspace` subcommands)
- Queue management
- Agent/session management
