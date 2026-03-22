# Contract: scpm-551 - Railway-Oriented Git Error

**bead_id:** scpm-551
**bead_title:** gix: implement railway-oriented git error
**phase:** contract
**updated_at:** 2026-03-20T22:30:13Z

## 1. Contract Summary

- THE SYSTEM SHALL implement railway-oriented programming for git operations
- All git operations SHALL return `Result<T, GitError>`
- Errors SHALL be descriptive and actionable

## 2. Preconditions

### System State
- The `thiserror` crate is available in project dependencies
- The legacy `VcsError` type exists for backward compatibility

### Required Inputs
- None for error type definitions

## 3. Postconditions

### State Changes
- `GitError` enum is defined with variants for: `NotFound`, `InvalidRef`, `Conflict`, `Unauthorized`, `Network`, `Io`, `Gix`, `GixDiscover`, `GixInit`, `GixStatus`, `GixStatusIter`
- `GitResult<T>` alias is defined using `GitError`
- `From<GitError>` is implemented for `VcsError`

### Return Guarantees
- All fallible gix operations return `GitResult<T>`
- Error messages contain actionable context

## 4. Invariants

- No error variant uses `String` for paths where `PathBuf` is appropriate
- The `Gix` variant transparently wraps `gix::Error` via `#[from]`
- No `unwrap()`, `expect()`, or `panic!()` in source code

## 5. Error Taxonomy

```rust
pub enum GitError {
    NotFound(PathBuf),           // Repository not found
    InvalidRef { name, reason }, // Invalid reference
    Conflict { message, conflicted_files }, // Merge conflict
    Unauthorized(String),       // Auth failure
    Network(String),             // Network issues
    Io(std::io::Error),         // IO errors with #[from]
    Gix(gix::Error),            // gitoxide errors with #[from]
    GixDiscover(gix::discover::Error), // with #[from]
    GixInit(gix::init::Error),          // with #[from]
    GixStatus(gix::status::Error),      // with #[from]
    GixStatusIter(gix::status::into_iter::Error), // with #[from]
}
```

## 6. Conversion to VcsError

```rust
impl From<GitError> for VcsError {
    fn from(err: GitError) -> Self {
        match err {
            GitError::NotFound(_) => VcsError::NotInitialized,
            GitError::InvalidRef { name, .. } => VcsError::BranchNotFound(name),
            GitError::Conflict { message, .. } => VcsError::Conflict(message, String::new()),
            GitError::Unauthorized(msg) => VcsError::PushFailed(msg),
            GitError::Network(msg) => VcsError::PullFailed(msg),
            GitError::Io(io_err) => VcsError::Io(io_err),
            GitError::Gix(gix_err) => VcsError::Unimplemented(gix_err.to_string()),
            // ... other variants
        }
    }
}
```
