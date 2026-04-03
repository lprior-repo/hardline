# ADR-007: Error Taxonomy - Hierarchical Error Codes

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline needs a comprehensive error handling system that:

1. **Provides actionable errors** - Errors tell the user/agent what went wrong and how to fix it
2. **Enables automation** - Error codes can be programmatically handled
3. **Hierarchical categorization** - 1xxx, 2xxx, 3xxx ranges for different subsystems
4. **Retry guidance** - Distinguishes retryable vs terminal errors
5. **Fix suggestions** - Includes recovery commands when possible

The architecture spec defines error codes 1xxx-9xxx. This ADR formalizes the complete taxonomy.

---

## Decision

### Error Code Ranges

| Range | Category | Description |
|-------|----------|-------------|
| 1xxx | Workspace | Workspace creation, management, state |
| 2xxx | Session | Session lifecycle, bead claiming |
| 3xxx | Bead | Task/bead operations, dependencies |
| 4xxx | Queue | Queue management, priority, ordering |
| 5xxx | VCS | Git operations, conflicts |
| 6xxx | Stack | Stacked PRs, branch stacks |
| 7xxx | GitHub | GitHub API, PRs, CI status |
| 8xxx | Snapshot | Backup/restore, checkpoints |
| 9xxx | Internal | System errors, database, infrastructure |

### Base Error Trait

```rust
pub trait ScpError: std::error::Error + Send + Sync {
    fn code(&self) -> u16;
    fn category(&self) -> ErrorCategory;
    fn is_retryable(&self) -> bool;
    fn fix(&self) -> Option<ErrorFix>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Workspace,
    Session,
    Bead,
    Queue,
    Vcs,
    Stack,
    GitHub,
    Snapshot,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorFix {
    pub command: String,
    pub description: String,
    pub risk: FixRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixRisk {
    Safe,      // Read-only or easily reversible
    Moderate,  // Modifies state but recoverable
    Dangerous, // Potentially destructive
}
```

### Complete Error Enum

