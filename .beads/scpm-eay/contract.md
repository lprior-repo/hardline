# Contract Specification: Git CLI Backend

## Context
- **Feature**: Git CLI backend implementation for VCS abstraction
- **Bead ID**: scpm-eay
- **Domain Terms**:
  - `GitCliBackend` - Executes git CLI commands via `std::process::Command`
  - `VcsBackend` - Trait for version control operations
  - `VcsStatus` - Repository state (Clean, Dirty, Conflicted, Detached)
  - `Commit` - Git commit with id, message, author, timestamp, parents
  - `Branch` - Git branch with name, is_current, tracking
- **Assumptions**:
  - Git CLI is available in PATH
  - Operations are performed in a valid git repository
  - All git commands are run with LC_ALL=C for consistent output parsing
- **Open Questions**: None

## Preconditions
- `GitCliBackend::new()` requires a valid path (directory exists or will be created)
- `status()` requires the repository to be initialized (`.git` exists)
- `log()` requires the repository to have at least one commit
- `diff()` requires the repository to be initialized

## Postconditions
- `GitCliBackend::new()` returns a `GitCliBackend` instance with the given path
- `status()` returns `VcsStatus::Clean` when no changes, `VcsStatus::Dirty` when changes exist
- `log(n)` returns exactly `n` commits (or fewer if repository has fewer commits)
- `diff()` returns empty string when no changes, diff output when changes exist
- All operations return `Err(VcsError)` on failure, never panic

## Invariants
- `GitCliBackend.repo_path` is always a valid `PathBuf`
- Repository path always exists after construction (caller responsibility)
- Output parsing is deterministic given same git version

## Error Taxonomy
- `VcsError::NotInitialized` - `.git` directory not found
- `VcsError::BranchNotFound(name)` - Referenced branch does not exist
- `VcsError::BranchExists(name)` - Branch with given name already exists
- `VcsError::Io(e)` - I/O error from git command execution
- `VcsError::ParseError(msg)` - Failed to parse git output
- `VcsError::GitNotInstalled` - Git CLI not found in PATH
- `VcsError::Unimplemented(msg)` - Feature not implemented

## Contract Signatures
```rust
pub struct GitCliBackend { /* ... */ }

impl GitCliBackend {
    pub fn new(repo_path: PathBuf) -> Self;
    pub fn status(&self) -> Result<VcsStatus>;
    pub fn log(&self, limit: usize) -> Result<Vec<Commit>>;
    pub fn diff(&self) -> Result<String>;
    pub fn current_branch(&self) -> Result<Option<String>>;
    pub fn list_branches(&self) -> Result<Vec<Branch>>;
}

impl VcsBackend for GitCliBackend {
    // ... trait methods
}
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| repo_path valid | Runtime | `PathBuf` |
| .git exists for status/log/diff | Runtime check | `Result<VcsError::NotInitialized>` |
| limit is valid usize | Compile-time | `usize` (always valid) |
| Output parsing | Runtime | `Result<VcsError::ParseError>` |

## Violation Examples (REQUIRED)
- VIOLATES P1: `GitCliBackend::new(PathBuf::from("/nonexistent"))` -- creates instance but subsequent ops fail with `Err(VcsError::NotInitialized)`
- VIOLATES P2: `status()` on non-git directory -- returns `Err(VcsError::NotInitialized)`
- VIOLATES P3: `log(0)` -- returns empty vector, not an error (0 is valid)
- VIOLATES Q1: `status()` on dirty repo -- returns `VcsStatus::Dirty`
- VIOLATES Q2: `log(1)` on repo with 1 commit -- returns vector with exactly 1 commit

## Ownership Contracts
- `GitCliBackend` owns `repo_path: PathBuf` - no mutation after construction
- No `&mut self` in any method - all operations are immutable
- Clone not implemented - use `new()` to create new instances

## Non-goals
- Network operations (fetch, push, pull) - handled by separate backend
- Complex diff output parsing - returns raw diff string
- Interactive git operations (merge conflicts resolution)
