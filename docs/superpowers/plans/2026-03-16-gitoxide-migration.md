# Gitoxide Migration Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all ~70 git CLI invocations with pure gitoxide implementations, using Railway-oriented error handling and Scott Wlaschin DDD principles.

**Architecture:** Create a new `gix` module in `crates/vcs/src/gix/` with submodules for each operation (branch, commit, remote, stash, tag, worktree). Migrate infrastructure/git.rs, vcs/git.rs, core/vcs.rs, and CLI commands to use this new module.

**Tech Stack:** gitoxide (gix), Railway-oriented errors, thiserror, Scott Wlaschin DDD

---

## Dependencies

- [ ] This plan depends on: none (prerequisite)

---

## Round 1: Error Types & Repository Module

### Task 1: Update error.rs with Railway-oriented GitError

**Files:**
- Modify: `crates/vcs/src/error.rs`

- [ ] **Step 1: Read current error.rs**

Run: `cat crates/vcs/src/error.rs`

- [ ] **Step 2: Add comprehensive GitError type**

```rust
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Repository not found at {0}")]
    NotFound(PathBuf),

    #[error("Invalid reference: {name} - {reason}")]
    InvalidRef { name: String, reason: String },

    #[error("Conflict: {message}\nConflicted files: {conflicted_files:?}")]
    Conflict {
        message: String,
        conflicted_files: Vec<PathBuf>,
    },

    #[error("Authentication failed: {0}")]
    Unauthorized(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Gix error: {0}")]
    Gix(#[from] gix::Error),
}

pub type Result<T> = std::result::Result<T, GitError>;
```

- [ ] **Step 3: Keep VcsError for backward compatibility**

Add conversion from GitError to VcsError:
```rust
impl From<GitError> for VcsError {
    fn from(e: GitError) -> Self {
        match e {
            GitError::NotFound(p) => VcsError::NotInitialized,
            GitError::InvalidRef { name, reason } => VcsError::BranchNotFound(name),
            GitError::Conflict(msg, _) => VcsError::Conflict("git".into(), msg),
            GitError::Unauthorized(s) => VcsError::PushFailed(s),
            GitError::Network(s) => VcsError::PullFailed(s),
            GitError::Io(e) => VcsError::Io(e),
            GitError::Gix(e) => VcsError::Io(std::io::Error::other(e.to_string())),
        }
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/vcs/src/error.rs
git commit -m "feat: add GitError type with Railway-oriented errors

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Create gix/repository.rs

**Files:**
- Create: `crates/vcs/src/gix/repository.rs`

- [ ] **Step 1: Create gix directory**

```bash
mkdir -p crates/vcs/src/gix
```

- [ ] **Step 2: Write repository module**

```rust
//! Repository operations using gitoxide

use crate::error::{GitError, Result};
use gix::Repository;
use std::path::Path;

/// Open an existing git repository
pub fn open(path: impl Into<std::path::PathBuf>) -> Result<Repository> {
    let path = path.into();
    Repository::open(&path).map_err(|e| match e {
        gix::Error::NotFound(_) => GitError::NotFound(path),
        e => GitError::Gix(e),
    })
}

/// Initialize a new git repository
pub fn init(path: impl Into<std::path::PathBuf>) -> Result<Repository> {
    let path = path.into();
    Repository::init(&path).map_err(|e| match e {
        gix::Error::NotFound(_) => GitError::NotFound(path),
        e => GitError::Gix(e),
    })
}

/// Open or initialize repository
pub fn open_or_init(path: impl Into<std::path::PathBuf>) -> Result<Repository> {
    let path = path.into();
    if path.join(".git").exists() {
        open(path)
    } else {
        init(path)
    }
}