```rust
#[derive(Error, Debug)]
pub enum Error {
    // ═══════════════════════════════════════════════════════════════════════════
    // WORKSPACE ERRORS (1xxx)
    // ═══════════════════════════════════════════════════════════════════════════
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),
    
    #[error("Workspace already exists: {0}")]
    WorkspaceAlreadyExists(String),
    
    #[error("Workspace locked by agent {agent_id} until {locked_until}")]
    WorkspaceLocked { agent_id: String, locked_until: DateTime<Utc> },
    
    #[error("Workspace in invalid state: {state} for operation {operation}")]
    WorkspaceInvalidState { state: WorkspaceState, operation: String },
    
    #[error("Workspace path already exists: {0}")]
    WorkspacePathExists(PathBuf),
    
    #[error("Workspace path not writable: {0}")]
    WorkspacePathNotWritable(PathBuf),
    
    #[error("Workspace corrupted: {details}")]
    WorkspaceCorrupted { details: String },
    
    #[error("Workspace limit exceeded: {current}/{max}")]
    WorkspaceLimitExceeded { current: usize, max: usize },
    
    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION ERRORS (2xxx)
    // ═══════════════════════════════════════════════════════════════════════════
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    #[error("Session already exists: {0}")]
    SessionAlreadyExists(String),
    
    #[error("Session already active: {0}")]
    SessionAlreadyActive(String),
    
    #[error("Session expired: {0}")]
    SessionExpired(String),
    
    #[error("Session in invalid state: {state} for operation {operation}")]
    SessionInvalidState { state: SessionState, operation: String },
    
    #[error("Session bead conflict: bead {bead_id} already claimed")]
    SessionBeadConflict { bead_id: String },
    
    // ═══════════════════════════════════════════════════════════════════════════
    // BEAD ERRORS (3xxx)
    // ═══════════════════════════════════════════════════════════════════════════
    #[error("Bead not found: {0}")]
    BeadNotFound(String),
    
    #[error("Bead already claimed by {claimed_by}")]
    BeadAlreadyClaimed { claimed_by: String },
    
    #[error("Bead dependency cycle detected: {cycle}")]
    BeadDependencyCycle { cycle: Vec<String> },
    
    #[error("Bead in invalid state: {state} for operation {operation}")]
    BeadInvalidState { state: BeadState, operation: String },
    
    #[error("Bead has unresolved dependencies: {dependencies:?}")]
    BeadUnresolvedDependencies { dependencies: Vec<String> },
    
    #[error("Bead priority invalid: {priority} (must be 0-4)")]
    BeadInvalidPriority { priority: u8 },
    
    // ═══════════════════════════════════════════════════════════════════════════
    // QUEUE ERRORS (4xxx)
    // ═══════════════════════════════════════════════════════════════════════════
    #[error("Queue is full: {current}/{max}")]
    QueueFull { current: usize, max: usize },
    
    #[error("Queue entry not found: {0}")]
    QueueEntryNotFound(String),
    
    #[error("Queue priority conflict: {message}")]
    QueuePriorityConflict { message: String },
    
    #[error("Queue stale entry: {entry_id} (status: {status})")]
    QueueStaleEntry { entry_id: String, status: String },
    
    #[error("Queue transition invalid: {from} -> {to}")]
    QueueTransitionInvalid { from: QueueStatus, to: QueueStatus },
    
    #[error("Queue empty")]
    QueueEmpty,
    
    // ═══════════════════════════════════════════════════════════════════════════
    // VCS ERRORS (5xxx)
    // ═══════════════════════════════════════════════════════════════════════════
    #[error("VCS not initialized at {0}")]
    VcsNotInitialized(PathBuf),
    
    #[error("VCS not found: {0}")]
    VcsNotFound(String),
    
    #[error("VCS conflict: {message}")]
    VcsConflict { message: String },
    
    #[error("VCS detached HEAD state")]
    VcsDetachedHead,
    
    #[error("VCS branch not found: {0}")]
    VcsBranchNotFound(String),
    
    #[error("VCS branch already exists: {0}")]
    VcsBranchAlreadyExists(String),
    
    #[error("VCS invalid ref: {name} ({reason})")]
    VcsInvalidRef { name: String, reason: String },
    
    #[error("VCS merge conflict in {files:?}")]
    VcsMergeConflict { files: Vec<PathBuf> },
    
    #[error("VCS push failed: {0}")]
    VcsPushFailed(String),
    
    #[error("VCS pull failed: {0}")]
    VcsPullFailed(String),
    
    #[error("VCS not installed: {0}")]
    VcsNotInstalled(String),
    
    // ═══════════════════════════════════════════════════════════════════════════
    // STACK ERRORS (6xxx)
    // ═══════════════════════════════════════════════════════════════════════════
    #[error("Stack not found: {0}")]
    StackNotFound(String),
    
    #[error("Stack orphaned: parent {parent} not found")]
    StackOrphaned { parent: String },
    
    #[error("Stack cyclic dependency detected")]
    StackCyclicDependency,
    
    #[error("Stack in invalid state: {state}")]
    StackInvalidState { state: String },
    
    #[error("Stack PR not found: {0}")]
    StackPrNotFound(String),
    
    // ═══════════════════════════════════════════════════════════════════════════
    // GITHUB ERRORS (7xxx)
    // ═══════════════════════════════════════════════════════════════════════════
    #[error("GitHub authentication failed: {0}")]
    GitHubAuthFailed(String),
    
    #[error("GitHub token expired")]
    GitHubTokenExpired,
    
    #[error("GitHub rate limited: retry after {retry_after}")]
    GitHubRateLimited { retry_after: DateTime<Utc> },
    
    #[error("GitHub PR closed: {0}")]
    GitHubPrClosed(String),
    
    #[error("GitHub PR not found: {0}")]
    GitHubPrNotFound(String),
    
    #[error("GitHub API error: {status} - {message}")]
    GitHubApiError { status: u16, message: String },
    
    #[error("GitHub CI status check failed: {checks:?}")]
    GitHubCiFailed { checks: Vec<String> },
    
    // ═══════════════════════════════════════════════════════════════════════════
    // SNAPSHOT ERRORS (8xxx)
    // ═══════════════════════════════════════════════════════════════════════════
    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),
    
    #[error("Snapshot corrupted: {details}")]
    SnapshotCorrupted { details: String },
    
    #[error("Snapshot expired: {0}")]
    SnapshotExpired(String),
    
    #[error("Snapshot limit exceeded: {current}/{max}")]
    SnapshotLimitExceeded { current: usize, max: usize },
    
    #[error("Snapshot restore failed: {0}")]
    SnapshotRestoreFailed(String),
    
    // ═══════════════════════════════════════════════════════════════════════════
    // INTERNAL ERRORS (9xxx)
    // ═══════════════════════════════════════════════════════════════════════════
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Database corrupted: {details}")]
    DatabaseCorrupted { details: String },
    
    #[error("Unexpected null value in {context}")]
    UnexpectedNull { context: String },
    
    #[error("Configuration error: {key} ({reason})")]
    ConfigurationError { key: String, reason: String },
    
    #[error("IO error: {0}")]
    Io(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Timeout: {operation} exceeded {duration}")]
    Timeout { operation: String, duration: Duration },
    
    #[error("Not implemented: {0}")]
    Unimplemented(String),
}
```

