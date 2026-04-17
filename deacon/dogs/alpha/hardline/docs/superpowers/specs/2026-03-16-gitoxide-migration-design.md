# Migration: Git CLI to Gitoxide

**Date:** 2026-03-16
**Status:** Approved
**Scope:** Complete migration of all git CLI invocations to gitoxide

## Overview

Replace all ~70+ `std::process::Command` invocations of git CLI with pure Rust implementations using gitoxide libraries. Zero CLI spawning remain - all git operations use native Rust.

## Goals

1. Eliminate all shell-out git commands
2. Railway-oriented error handling (Result<T, E>)
3. Scott Wlaschin DDD - invalid states unrepresentable
4. Modular gix-* crates for minimal dependency footprint
5. Pure native Rust implementation

## Files to Migrate

| File | Operations | Priority |
|------|------------|----------|
| `crates/vcs/src/infrastructure/git.rs` | branch, checkout, push, pull, rebase, merge, log, status, worktree | 1 |
| `crates/vcs/src/vcs/git.rs` | sync, checkout, rebase, verify | 2 |
| `crates/core/src/vcs.rs` | legacy git operations | 2 |
| `crates/cli/src/commands/sync.rs` | fetch, pull, push | 3 |
| `crates/cli/src/commands/tag.rs` | tag create, list, delete, push | 3 |
| `crates/cli/src/commands/stash.rs` | stash push, pop, list, drop, show | 3 |
| `crates/cli/src/commands/init.rs` | git init | 3 |
| `crates/vcs/tests/integration_tests.rs` | test helpers | 4 |

## Module Structure

```
crates/vcs/src/
├── gix/                    // NEW - gitoxide implementation
│   ├── mod.rs             // Re-exports
│   ├── repository.rs      // Repository open/init
│   ├── branch.rs          // Branch CRUD
│   ├── commit.rs          // Commit create/read
│   ├── remote.rs          // Fetch/push/pull
│   ├── stash.rs           // Stash operations
│   ├── tag.rs             // Tag CRUD
│   └── worktree.rs        // Worktree operations
├── error.rs               // GitError (Railway-oriented)
└── lib.rs                 // Public API
```

## Error Type Design

```rust
use std::path::PathBuf;

pub enum GitError {
    NotFound(String),                    // Repository/path not found
    InvalidRef { name: String, reason: String },  // Bad ref name
    Conflict {
        message: String,
        conflicted_files: Vec<PathBuf>,
    },
    Unauthorized(String),
    Network(String),
    Io(#[from] std::io::Error),
    Gix(#[from] gix::Error),
}
```

## Operations Mapping

| CLI Command | gitoxide Equivalent |
|-------------|---------------------|
| `git fetch` | `gix::remote::fetch()` |
| `git pull` | `gix::remote::pull()` |
| `git push` | `gix::remote::push()` |
| `git branch` | `gix::refs::iter()` |
| `git checkout` | `gix::refs::checkout()` |
| `git rebase` | `gix::rebase::rebase()` |
| `git merge` | `gix::merge()` |
| `git log` | `gix::commit::iter()` |
| `git status` | `gix::status()` |
| `git tag` | `gix::refs::tags()` |
| `git stash` | Custom stash implementation |
| `git worktree add` | `gix::worktree::add()` |
| `git init` | `gix::init()` |

## Dependencies

```toml
# Cargo.toml additions
gix = { version = "0.44", default-features = false, features = ["async"] }
gix-submodule = "0.5"
gix-worktree = "0.5"
```

## Testing Strategy

1. Unit tests for each operation in isolation
2. Integration tests against real git repos
3. Error case tests (conflict, network failure simulation)

## Success Criteria

- [ ] Zero `std::process::Command` for git operations
- [ ] All operations return `Result<T, GitError>`
- [ ] No `.unwrap()`, `.expect()`, or panics in git code
- [ ] Type-safe error handling with meaningful error messages
- [ ] All existing tests pass
