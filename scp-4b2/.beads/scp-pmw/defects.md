# Black Hat Review Defects: scp-pmw

## Phase 1: Contract Parity Violations

### Defect 1: spawn - Missing P1 validation
**Location**: `crates/cli/src/commands/workspace.rs:12-37`
**Contract**: P1 - Workspace name must be valid identifier (non-empty, alphanumeric + dash/underscore)
**Current**: No validation of workspace name
**Expected**: Should validate name format and return `Error::InvalidIdentifier` for invalid names

### Defect 2: done - Missing P3 validation
**Location**: `crates/cli/src/commands/workspace.rs:124-142`
**Contract**: P3 - Workspace must exist for done operation
**Current**: No check if workspace exists before rebase/push
**Expected**: Should check workspace exists and return `Error::WorkspaceNotFound`

### Defect 3: abort - Missing P5 validation
**Location**: `crates/cli/src/commands/workspace.rs:145-164`
**Contract**: P5 - Target workspace must not be "main" for abort
**Current**: No check to prevent aborting "main" workspace
**Expected**: Should check if name is "main" and return `Error::InvalidOperation`

## Phase 2-5: No Additional Issues
- Functions are appropriately sized (<25 lines)
- No functional purity violations
- Code is simple and straightforward
