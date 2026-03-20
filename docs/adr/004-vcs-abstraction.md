# ADR-004: VCS Abstraction - Unified Version Control Backend

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline requires unified access to both Git and Jujutsu (JJ) version control systems. AI agents and CLI commands need consistent behavior regardless of which VCS backs the repository. The system must support:

1. **Git repositories** - using gitoxide (gix) for pure Rust implementation
2. **JJ repositories** - using jj-lib for native JJ operations
3. **Unified interface** - same API regardless of backend
4. **No shell out** - all VCS operations via Rust libraries, not CLI spawning

The architecture spec defines a `trait VcsBackend` but implementations are incomplete. This ADR formalizes the complete abstraction.

---

## Decision

### Core Trait Design

```rust
pub trait VcsBackend: Send + Sync {
    // Repository operations
    fn is_initialized(&self, path: &Path) -> bool;
    fn init(&self, path: &Path) -> Result<()>;
    fn open(&self, path: &Path) -> Result<Self::Repository>
    where Self: Sized;

    // Branch operations
    fn current_branch(&self, repo: &Self::Repository) -> Result<Option<BranchName>>;
    fn list_branches(&self, repo: &Self::Repository) -> Result<Vec<Branch>>;
    fn create_branch(&self, repo: &Self::Repository, name: &BranchName) -> Result<()>;
    fn delete_branch(&self, repo: &Self::Repository, name: &BranchName) -> Result<()>;
    fn switch_branch(&self, repo: &Self::Repository, name: &BranchName) -> Result<()>;

    // Commit operations
    fn status(&self, repo: &Self::Repository) -> Result<VcsStatus>;
    fn add(&self, repo: &Self::Repository, paths: &[&Path]) -> Result<()>;
    fn commit(&self, repo: &Self::Repository, message: &str) -> Result<CommitHash>;
    fn log(&self, repo: &Self::Repository, count: usize) -> Result<Vec<Commit>>;

    // Remote operations
    fn fetch(&self, repo: &Self::Repository, remote: Option<&str>, options: FetchOptions) -> Result<Vec<String>>;
    fn push(&self, repo: &Self::Repository, remote: &str, refspec: &str, options: PushOptions) -> Result<()>;
    fn pull(&self, repo: &Self::Repository, remote: Option<&str>, options: PullOptions) -> Result<()>;
    
    // Merge/rebase
    fn rebase(&self, repo: &Self::Repository, onto: &BranchName) -> Result<()>;
    fn merge(&self, repo: &Self::Repository, branch: &BranchName) -> Result<()>;

    // Workspace operations (JJ-specific, no-op for git)
    fn workspace_create(&self, repo: &Self::Repository, name: &str) -> Result<()>;
    fn workspace_list(&self, repo: &Self::Repository) -> Result<Vec<WorkspaceInfo>>;
    fn workspace_switch(&self, repo: &Self::Repository, name: &str) -> Result<()>;
    fn workspace_forget(&self, name: &str) -> Result<()>;

    // Operation log (JJ-specific)
    fn operation_log(&self, repo: &Self::Repository) -> Result<Vec<Operation>>;
    fn undo(&self, repo: &Self::Repository, operation_id: &str) -> Result<()>;
    fn checkpoint(&self, repo: &Self::Repository, name: &str) -> Result<()>;
}
```

### Type Hierarchy

```rust
// Branch representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    pub name: BranchName,
    pub is_current: bool,
    pub tracking: Option<BranchName>,
}

// Commit representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    pub hash: CommitHash,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub parent_hashes: Vec<CommitHash>,
}

// VCS status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VcsStatus {
    Clean,
    Dirty,
    Conflicted,
    Detached,
}

// Workspace info (JJ-specific)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub path: PathBuf,
    pub commit: CommitHash,
}

// Operation (JJ-specific)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    pub id: OperationId,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub tags: Vec<String>,
}
```

---

## Variants

### Variant A: Single Unified Trait with Option Types

```rust
pub trait VcsBackend {
    fn vcs_type(&self) -> VcsType;
    fn workspace_create(&self, name: &str) -> Result<()>;
    fn workspace_list(&self) -> Result<Vec<WorkspaceInfo>>;
    fn operation_log(&self) -> Result<Vec<Operation>>;
}
```

**Pros:**
- Single trait, simple type hierarchy
- Easy to switch implementations

**Cons:**
- Many methods return `None` or empty for git (pollutes API)
- Semantic confusion - git doesn't have workspaces
- Hard to know which operations are valid per backend

### Variant B: Core Trait + Extension Traits