### Error Code Mapping

```rust
impl Error {
    pub fn code(&self) -> u16 {
        match self {
            // Workspace (1xxx)
            Error::WorkspaceNotFound(_) => 1001,
            Error::WorkspaceAlreadyExists(_) => 1002,
            Error::WorkspaceLocked { .. } => 1003,
            Error::WorkspaceInvalidState { .. } => 1004,
            Error::WorkspacePathExists(_) => 1005,
            Error::WorkspacePathNotWritable(_) => 1006,
            Error::WorkspaceCorrupted { .. } => 1007,
            Error::WorkspaceLimitExceeded { .. } => 1008,
            
            // Session (2xxx)
            Error::SessionNotFound(_) => 2001,
            Error::SessionAlreadyExists(_) => 2002,
            Error::SessionAlreadyActive(_) => 2003,
            Error::SessionExpired(_) => 2004,
            Error::SessionInvalidState { .. } => 2005,
            Error::SessionBeadConflict { .. } => 2006,
            
            // Bead (3xxx)
            Error::BeadNotFound(_) => 3001,
            Error::BeadAlreadyClaimed { .. } => 3002,
            Error::BeadDependencyCycle { .. } => 3003,
            Error::BeadInvalidState { .. } => 3004,
            Error::BeadUnresolvedDependencies { .. } => 3005,
            Error::BeadInvalidPriority { .. } => 3006,
            
            // Queue (4xxx)
            Error::QueueFull { .. } => 4001,
            Error::QueueEntryNotFound(_) => 4002,
            Error::QueuePriorityConflict { .. } => 4003,
            Error::QueueStaleEntry { .. } => 4004,
            Error::QueueTransitionInvalid { .. } => 4005,
            Error::QueueEmpty => 4006,
            
            // VCS (5xxx)
            Error::VcsNotInitialized(_) => 5001,
            Error::VcsNotFound(_) => 5002,
            Error::VcsConflict { .. } => 5003,
            Error::VcsDetachedHead => 5004,
            Error::VcsBranchNotFound(_) => 5005,
            Error::VcsBranchAlreadyExists(_) => 5006,
            Error::VcsInvalidRef { .. } => 5007,
            Error::VcsMergeConflict { .. } => 5008,
            Error::VcsPushFailed(_) => 5009,
            Error::VcsPullFailed(_) => 5010,
            Error::VcsNotInstalled(_) => 5011,
            
            // Stack (6xxx)
            Error::StackNotFound(_) => 6001,
            Error::StackOrphaned { .. } => 6002,
            Error::StackCyclicDependency => 6003,
            Error::StackInvalidState { .. } => 6004,
            Error::StackPrNotFound(_) => 6005,
            
            // GitHub (7xxx)
            Error::GitHubAuthFailed(_) => 7001,
            Error::GitHubTokenExpired => 7002,
            Error::GitHubRateLimited { .. } => 7003,
            Error::GitHubPrClosed(_) => 7004,
            Error::GitHubPrNotFound(_) => 7005,
            Error::GitHubApiError { .. } => 7006,
            Error::GitHubCiFailed { .. } => 7007,
            
            // Snapshot (8xxx)
            Error::SnapshotNotFound(_) => 8001,
            Error::SnapshotCorrupted { .. } => 8002,
            Error::SnapshotExpired(_) => 8003,
            Error::SnapshotLimitExceeded { .. } => 8004,
            Error::SnapshotRestoreFailed(_) => 8005,
            
            // Internal (9xxx)
            Error::Internal(_) => 9001,
            Error::Database(_) => 9002,
            Error::DatabaseCorrupted { .. } => 9003,
            Error::UnexpectedNull { .. } => 9004,
            Error::ConfigurationError { .. } => 9005,
            Error::Io(_) => 9006,
            Error::Serialization(_) => 9007,
            Error::Timeout { .. } => 9008,
            Error::Unimplemented(_) => 9009,
        }
    }
    
    pub fn category(&self) -> ErrorCategory {
        match self {
            Error::WorkspaceNotFound(_) ..= Error::WorkspaceLimitExceeded { .. } => ErrorCategory::Workspace,
            Error::SessionNotFound(_) ..= Error::SessionBeadConflict { .. } => ErrorCategory::Session,
            Error::BeadNotFound(_) ..= Error::BeadInvalidPriority { .. } => ErrorCategory::Bead,
            Error::QueueFull { .. } ..= Error::QueueEmpty => ErrorCategory::Queue,
            Error::VcsNotInitialized(_) ..= Error::VcsNotInstalled(_) => ErrorCategory::Vcs,
            Error::StackNotFound(_) ..= Error::StackPrNotFound(_) => ErrorCategory::Stack,
            Error::GitHubAuthFailed(_) ..= Error::GitHubCiFailed { .. } => ErrorCategory::GitHub,
            Error::SnapshotNotFound(_) ..= Error::SnapshotRestoreFailed(_) => ErrorCategory::Snapshot,
            Error::Internal(_) ..= Error::Unimplemented(_) => ErrorCategory::Internal,
        }
    }
    
    pub fn is_retryable(&self) -> bool {
        match self {
            // Retryable
            Error::VcsPushFailed(_) => true,
            Error::VcsPullFailed(_) => true,
            Error::GitHubRateLimited { .. } => true,
            Error::GitHubApiError { status: 502.., .. } => true,
            Error::Timeout { .. } => true,
            
            // Not retryable
            Error::WorkspaceNotFound(_) => false,
            Error::WorkspaceAlreadyExists(_) => false,
            Error::WorkspaceInvalidState { .. } => false,
            Error::SessionExpired(_) => false,
            Error::BeadAlreadyClaimed { .. } => false,
            Error::QueueEmpty => false,
            _ => false,
        }
    }
}
```

