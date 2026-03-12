# Implementation Summary

## Bead: scp-2j5
## Title: cli: Add switch and context commands

### Changes Made

#### 1. Top-level `switch` command (main.rs)
Added `Switch` command to top-level Commands enum and wired it to call `commands::workspace::switch()`.

#### 2. Top-level `context` command (main.rs)
Added `Context` command to top-level Commands enum and wired it to call `commands::context::run()`.

#### 3. Top-level `whereami` command (main.rs)
Added `Whereami` command as an alias for `context`.

### Implementation Details

The actual logic for these commands already exists in:
- `crates/cli/src/commands/workspace.rs::switch()` - Validates workspace name, checks workspace exists, checks working copy is clean, switches workspace
- `crates/cli/src/commands/context.rs::run()` - Shows current workspace, branch, VCS status
- `crates/cli/src/commands/context.rs::whereami()` - Alias for run()

### Error Types Used

All contract error types were already available:
- `Error::WorkspaceNotFound(String)` - P2 enforcement
- `Error::WorkingCopyDirty` - P3 enforcement  
- `Error::VcsNotInitialized` - P4 enforcement (in context)
- `Error::InvalidIdentifier(String)` - P1 enforcement

### Preconditions Met

| Precondition | Implementation |
|---|---|
| P1: workspace name non-empty | `workspace.rs::switch` validates `name.is_empty()` |
| P2: workspace exists | `workspace.rs::switch` checks via `backend.list_workspaces()` |
| P3: working copy clean | `workspace.rs::switch` checks via `backend.status()` |
| P4: context runs anywhere | `context.rs::run` handles missing VCS gracefully |

### Postconditions Met

| Postcondition | Implementation |
|---|---|
| Q1: workspace changed | `backend.switch_workspace(name)` |
| Q2: success output | `Output::success(format!("Switched to '{}'", name))` |
| Q3: context shows workspace/branch/status | `context.rs::run` outputs all three |
| Q4: error message on not found | `Error::WorkspaceNotFound` with suggestion |
| Q5: error message on dirty | `Error::WorkingCopyDirty` with suggestion |

### Invariants Met

| Invariant | Implementation |
|---|---|
| I1: exit with appropriate code | All errors use proper Error types with exit_code() |
| I2: human-readable output | Output::info used throughout |
| I3: graceful error handling | No panics, all errors properly handled |