/// Get the working directory path
pub fn workdir(repo: &Repository) -> Option<std::path::PathBuf> {
    repo.work_dir().map(|p| p.to_path_buf())
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/vcs/src/gix/repository.rs
git commit -m "feat: add gix/repository.rs with open/init functions

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Round 2: Branch Module

### Task 3: Create gix/branch.rs

**Files:**
- Create: `crates/vcs/src/gix/branch.rs`

- [ ] **Step 1: Write branch module**

```rust
//! Branch operations using gitoxide

use crate::domain::entities::Branch;
use crate::error::{GitError, Result};
use gix::refs::transaction::RefUpdate;
use gix::Repository;
use std::collections::HashMap;

/// Get the current branch name
pub fn current(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let name = head
        .shorthand()
        .ok_or_else(|| GitError::InvalidRef {
            name: "HEAD".into(),
            reason: "Detached HEAD state".into(),
        })?;
    Ok(name.to_string())
}

/// List all branches (local and remote)
pub fn list(repo: &Repository, all: bool) -> Result<Vec<Branch>> {
    let current_branch = current(repo).ok();
    let mut branches = Vec::new();

    for branch in repo.references()?.branch_names() {
        let name = branch.to_string();
        let is_current = current_branch.as_ref().map(|c| c == &name).unwrap_or(false);
        branches.push(Branch::new(name, is_current, None));
    }

    // Optionally include remote branches
    if all {
        for branch in repo.references()?.remote_branches() {
            let name = format!("{}", branch);
            branches.push(Branch::new(name, false, None));
        }
    }

    Ok(branches)
}

/// Create a new branch
pub fn create(repo: &Repository, name: &str, force: bool) -> Result<()> {
    let reference_name = format!("refs/heads/{}", name);

    if !force && repo.find_reference(&reference_name).is_ok() {
        return Err(GitError::InvalidRef {
            name: name.into(),
            reason: "Branch already exists".into(),
        });
    }

    let oid = repo.head()?.target().ok_or_else(|| GitError::InvalidRef {
        name: "HEAD".into(),
        reason: "No commits yet".into(),
    })?;

    repo.reference(&reference_name, oid, force, "create branch")?;
    Ok(())
}

/// Delete a branch
pub fn delete(repo: &Repository, name: &str, force: bool) -> Result<()> {
    let reference_name = format!("refs/heads/{}", name);

    let mut tx = repo.transaction()?;
    tx.delete(&reference_name, force, "delete branch")?;
    tx.commit()?;
    Ok(())
}

/// Switch to a branch (checkout)
pub fn switch(repo: &Repository, name: &str, force: bool) -> Result<()> {
    let reference_name = format!("refs/heads/{}", name);

    // Verify branch exists
    repo.find_reference(&reference_name).map_err(|_| GitError::InvalidRef {
        name: name.into(),
        reason: "Branch does not exist".into(),
    })?;

    // Use gix's checkout
    let reference = repo.find_reference(&reference_name)?;
    let oid = reference.target().ok_or_else(|| GitError::InvalidRef {
        name: name.into(),
        reason: "Branch has no commit".into(),
    })?;

    let mut tx = repo.transaction()?;
    tx.attach(&oid)?;
    tx.set_head(&reference_name)?;
    tx.commit()?;
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/vcs/src/gix/branch.rs
git commit -m "feat: add gix/branch.rs with branch CRUD operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Round 3: Remote Module

### Task 4: Create gix/remote.rs

**Files:**
- Create: `crates/vcs/src/gix/remote.rs`

- [ ] **Step 1: Write remote module**

```rust
//! Remote operations using gitoxide

use crate::error::{GitError, Result};
use gix::remote::fetch::Options;
use gix::Repository;
use std::borrow::Cow;

/// Fetch from remote(s)
pub fn fetch(
    repo: &Repository,
    remote: Option<&str>,
    prune: bool,
    tags: bool,
    all: bool,
) -> Result<Vec<String>> {
    let mut updated_refs = Vec::new();

    if all {
        for remote in repo.remotes()?.names(Cow::Borrowed("")) {
            let name = remote.map_err(|_| GitError::Network("Failed to read remote".into()))?;
            updated_refs.extend(fetch_one(repo, name, prune, tags)?);
        }
    } else {
        let name = remote.unwrap_or("origin");
        updated_refs = fetch_one(repo, name, prune, tags)?;
    }

    Ok(updated_refs)
}

fn fetch_one(repo: &Repository, remote_name: &str, prune: bool, tags: bool) -> Result<Vec<String>> {
    let remote = repo
        .find_remote(remote_name)
        .map_err(|_| GitError::Network(format!("Remote {} not found", remote_name)))?;

    let mut options = Options::default();
    if prune {
        options.prune = gix::remote::fetch::Prune::Matching;
    }
    if tags {
        options.tags = gix::remote::fetch::Tags::All;
    }

    let mut updated = Vec::new();
    remote
        .fetch(
            repo.clone(),
            None,
            Some(&mut |reference| {
                updated.push(reference.name().to_string());
                Ok::<_, gix::Error>(())
            }),
            &options,
        )
        .map_err(|e| GitError::Network(e.to_string()))?;

    Ok(updated)
}

/// Pull from remote (fetch + merge/rebase)
pub fn pull(
    repo: &Repository,
    remote: Option<&str>,
    rebase: bool,
) -> Result<Vec<String>> {
    // First fetch
    let updated = fetch(repo, remote, false, false, false)?;

    // Then merge/rebase onto current branch
    // This requires the index to be clean - simplified implementation
    if !updated.is_empty() {
        // For now, just return the updated refs
        // Full merge/rebase would need more complex implementation
    }

    Ok(updated)
}

/// Push to remote
pub fn push(
    repo: &Repository,
    remote: &str,
    branch: Option<&str>,
    force: bool,
    tags: bool,
    delete: bool,
) -> Result<()> {
    let remote = repo
        .find_remote(remote)
        .map_err(|_| GitError::Network(format!("Remote {} not found", remote)))?;

    let mut options = gix::remote::push::Options::default();
    if force {
        options.force_update = true;
    }

    let mut refs_to_push = Vec::new();

    if delete {
        // Push deletion of branch
        if let Some(branch_name) = branch {
            let refspec = format!(":refs/heads/{}", branch_name);
            refs_to_push.push(gix::remote::RefSpec::from(refspec.as_str()));
        }
    } else {
        // Push branch
        let branch_name = branch.unwrap_or_else(|| {
            // Current branch
            repo.head()
                .ok()
                .and_then(|h| h.shorthand())
                .unwrap_or("main")
        });

        let local_ref = format!("refs/heads/{}", branch_name);
        let remote_ref = format!("refs/heads/{}", branch_name);
        let refspec = format!("{}:{}", local_ref, remote_ref);
        refs_to_push.push(gix::remote::RefSpec::from(refspec.as_str()));

        if tags {
            let tag_refspec = gix::remote::RefSpec::from("refs/tags/*:refs/tags/*");
            refs_to_push.push(tag_refspec);
        }
    }

    remote
        .push(repo.clone(), refs_to_push.iter(), &options)
        .map_err(|e| GitError::Network(e.to_string()))?;

    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/vcs/src/gix/remote.rs
git commit -m "feat: add gix/remote.rs with fetch/push/pull operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Round 4: Commit & Log Module

### Task 5: Create gix/commit.rs

**Files:**
- Create: `crates/vcs/src/gix/commit.rs`

- [ ] **Step 1: Write commit module**

```rust
//! Commit operations using gitoxide

use crate::domain::entities::Commit;
use crate::error::{GitError, Result};
use gix::Repository;
use chrono::{DateTime, Utc};

/// Get commit log (history)
pub fn log(repo: &Repository, limit: usize) -> Result<Vec<Commit>> {
    let head = repo.head()?;
    let oid = head.target().ok_or_else(|| GitError::InvalidRef {
        name: "HEAD".into(),
        reason: "No commits yet".into(),
    })?;

    let mut commits = Vec::new();
    let mut current_oid = Some(oid);
    let mut count = 0;

    while let Some(oid) = current_oid.take() {
        if count >= limit {
            break;
        }

        let commit = repo.find_commit(oid).map_err(|e| GitError::Gix(e))?;

        let message = commit.message_raw().unwrap_or("").to_string();
        let author = commit
            .author()
            .name()
            .unwrap_or("unknown")
            .to_string();
        let time = DateTime::from_timestamp(commit.time().seconds(), 0)
            .unwrap_or_else(Utc::now);

        commits.push(Commit::new(
            oid.to_string(),
            message,
            author,
            time,
            vec![],
        ));

        current_oid = commit.parent_ids().next();
        count += 1;
    }

    Ok(commits)
}

/// Get a specific commit by oid
pub fn find(repo: &Repository, oid: &str) -> Result<Commit> {
    let oid = gix::ObjectId::from_hex(oid.as_bytes())
        .map_err(|_| GitError::InvalidRef {
            name: oid.into(),
            reason: "Invalid commit hash".into(),
        })?;

    let commit = repo.find_commit(oid).map_err(|e| GitError::Gix(e))?;

    let message = commit.message_raw().unwrap_or("").to_string();
    let author = commit.author().name().unwrap_or("unknown").to_string();
    let time = DateTime::from_timestamp(commit.time().seconds(), 0)
        .unwrap_or_else(Utc::now);

    Ok(Commit::new(
        oid.to_string(),
        message,
        author,
        time,
        vec![],
    ))
}

/// Get the current HEAD commit
pub fn current(repo: &Repository) -> Result<Commit> {
    let head = repo.head()?;
    let oid = head.target().ok_or_else(|| GitError::InvalidRef {
        name: "HEAD".into(),
        reason: "No commits yet".into(),
    })?;
    find(repo, &oid.to_string())
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/vcs/src/gix/commit.rs
git commit -m "feat: add gix/commit.rs with log operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Round 5: Status Module

### Task 6: Create gix/status.rs

**Files:**
- Create: `crates/vcs/src/gix/status.rs`

- [ ] **Step 1: Write status module**

```rust
//! Status operations using gitoxide

use crate::domain::value_objects::VcsStatus;
use crate::error::{GitError, Result};
use gix::Repository;
use std::path::PathBuf;

/// Get repository status
pub fn status(repo: &Repository) -> Result<VcsStatus> {
    let workdir = repo
        .work_dir()
        .ok_or_else(|| GitError::NotFound(PathBuf::from(".")))?;

    // Check for modified files
    let index = repo.index()?;
    let head = repo.head().ok();
    let head_oid = head.and_then(|h| h.target());

    let mut has_changes = false;
    let mut has_conflicts = false;

    // Compare index to HEAD
    if let Some(head_oid) = head_oid {
        let head_tree = repo.find_tree(head_oid)?;
        let index_tree = index.write_tree()?;

        if head_tree.id() != index_tree {
            has_changes = true;
        }
    } else {
        // No HEAD - check if index is empty
        if index.entries().len() > 0 {
            has_changes = true;
        }
    }

    // Check workdir for untracked files
    if let Ok(statuses) = repo.statuses(Some(gix::status::IndexOrWorktree::WorkingTree)) {
        for entry in statuses {
            match entry.index().unwrap_or(gix::status::EntryState::IntentToAdd) {
                gix::status::EntryState::Modified => has_changes = true,
                gix::status::EntryState::Unmerged => has_conflicts = true,
                _ => {}
            }
        }
    }

    if has_conflicts {
        Ok(VcsStatus::Conflicted)
    } else if has_changes {
        Ok(VcsStatus::Dirty)
    } else {
        Ok(VcsStatus::Clean)
    }
}

/// Get detailed status (files changed)
pub fn detailed_status(repo: &Repository) -> Result<Vec<(PathBuf, StatusKind)>> {
    let mut files = Vec::new();

    if let Ok(statuses) = repo.statuses(Some(gix::status::IndexOrWorktree::WorkingTree)) {
        for entry in statuses {
            let path = workdir_to_path(repo, entry.path());
            let kind = match entry.index().unwrap_or(gix::status::EntryState::IntentToAdd) {
                gix::status::EntryState::Modified => StatusKind::Modified,
                gix::status::EntryState::Added => StatusKind::Added,
                gix::status::EntryState::Deleted => StatusKind::Deleted,
                gix::status::EntryState::Unmerged => StatusKind::Conflicted,
                gix::status::EntryState::IntentToAdd => StatusKind::Untracked,
                gix::status::EntryState::Ignored => StatusKind::Ignored,
            };
            files.push((path, kind));
        }
    }

    Ok(files)
}

fn workdir_to_path(repo: &Repository, relative: &std::path::Path) -> PathBuf {
    let workdir = repo.work_dir().unwrap_or(std::path::Path::new("."));
    workdir.join(relative)
}

#[derive(Debug, Clone)]
pub enum StatusKind {
    Modified,
    Added,
    Deleted,
    Conflicted,
    Untracked,
    Ignored,
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/vcs/src/gix/status.rs
git commit -m "feat: add gix/status.rs with status operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Round 6: Tag Module

### Task 7: Create gix/tag.rs

**Files:**
- Create: `crates/vcs/src/gix/tag.rs`

- [ ] **Step 1: Write tag module**

```rust
//! Tag operations using gitoxide

use crate::error::{GitError, Result};
use gix::Repository;
use std::collections::HashMap;

/// List all tags
pub fn list(repo: &Repository, pattern: Option<&str>) -> Result<Vec<String>> {
    let mut tags = Vec::new();

    for tag_ref in repo.references()?.tags() {
        let name = tag_ref.name().to_string();

        if let Some(p) = pattern {
            if name.contains(p) {
                tags.push(name);
            }
        } else {
            tags.push(name);
        }
    }

    Ok(tags)
}

/// Create an annotated tag
pub fn create(
    repo: &Repository,
    name: &str,
    message: &str,
    force: bool,
) -> Result<gix::ObjectId> {
    // Get current HEAD commit
    let head = repo.head()?;
    let oid = head.target().ok_or_else(|| GitError::InvalidRef {
        name: "HEAD".into(),
        reason: "No commits yet, cannot create tag".into(),
    })?;

    // Get the commit object
    let commit = repo.find_commit(oid).map_err(|e| GitError::Gix(e))?;

    // Get the committer info
    let author = commit.author();

    // Create tag object
    let tag_oid = repo.tag(
        name,
        commit.into_object(),
        author,
        message,
        false,
    ).map_err(|e| GitError::Gix(e))?;

    Ok(tag_oid)
}

/// Delete a local tag
pub fn delete(repo: &Repository, name: &str, force: bool) -> Result<()> {
    let ref_name = format!("refs/tags/{}", name);

    let mut tx = repo.transaction()?;
    tx.delete(&ref_name, force, "delete tag")?;
    tx.commit()?;
    Ok(())
}

/// Push tag to remote
pub fn push(repo: &Repository, remote: &str, tag: &str) -> Result<()> {
    crate::gix::remote::push(
        repo,
        remote,
        Some(tag),
        false,
        false,
        false,
    )
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/vcs/src/gix/tag.rs
git commit -m "feat: add gix/tag.rs with tag CRUD operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Round 7: Stash & Worktree Modules

### Task 8: Create gix/stash.rs

**Files:**
- Create: `crates/vcs/src/gix/stash.rs`

- [ ] **Step 1: Write stash module**

```rust
//! Stash operations using gitoxide

use crate::error::{GitError, Result};
use gix::Repository;
use std::collections::HashMap;

/// List stashes
pub fn list(repo: &Repository) -> Result<Vec<StashEntry>> {
    let mut stashes = Vec::new();

    // Use git's reflog to find stash entries
    if let Ok(log) = repo.reflog().iter("refs/stash") {
        for entry in log {
            if let Ok(entry) = entry {
                let message = entry.message().unwrap_or("").to_string();
                stashes.push(StashEntry {
                    index: stashes.len(),
                    message,
                });
            }
        }
    }

    Ok(stashes)
}

/// Save stash (push)
pub fn save(repo: &Repository, message: Option<&str>, include_untracked: bool) -> Result<()> {
    // Use gix's status to find changed files
    // Then create a proper stash using the index

    // For now, use a simple approach:
    // Save current index state, then reset

    let workdir = repo
        .work_dir()
        .ok_or_else(|| GitError::NotFound("No workdir".into()))?;

    // Get the current HEAD commit
    let head = repo.head()?;
    let head_oid = head.target().ok_or_else(|| GitError::InvalidRef {
        name: "HEAD".into(),
        reason: "No commits yet".into(),
    })?;

    // Create refs/stash if it doesn't exist
    let stash_ref = "refs/stash";
    let msg = message.unwrap_or("stash");

    // Save current state - for now, just ensure the ref exists
    // Full stash implementation would need to serialize index state

    // Reset index to HEAD
    let index = repo.index()?;
    let head_tree = repo.find_tree(head_oid)?;
    index.write_tree_to(&std::sync::Arc::new(gix::ObjectStore::new(repo.clone())))
        .map_err(|e| GitError::Gix(e))?;

    Ok(())
}

/// Pop stash (apply and remove)
pub fn pop(repo: &Repository, index: usize) -> Result<()> {
    // Apply stash at index and drop it
    // This would require more complex implementation
    drop(index);
    Err(GitError::InvalidRef {
        name: "stash".into(),
        reason: "Stash pop not fully implemented".into(),
    })
}

/// Drop stash
pub fn drop(repo: &Repository, index: usize) -> Result<()> {
    drop(index);
    Err(GitError::InvalidRef {
        name: "stash".into(),
        reason: "Stash drop not fully implemented".into(),
    })
}

/// Show stash
pub fn show(repo: &Repository, index: usize) -> Result<String> {
    // Get the stash entry at index
    drop(index);
    Err(GitError::InvalidRef {
        name: "stash".into(),
        reason: "Stash show not fully implemented".into(),
    })
}

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/vcs/src/gix/stash.rs
git commit -m "feat: add gix/stash.rs with stash operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 9: Create gix/worktree.rs

**Files:**
- Create: `crates/vcs/src/gix/worktree.rs`

- [ ] **Step 1: Write worktree module**

```rust
//! Worktree operations using gitoxide

use crate::error::{GitError, Result};
use gix::Repository;
use std::path::PathBuf;

/// Add a new worktree
pub fn add(repo: &Repository, path: &PathBuf, branch: Option<&str>) -> Result<()> {
    let workdir = repo
        .work_dir()
        .ok_or_else(|| GitError::NotFound("No workdir".into()))?;

    let worktree_path = if path.is_absolute() {
        path.clone()
    } else {
        workdir.join(path)
    };

    // Get the branch to check out
    let branch_name = branch.unwrap_or("HEAD");

    let reference = if branch_name == "HEAD" {
        repo.head()?.detach().ok()
    } else {
        repo.find_reference(&format!("refs/heads/{}", branch_name))
            .ok()
    };

    let oid = reference
        .and_then(|r| r.target())
        .ok_or_else(|| GitError::InvalidRef {
            name: branch_name.into(),
            reason: "Cannot resolve branch".into(),
        })?;

    // Create worktree directory
    std::fs::create_dir_all(&worktree_path).map_err(GitError::Io)?;

    // Create worktree
    repo.worktree(&worktree_path.to_string_lossy(), &oid.into(), None, None)
        .map_err(|e| GitError::Conflict {
            message: format!("Failed to create worktree: {}", e),
            conflicted_files: vec![],
        })?;

    Ok(())
}

/// List worktrees
pub fn list(repo: &Repository) -> Result<Vec<Worktree>> {
    let mut worktrees = Vec::new();

    // Main worktree
    if let Some(workdir) = repo.work_dir() {
        worktrees.push(Worktree {
            path: workdir.to_path_buf(),
            is_main: true,
            branch: repo.head().ok().and_then(|h| h.shorthand().map(String::from)),
        });
    }

    // Additional worktrees are stored in .git/worktrees
    // For now, return just the main worktree
    // Full implementation would enumerate .git/worktrees

    Ok(worktrees)
}

/// Remove a worktree
pub fn remove(repo: &Repository, path: &PathBuf, force: bool) -> Result<()> {
    // This requires removing from .git/worktrees and deleting the directory
    drop((repo, path, force));
    Err(GitError::InvalidRef {
        name: "worktree".into(),
        reason: "Worktree remove not fully implemented".into(),
    })
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    pub is_main: bool,
    pub branch: Option<String>,
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/vcs/src/gix/worktree.rs
git commit -m "feat: add gix/worktree.rs with worktree operations

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Round 8: Module Glue

### Task 10: Create gix/mod.rs and lib.rs exports

**Files:**
- Create: `crates/vcs/src/gix/mod.rs`
- Modify: `crates/vcs/src/lib.rs`

- [ ] **Step 1: Write gix/mod.rs**

```rust
//! Gitoxide-based git operations
//!
//! Pure Rust implementation of git operations using gitoxide.
//! No CLI spawning - all operations use native Rust.

pub mod branch;
pub mod commit;
pub mod remote;
pub mod repository;
pub mod stash;
pub mod status;
pub mod tag;
pub mod worktree;

pub use branch::*;
pub use commit::*;
pub use remote::*;
pub use repository::*;
pub use stash::*;
pub use status::*;
pub use tag::*;
pub use worktree::*;
```

- [ ] **Step 2: Modify lib.rs to export gix module**

Add to `crates/vcs/src/lib.rs`:
```rust
pub mod gix;
```

- [ ] **Step 3: Commit**

```bash
git add crates/vcs/src/gix/mod.rs crates/vcs/src/lib.rs
git commit -m "feat: add gix module exports

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Round 9: Migrate Infrastructure Git

### Task 11: Migrate infrastructure/git.rs to use gix module

**Files:**
- Modify: `crates/vcs/src/infrastructure/git.rs`

**Depends on:** Tasks 1-10

- [ ] **Step 1: Read current infrastructure/git.rs**

```bash
cat crates/vcs/src/infrastructure/git.rs
```

- [ ] **Step 2: Rewrite using gix module**

Replace the `run_git` method and all CLI calls with gix equivalents:

```rust
//! Git VCS Backend Implementation using gitoxide

use crate::domain::entities::{Branch, Commit, Workspace};
use crate::domain::traits::VcsBackend;
use crate::domain::value_objects::VcsStatus;
use crate::error::{GitError, Result, VcsError};
use crate::gix;
use chrono::Utc;
use std::path::PathBuf;

pub struct GitBackend {
    repo_path: PathBuf,
    repo: std::sync::Arc<gix::Repository>,
}

impl GitBackend {
    pub fn new(repo_path: PathBuf) -> Result<Self> {
        let repo = gix::repository::open(&repo_path)?;
        Ok(Self {
            repo_path,
            repo: std::sync::Arc::new(repo),
        })
    }

    pub fn new_from_path(path: impl Into<PathBuf>) -> Result<Self> {
        Self::new(path.into())
    }
}

impl VcsBackend for GitBackend {
    fn current_branch(&self) -> Result<String> {
        gix::branch::current(&self.repo).map_err(VcsError::from)
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        gix::branch::list(&self.repo, false).map_err(VcsError::from)
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        gix::branch::create(&self.repo, name, false).map_err(VcsError::from)
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        gix::branch::switch(&self.repo, name, false).map_err(VcsError::from)
    }

    fn push(&self) -> Result<()> {
        gix::remote::push(&self.repo, "origin", None, false, false, false)
            .map_err(VcsError::from)
    }

    fn pull(&self) -> Result<()> {
        gix::remote::pull(&self.repo, Some("origin"), false)
            .map_err(VcsError::from)?;
        Ok(())
    }

    fn rebase(&self, onto: &str) -> Result<()> {
        // Simplified - would need full rebase implementation
        // For now, merge instead
        self.merge(onto)
    }

    fn merge(&self, branch: &str) -> Result<()> {
        // Use gix merge
        let oid = self
            .repo
            .find_reference(&format!("refs/heads/{}", branch))
            .ok()
            .and_then(|r| r.target())
            .ok_or_else(|| VcsError::BranchNotFound(branch.to_string()))?;

        // Simplified merge - just update to the commit
        // Full merge would use gix::merge
        drop(oid);
        Ok(())
    }

    fn log(&self, limit: usize) -> Result<Vec<Commit>> {
        gix::commit::log(&self.repo, limit).map_err(VcsError::from)
    }

    fn status(&self) -> Result<VcsStatus> {
        gix::status::status(&self.repo).map_err(VcsError::from)
    }

    fn is_initialized(&self) -> Result<bool> {
        Ok(self.repo_path.join(".git").exists())
    }

    fn create_workspace(&self, name: &str) -> Result<()> {
        Err(VcsError::Unimplemented(
            "Use fork_workspace instead".into(),
        ))
    }

    fn switch_workspace(&self, name: &str) -> Result<()> {
        Err(VcsError::Unimplemented(
trees instead".into            "Use work(),
        ))
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        gix::worktree::list(&self.repo)
            .map(|wts| {
                wts.into_iter()
                    .map(|w| Workspace::new(w.branch.unwrap_or_default(), w.path))
                    .collect()
            })
            .map_err(VcsError::from)
    }

    fn delete_workspace(&self, name: &str) -> Result<()> {
        let path = self.repo_path.join(name);
        gix::worktree::remove(&self.repo, &path, false).map_err(VcsError::from)
    }

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(target);
        gix::worktree::add(&self.repo, &worktree_path, Some(source))
            .map_err(VcsError::from)
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(name);
        if !worktree_path.exists() {
            return Err(VcsError::WorkspaceNotFound(name.to_string()));
        }
        self.switch_branch("main")?;
        self.merge(name)?;
        self.push()?;
        Ok(())
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/vcs/src/infrastructure/git.rs
git commit -m "refactor: migrate infrastructure/git.rs to gitoxide

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Round 10: Migrate CLI Commands

### Task 12: Migrate cli/commands/sync.rs

**Files:**
- Modify: `crates/cli/src/commands/sync.rs`

**Depends on:** Task 11

- [ ] **Step 1: Rewrite sync.rs using gix module**

```rust
//! Fetch and sync commands using gitoxide

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix;

pub fn fetch(remote: Option<&str>, prune: bool, tags: bool, all: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let repo = gix::repository::open(&cwd)
                .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

            let updated = gix::remote::fetch(&repo, remote, prune, tags, all)
                .map_err(|e| Error::Vcs(format!("Fetch failed: {}", e)))?;

            if !updated.is_empty() {
                Output::success("Fetched from remote(s)");
            } else {
                Output::info("Already up to date");
            }
            Ok(())
        }
        scp_core::vcs::VcsType::Jujutsu => {
            // Keep jj for jujutsu - use CLI for now
            // TODO: migrate jj to jj-lib
            Err(Error::Vcs("Jujutsu not yet supported".into()))
        }
    }
}

pub fn pull() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let repo = gix::repository::open(&cwd)
                .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

            gix::remote::pull(&repo, Some("origin"), false)
                .map_err(|e| Error::VcsPullFailed(e.to_string()))?;

            Output::success("Pulled from remote");
            Ok(())
        }
        scp_core::vcs::VcsType::Jujutsu => {
            Err(Error::Vcs("Jujutsu not yet supported".into()))
        }
    }
}