---

## Variants

### Variant A: Flat Error Enum (REJECTED)

```rust
enum Error {
    WorkspaceNotFound,
    WorkspaceAlreadyExists,
    // ... 100+ variants
}
```

**Rejected because:**
- No hierarchical organization
- Hard to group errors by category
- Doesn't match architecture spec's 1xxx-9xxx ranges

### Variant B: Trait Objects (REJECTED)

```rust
trait Error: std::error::Error {
    fn code(&self) -> u16;
    fn category(&self) -> ErrorCategory;
}
```

**Rejected because:**
- No exhaustive matching
- Hard to serialize/deserialize
- Slower dispatch

### Variant C: Flat Enum with Code Ranges (CHOSEN)

**Chosen because:**
- Exhaustive matching (compiler enforced)
- Matches architecture spec
- Easy to serialize
- Fast dispatch

---

## Invariants

### Error Code Invariants

```rust
/// INVARIANT: Error code ranges match category
fn assert_code_range(code: u16, category: ErrorCategory) {
    match category {
        ErrorCategory::Workspace => assert!(1000 <= code && code < 2000),
        ErrorCategory::Session => assert!(2000 <= code && code < 3000),
        ErrorCategory::Bead => assert!(3000 <= code && code < 4000),
        ErrorCategory::Queue => assert!(4000 <= code && code < 5000),
        ErrorCategory::Vcs => assert!(5000 <= code && code < 6000),
        ErrorCategory::Stack => assert!(6000 <= code && code < 7000),
        ErrorCategory::GitHub => assert!(7000 <= code && code < 8000),
        ErrorCategory::Snapshot => assert!(8000 <= code && code < 9000),
        ErrorCategory::Internal => assert!(9000 <= code && code < 10000),
    }
}

/// INVARIANT: Every error variant has a unique code
// Compiler enforces this with exhaustive match
```

