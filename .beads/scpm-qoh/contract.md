# Contract Specification: JJ Backend (scpm-qoh)

## Context
- **Feature**: JJ (Jujutsu) VCS Backend Implementation
- **Domain terms**: VcsBackend trait, JjBackend struct, VcsError, Commit, Branch, Workspace, VcsStatus
- **Assumptions**: JJ CLI is installed and available in PATH; target path is a valid jj repository
- **Open questions**: None

## Preconditions
- JjBackend::new() requires a valid PathBuf pointing to a jj repository
- All VCS operations require the JJ CLI to be installed and executable
- Workspace operations require the workspace to exist (for switch/delete/merge)

## Postconditions
- All `run_jj()` calls return `Result<std::process::Output, VcsError>`
- All trait methods return `Result<T, VcsError>` - never panic
- Successful operations produce `Ok(value)` with correctly parsed domain types
- Failed operations produce `Err(VcsError::Variant(...))` with meaningful context

## Invariants
- JjBackend.repo_path is never mutated after construction
- No unwrap/expect/panic in source code (tests exempt)
- All CLI failures are captured as typed VcsError variants

## Error Taxonomy
| Variant | Trigger | Context |
|---|---|---|
| `VcsError::NotInitialized` | No .jj directory found | is_initialized() returns false |
| `VcsError::Conflict(op, msg)` | JJ operation failed with conflict | Rebase, merge, workspace add conflicts |
| `VcsError::PushFailed(msg)` | `jj git push` failed | Network or auth issues |
| `VcsError::PullFailed(msg)` | `jj git fetch` failed | Network or remote issues |
| `VcsError::RebaseFailed(msg)` | `jj rebase` failed | Target doesn't exist, conflict |
| `VcsError::BranchExists(name)` | `jj bookmark create` already exists | Duplicate branch |
| `VcsError::BranchNotFound(name)` | `jj bookmark set` target missing | Invalid branch name |
| `VcsError::WorkspaceExists(name)` | `jj workspace add` already exists | Duplicate workspace |
| `VcsError::WorkspaceNotFound(name)` | `jj workspace delete/root` missing | Invalid workspace |
| `VcsError::Io(err)` | std::io::Error from Command::output() | JJ binary not found, permission denied |
| `VcsError::Unimplemented(msg)` | Fallback for unsupported operations | Never raised in JJ backend |

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| JJ CLI installed | Runtime | `Command::new("jj")` will fail with IoError if missing |
| Valid repository path | Runtime | `run_jj()` will fail if not a jj repo |
| Non-empty branch name | Runtime | `jj bookmark create ""` fails with validation |
| Non-empty workspace name | Runtime | `jj workspace add ""` fails with validation |
| Valid utf8 output | Runtime | `String::from_utf8_lossy()` handles invalid utf8 |

## Violation Examples
- VIOLATES <P1>: `JjBackend::new(PathBuf::from("/nonexistent")).is_initialized()` → returns `Ok(false)` (not an error)
- VIOLATES <P2>: `run_jj(&["invalid-command"])` → returns `Err(VcsError::Io(...))` because jj exits with error
- VIOLATES <P3>: `jj bookmark create "existing"` when bookmark exists → returns `Err(VcsError::BranchExists("existing"))`

## Ownership Contracts
- `JjBackend::new(repo_path: PathBuf)` - takes ownership of PathBuf, stores internally
- `run_jj(&[&str])` - borrows slice, creates new Command each time (no mutable state)
- All trait methods borrow `&self` - no mutation of backend state
- Output parsing clones data into domain entities (no lifetime issues)

## Non-goals
- [ ] Implementing git operations (handled by GitBackend)
- [ ] Async operations (current implementation is sync)
- [ ] Transaction support (jj handles this internally)