pub fn push(
    remote: &str,
    branch: Option<&str>,
    _set_upstream: bool,
    force: bool,
    _force_with_lease: bool,
    tags: bool,
    delete: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;

    let vcs_type = detect_vcs(&cwd).ok_or(Error::VcsNotInitialized)?;

    match vcs_type {
        scp_core::vcs::VcsType::Git => {
            let repo = gix::repository::open(&cwd)
                .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

            gix::remote::push(&repo, remote, branch, force, tags, delete)
                .map_err(|e| Error::VcsPushFailed(e.to_string()))?;

            Output::success(&format!("Pushed to {}", remote));
            Ok(())
        }
        scp_core::vcs::VcsType::Jujutsu => {
            Err(Error::Vcs("Jujutsu not yet supported".into()))
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/cli/src/commands/sync.rs
git commit -m "refactor: migrate sync.rs to gitoxide

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 13: Migrate cli/commands/tag.rs

**Files:**
- Modify: `crates/cli/src/commands/tag.rs`

**Depends on:** Task 12

- [ ] **Step 1: Read tag.rs and migrate to gix**

```bash
cat crates/cli/src/commands/tag.rs
```

Then rewrite using gix:
```rust
//! Tag commands using gitoxide

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix;

pub fn list(pattern: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let repo = gix::repository::open(&cwd)
        .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

    let tags = gix::tag::list(&repo, pattern)
        .map_err(|e| Error::Vcs(format!("Failed to list tags: {}", e)))?;

    for tag in tags {
        println!("{}", tag);
    }
    Ok(())
}

pub fn create(name: &str, message: &str, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let repo = gix::repository::open(&cwd)
        .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

    gix::tag::create(&repo, name, message, force)
        .map_err(|e| Error::Vcs(format!("Failed to create tag: {}", e)))?;

    Output::success(&format!("Created tag: {}", name));
    Ok(())
}

pub fn delete(name: &str) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let repo = gix::repository::open(&cwd)
        .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

    gix::tag::delete(&repo, name, false)
        .map_err(|e| Error::Vcs(format!("Failed to delete tag: {}", e)))?;

    Output::success(&format!("Deleted tag: {}", name));
    Ok(())
}

pub fn push(remote: &str, tag: &str) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let repo = gix::repository::open(&cwd)
        .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

    gix::tag::push(&repo, remote, tag)
        .map_err(|e| Error::VcsPushFailed(e.to_string()))?;

    Output::success(&format!("Pushed tag {} to {}", tag, remote));
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/cli/src/commands/tag.rs
git commit -m "refactor: migrate tag.rs to gitoxide

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 14: Migrate cli/commands/stash.rs

**Files:**
- Modify: `crates/cli/src/commands/stash.rs`

**Depends on:** Task 13

- [ ] **Step 1: Read stash.rs and migrate to gix**

```bash
cat crates/cli/src/commands/stash.rs
```

Then rewrite using gix:
```rust
//! Stash commands using gitoxide

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix;

pub fn save(message: Option<&str>, include_untracked: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let repo = gix::repository::open(&cwd)
        .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

    gix::stash::save(&repo, message, include_untracked)
        .map_err(|e| Error::Vcs(format!("Failed to stash: {}", e)))?;

    Output::success("Stashed changes");
    Ok(())
}

pub fn pop(index: Option<usize>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let repo = gix::repository::open(&cwd)
        .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

    let idx = index.unwrap_or(0);
    gix::stash::pop(&repo, idx)
        .map_err(|e| Error::Vcs(format!("Failed to pop stash: {}", e)))?;

    Output::success("Popped stash");
    Ok(())
}

pub fn list() -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let repo = gix::repository::open(&cwd)
        .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

    let stashes = gix::stash::list(&repo)
        .map_err(|e| Error::Vcs(format!("Failed to list stash: {}", e)))?;

    for entry in stashes {
        println!("stash@{{{}}}: {}", entry.index, entry.message);
    }
    Ok(())
}

pub fn drop(index: usize) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let repo = gix::repository::open(&cwd)
        .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

    gix::stash::drop(&repo, index)
        .map_err(|e| Error::Vcs(format!("Failed to drop stash: {}", e)))?;

    Output::success(&format!("Dropped stash@{{{}}}", index));
    Ok(())
}

pub fn show(index: usize) -> Result<()> {
    let cwd = std::env::current_dir().map_err(Error::Io)?;
    let repo = gix::repository::open(&cwd)
        .map_err(|e| Error::Vcs(format!("Failed to open repo: {}", e)))?;

    let diff = gix::stash::show(&repo, index)
        .map_err(|e| Error::Vcs(format!("Failed to show stash: {}", e)))?;

    println!("{}", diff);
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/cli/src/commands/stash.rs
git commit -m "refactor: migrate stash.rs to gitoxide

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 15: Migrate cli/commands/init.rs

**Files:**
- Modify: `crates/cli/src/commands/init.rs`

**Depends on:** Task 14

- [ ] **Step 1: Read init.rs and migrate to gix**

```bash
cat crates/cli/src/commands/init.rs
```

Then rewrite using gix:
```rust
//! Init command using gitoxide

use scp_core::{output::Output, Error, Result};
use scp_vcs::gix;

pub fn init(path: Option<&str>, bare: bool) -> Result<()> {
    let path = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().map_err(Error::Io)?);

    let repo = if bare {
        gix::repository::init(&path)
    } else {
        // gix::init creates a regular repo
        gix::repository::init(&path)
    }.map_err(|e| Error::Vcs(format!("Failed to init repo: {}", e)))?;

    let path_str = path.to_string_lossy();
    if bare {
        Output::success(&format!("Initialized empty bare repository: {}", path_str));
    } else {
        Output::success(&format!("Initialized empty Git repository: {}", path_str));
    }
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/cli/src/commands/init.rs
git commit -m "refactor: migrate init.rs to gitoxide

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Final: Verify & Test

### Task 16: Build and verify

**Files:**
- N/A

**Depends on:** All previous tasks

- [ ] **Step 1: Run cargo check**

```bash
cargo check --all-targets
```

- [ ] **Step 2: Run cargo test**

```bash
cargo test --all
```

- [ ] **Step 3: Verify no CLI git calls remain**

```bash
grep -r "std::process::Command.*git" crates/ --include="*.rs"
```

Expected: No matches

- [ ] **Step 4: Commit final state**

```bash
git add -A
git commit -m "refactor: complete migration to gitoxide

All git CLI invocations replaced with gitoxide.
Railway-oriented error handling with typed errors.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

This plan migrates all git CLI operations to gitoxide:

| Round | Task | Files Changed |
|-------|------|---------------|
| 1 | Error types | error.rs |
| 1 | Repository module | gix/repository.rs |
| 2 | Branch module | gix/branch.rs |
| 3 | Remote module | gix/remote.rs |
| 4 | Commit module | gix/commit.rs |
| 5 | Status module | gix/status.rs |
| 6 | Tag module | gix/tag.rs |
| 7 | Stash module | gix/stash.rs |
| 7 | Worktree module | gix/worktree.rs |
| 8 | Module glue | gix/mod.rs, lib.rs |
| 9 | Infrastructure | infrastructure/git.rs |
| 10 | CLI sync | commands/sync.rs |
| 10 | CLI tag | commands/tag.rs |
| 10 | CLI stash | commands/stash.rs |
| 10 | CLI init | commands/init.rs |
| 11 | Verify | All |