### Retry Invariants

```rust
/// INVARIANT: Terminal errors are never retryable
fn assert_terminal_not_retryable(error: &Error) {
    if matches!(error, Error::WorkspaceLimitExceeded { .. }) {
        assert!(!error.is_retryable());
    }
}

/// INVARIANT: Network errors are retryable (with backoff)
fn assert_network_retryable(error: &Error) {
    if matches!(error, Error::VcsPullFailed(_) | Error::GitHubRateLimited { .. }) {
        assert!(error.is_retryable());
    }
}
```

### Fix Invariants

```rust
/// INVARIANT: Fix commands are non-empty for recoverable errors
fn assert_fix_available(error: &Error) {
    match error {
        Error::WorkspaceNotFound(name) => {
            let fix = error.fix();
            assert!(fix.is_some());
            assert!(!fix.unwrap().command.is_empty());
        }
        _ => {}  // Other errors may or may not have fixes
    }
}

/// INVARIANT: Dangerous fixes are never marked Safe
fn assert_fix_risk_matches_command(fix: &ErrorFix) {
    if fix.command.contains("delete") || fix.command.contains("drop") {
        assert!(matches!(fix.risk, FixRisk::Dangerous | FixRisk::Moderate));
    }
}
```

---

## Consequences

### Positive

1. **Actionable** - Every error includes context and fix suggestion
2. **Programmable** - Error codes enable automated handling
3. **Hierarchical** - Easy to filter by category
4. **Retry guidance** - Distinguishes retryable vs terminal
5. **Matches spec** - Aligns with architecture-spec.md

### Negative

1. **Large enum** - 60+ variants, but manageable
2. **Code churn** - Adding new errors requires updating code() method

### JSON Error Format

```json
{
  "error": {
    "code": 1001,
    "category": "workspace",
    "message": "Workspace not found: agent-123",
    "retryable": false,
    "fix": {
      "command": "hardline workspace list",
      "description": "List available workspaces",
      "risk": "safe"
    }
  }
}
```

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/scp-error/src/lib.rs` | Complete Error enum |
| `crates/core/src/error.rs` | Error construction helpers |
| `crates/cli/src/commands/` | Error handling in commands |

---

## Related ADRs

- ADR-001: CLI Architecture (error output format)
- ADR-002: Durable Workflow Execution (retry handling)
- ADR-006: Database Schema (DatabaseError variants)