```rust
pub trait VcsBackend {
    fn vcs_type(&self) -> VcsType;
    fn /* ... common operations */ 
}

pub trait WorkspaceSupport: VcsBackend {
    fn workspace_create(&self, name: &str) -> Result<()>;
    fn workspace_list(&self) -> Result<Vec<WorkspaceInfo>>;
    fn workspace_switch(&self, name: &str) -> Result<()>;
}

pub trait OperationLogSupport: VcsBackend {
    fn operation_log(&self) -> Result<Vec<Operation>>;
    fn undo(&self, operation_id: &str) -> Result<()>;
}
```

**Pros:**
- Clear which operations are available per backend
- Forced downcasting to use JJ-specific features

**Cons:**
- More complex type hierarchy
- Requires trait objects or enum dispatch

### Variant C: Enum-Based Backend Dispatch (CHOSEN)

```rust
pub enum VcsBackend {
    Git(GitBackend),
    Jujutsu(JjBackend),
}

impl VcsBackend {
    pub fn open(path: &Path) -> Result<Self> {
        if Self::is_jj_repo(path) {
            Ok(Self::Jujutsu(JjBackend::open(path)?))
        } else if Self::is_git_repo(path) {
            Ok(Self::Git(GitBackend::open(path)?))
        } else {
            Err(VcsError::NotInitialized)
        }
    }
}
```

**Pros:**
- Compile-time guarantee of backend-specific behavior
- No trait objects or dynamic dispatch
- Clear error messages when wrong backend used

**Cons:**
- Pattern matching required at call sites
- Adding new backends modifies enum

---

## Invariants

### Repository Invariants

```rust
/// INVARIANT: Repository path must exist and contain valid VCS metadata
assert!(repo_path.join(".git").exists() || repo_path.join(".jj").exists());

/// INVARIANT: Backend type must match actual repository type
assert_eq!(backend.vcs_type(), detect_vcs_type(repo_path));
```

### Branch Invariants

```rust
/// INVARIANT: Branch names must be valid
pub fn is_valid_branch_name(name: &str) -> bool {
    !name.is_empty() 
    && !name.contains(' ')
    && !name.contains('~')
    && !name.contains("..")
}

/// INVARIANT: Current branch is always in branch list
assert!(branches.iter().any(|b| b.is_current));
```

### Status Invariants

```rust
/// INVARIANT: Status is mutually exclusive
matches!(status, VcsStatus::Clean) 
    || matches!(status, VcsStatus::Dirty)
    || matches!(status, VcsStatus::Conflicted)
    || matches!(status, VcsStatus::Detached);

/// INVARIANT: Conflicted implies Dirty
assert_eq!(status == VcsStatus::Conflicted, status == VcsStatus::Dirty);
```

### Workspace Invariants (JJ-specific)

```rust
/// INVARIANT: Workspace names are unique per repository
assert!(workspaces.iter().all_unique_by(|w| &w.name));

/// INVARIANT: Workspace commit must exist in repository
assert!(repo.contains_commit(workspace.commit));
```

### Operation Invariants (JJ-specific)

```rust
/// INVARIANT: Operations are ordered by timestamp (newest first)
assert!(operations.windows(2).all(|w| w[0].timestamp >= w[1].timestamp));

/// INVARIANT: Operation IDs are unique
assert!(operations.iter().all_unique_by(|op| &op.id));
```

---

## Consequences

### Positive

1. **Backend transparency** - CLI and domain code doesn't care about Git vs JJ
2. **Testability** - Can mock backends for unit tests
3. **Extensibility** - Easy to add new VCS backends (Mercurial, Fossil)
4. **No shell out** - Pure Rust implementations via gix/jj-lib

### Negative

1. **Enum dispatch overhead** - Minor pattern matching at call sites
2. **Feature parity gaps** - JJ has workspaces, Git doesn't (handled via Result)
3. **jj-lib complexity** - JJ library is less mature than gix

### Implementation Requirements

| Backend | Library | Operations |
|---------|---------|------------|
| Git | `gix` | All core VCS operations |
| JJ | `jj-lib` | All core + workspaces + operation log |

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/vcs/src/backend/mod.rs` | VcsBackend enum |
| `crates/vcs/src/backend/git.rs` | GitBackend implementation |
| `crates/vcs/src/backend/jj.rs` | JjBackend implementation |
| `crates/vcs/src/domain/types.rs` | BranchName, Commit, VcsStatus types |
| `crates/vcs/src/traits.rs` | VcsBackend trait |

---

## Related ADRs

- ADR-001: CLI Architecture (commands that use VCS)
- ADR-005: Workspace Isolation Model (JJ workspace concept)
- ADR-003: Restate Feature Parity (JJ integration for durability)
