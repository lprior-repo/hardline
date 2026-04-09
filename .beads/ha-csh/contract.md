# Contract: VCS Worktree Operations

## bead_id: ha-csh
## bead_title: VCS gix: Worktree operations (list, create, remove)
## phase: contract
## updated_at: 2026-04-08T22:56:00Z

---

## 1. Overview

This contract defines worktree operations for the VCS layer using gix (pure Rust Git implementation).

## 2. WorktreeInfo Type

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: AbsolutePath,      // Absolute path to worktree root
    pub name: WorktreeName,       // Human-readable name
    pub is_main: bool,            // True for main repository worktree
    pub branch: Option<BranchName>, // Current branch (None if detached)
    pub head: CommitId,          // Current HEAD commit
}
```

## 3. Operations

### 3.1 list_worktrees(repo: &Repository) -> Vec<WorktreeInfo>

**Preconditions:**
- P1: Repository must be open and valid

**Postconditions:**
- Q1: Returns all worktrees including main repository
- Q2: Main worktree has `is_main = true`
- Q3: Worktree paths are absolute and canonical
- Q4: Branch name has no `refs/heads/` prefix

**Porcelain Parsing:**
- Uses `git worktree list --porcelain` output format
- Parses fields: `worktree <path>`, `HEAD <hex>`, `branch <ref>`, `detached`

**Errors:**
- E1: `VcsError::GitParseError` if porcelain output malformed

### 3.2 create_worktree(repo: &Repository, path: &Path, branch: &BranchName) -> WorktreeInfo

**Preconditions:**
- P1: Repository must be open and valid
- P2: Path must not already exist
- P3: Branch must exist in repository

**Postconditions:**
- Q1: New worktree directory created at path
- Q2: Worktree is on specified branch
- Q3: Returns `WorktreeInfo` for new worktree

**Errors:**
- E1: `VcsError::PathExists` if path already exists
- E2: `VcsError::NotFound` if branch doesn't exist
- E3: `VcsError::GitReferenceError` on gix failure

### 3.3 remove_worktree(repo: &Repository, path: &Path, force: bool) -> Result<(), VcsError>

**Preconditions:**
- P1: Repository must be open and valid
- P2: Path must be an existing worktree (not main)
- P3: Worktree working directory must be clean (unless force=true)

**Postconditions:**
- Q1: Worktree directory removed from filesystem
- Q2: Worktree reference removed from .git/worktrees

**Errors:**
- E1: `VcsError::NotFound` if worktree doesn't exist
- E2: `VcsError::DirtyWorkingDirectory` if not clean and force=false
- E3: `VcsError::InvalidState` if trying to remove main worktree

## 4. Error Taxonomy

| Error | Condition |
|-------|-----------|
| `VcsError::PathExists` | Worktree path already exists |
| `VcsError::NotFound { entity: "Worktree" }` | Worktree not found |
| `VcsError::DirtyWorkingDirectory` | Worktree has uncommitted changes |
| `VcsError::InvalidState("cannot remove main worktree")` | Attempt to remove main |
| `VcsError::GitParseError` | Porcelain parsing failed |

## 5. Invariants

- I1: Worktree paths are always absolute
- I2: `is_main` is `true` only for the main repository
- I3: Worktree branch is `None` only when in detached HEAD state
- I4: All worktrees in list belong to the same repository

## 6. gix Worktree API

gix provides worktree operations via:
- `repo.worktrees()` - iterate all worktrees
- `gix::worktree::Stack` - worktree management

Note: gix worktree support is evolving. The implementation uses porcelain parsing as fallback.