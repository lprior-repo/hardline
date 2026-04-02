# ADR-004: VCS Abstraction - Git-Only Version Control Backend

**Date:** 2026-03-20
**Revised:** 2026-04-02
**Status:** Accepted
**Deciders:** Lewis

---

## Context

Hardline requires a version control backend for workspace operations, branch management, and commit handling. The system must support:

1. **Git repositories** - using gitoxide (gix) for pure Rust implementation
2. **No shell out** - all VCS operations via Rust libraries, not CLI spawning
3. **Workspace isolation** - full clones for agent isolation (see ADR-005)
4. **Reliable concurrency** - Git's locking model is sufficient when each workspace is an isolated full clone

The original design attempted to abstract over both Git and Jujutsu (JJ). That added complexity without sufficient justification. JJ has been removed from the project. Hardline is Git-only.

---

## Decision

### Single Backend: Git via gitoxide

Hardline uses a single VCS backend. There is no `VcsBackend` trait with multiple implementations. Instead, the `GitBackend` struct provides all VCS operations directly.

```rust
pub struct GitBackend {
    repo_path: AbsolutePath,
}

impl GitBackend {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn init(path: &Path) -> Result<Self>;
    pub fn clone(source: &str, target: &Path) -> Result<Self>;

    // Branch operations
    pub fn current_branch(&self) -> Result<Option<BranchName>>;
    pub fn list_branches(&self) -> Result<Vec<Branch>>;
    pub fn create_branch(&self, name: &BranchName) -> Result<()>;
    pub fn delete_branch(&self, name: &BranchName) -> Result<()>;
    pub fn switch_branch(&self, name: &BranchName) -> Result<()>;

    // Commit operations
    pub fn status(&self) -> Result<VcsStatus>;
    pub fn add(&self, paths: &[&Path]) -> Result<()>;
    pub fn commit(&self, message: &str) -> Result<CommitHash>;
    pub fn log(&self, count: usize) -> Result<Vec<Commit>>;

    // Remote operations
    pub fn fetch(&self, remote: Option<&str>, options: FetchOptions) -> Result<Vec<String>>;
    pub fn push(&self, remote: &str, refspec: &str, options: PushOptions) -> Result<()>;
    pub fn pull(&self, remote: Option<&str>, options: PullOptions) -> Result<()>;

    // Merge/rebase
    pub fn rebase(&self, onto: &BranchName) -> Result<()>;
    pub fn merge(&self, branch: &BranchName) -> Result<()>;
}
```

### Why No Trait

The original design used a `VcsBackend` trait with enum dispatch over `Git` and `JJ` variants. That is unnecessary now:

- **No alternative backends** exist or are planned
- **Enum dispatch** added pattern matching at every call site for no benefit
- **Trait objects** added dynamic dispatch overhead
- **Extension traits** for JJ-specific features (workspaces, operation log) polluted the API

A concrete struct is simpler, faster, and easier to maintain. If a second VCS backend is ever needed, a trait can be extracted then. YAGNI.

### Type Hierarchy

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchName(String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    pub name: BranchName,
    pub is_current: bool,
    pub tracking: Option<BranchName>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    pub hash: CommitHash,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub parent_hashes: Vec<CommitHash>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VcsStatus {
    Clean,
    Dirty,
    Conflicted,
    Detached,
}
```

---

## Variants Considered

### Variant A: Trait-Based Multi-Backend (REJECTED)

```rust
pub trait VcsBackend: Send + Sync {
    fn current_branch(&self) -> Result<Option<BranchName>>;
    // ... many methods
}

pub enum VcsBackend {
    Git(GitBackend),
    Jj(JjBackend),  // removed from project
}
```

**Rejected because:**
- No alternative backend exists
- Enum dispatch adds overhead at every call site
- JJ-specific methods (workspace, operation log) had no Git implementation
- Violates YAGNI

### Variant B: Thin Wrapper Around git2 (REJECTED)

```rust
pub struct GitBackend {
    repo: git2::Repository,
}
```

**Rejected because:**
- `git2` is a C binding (libgit2), not pure Rust
- Hardline requires pure Rust for WASM compatibility and build simplicity
- gitoxide (gix) provides the same functionality in pure Rust

### Variant C: Concrete GitBackend with gix (CHOSEN)

```rust
pub struct GitBackend {
    repo_path: AbsolutePath,
}
```

**Chosen because:**
- Single concrete type, no dispatch overhead
- Pure Rust via gitoxide (gix)
- Simple mental model: one backend, one set of operations
- Easy to test by cloning to temp directories
- Workspace operations live in the workspace crate, not here

---

## Invariants

### Repository Invariants

```rust
/// INVARIANT: Repository path must exist and contain a .git directory
assert!(repo_path.join(".git").exists());

/// INVARIANT: Backend is always Git - no runtime detection needed
```

### Branch Invariants

```rust
/// INVARIANT: Branch names must be valid Git branch names
pub fn is_valid_branch_name(name: &str) -> bool {
    !name.is_empty()
    && !name.contains(' ')
    && !name.contains('~')
    && !name.contains("..")
}

/// INVARIANT: Current branch is always in branch list (when not detached)
if let Some(current) = current_branch {
    assert!(branches.iter().any(|b| b.is_current));
}
```

### Status Invariants

```rust
/// INVARIANT: Status is mutually exclusive
matches!(status, VcsStatus::Clean)
    || matches!(status, VcsStatus::Dirty)
    || matches!(status, VcsStatus::Conflicted)
    || matches!(status, VcsStatus::Detached);
```

---

## Consequences

### Positive

1. **Simplicity** - One backend, one code path, no abstraction overhead
2. **Performance** - No trait objects, no enum dispatch, direct method calls
3. **Pure Rust** - gitoxide provides all Git operations without C dependencies
4. **Testability** - Clone to temp dir, run operations, assert results
5. **Maintainability** - Less code to maintain, no unused abstraction layers

### Negative

1. **No VCS flexibility** - If a non-Git VCS is needed, refactoring is required
2. **gitoxide maturity** - gix is less mature than libgit2, some edge cases may surface

### Implementation Requirements

| Backend | Library | Operations |
|---------|---------|------------|
| Git | `gix` | All core VCS operations (branch, commit, fetch, push, pull, rebase, merge) |

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/vcs/src/backend/mod.rs` | GitBackend struct and implementation |
| `crates/vcs/src/backend/git.rs` | Git operations via gitoxide |
| `crates/vcs/src/domain/types.rs` | BranchName, Commit, VcsStatus types |

---

## Related ADRs

- ADR-001: CLI Architecture (commands that use VCS)
- ADR-005: Workspace Isolation Model (full clone isolation via Git)
- ADR-003: Restate Feature Parity (durable VCS operations)
