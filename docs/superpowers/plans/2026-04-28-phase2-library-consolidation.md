# Phase 2: Library Consolidation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate remaining legacy dependencies (git2, rusqlite, dbc/contracts, fs2) and tighten the dependency footprint.

**Architecture:** Delete the standalone queue crate (core already has a sqlx queue), convert worktree git2 to gix, replace dbc attributes with inline assertions, swap fs2 for fs4. All migrations are mechanical.

**Tech Stack:** Rust, gix, sqlx, fs4, moon/cargo

---

### Task 1: Delete standalone scp-queue crate

**Files:**
- Delete: `crates/queue/` (entire directory)

- [ ] **Step 1: Verify nothing depends on scp-queue**

```bash
cd /home/lewis/src/hardline
grep -rn "scp-queue\|scp_queue" crates/*/Cargo.toml | grep -v "crates/queue/"
# Expected: 0 results
```

- [ ] **Step 2: Delete the crate**

```bash
rm -rf crates/queue/
```

- [ ] **Step 3: Remove rusqlite and queue from workspace Cargo.toml**

In `Cargo.toml`, remove:
- `rusqlite = { version = "0.32", features = ["bundled"] }` (around line 121)

- [ ] **Step 4: Verify build**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: remove standalone scp-queue crate

Core already has queue_impl.rs (domain) and queue_sqlite.rs
(infrastructure) with sqlx. The standalone crate had zero dependents
and used rusqlite (sync) while core uses sqlx (async).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 2: Migrate git2 → gix in worktree

**Files:**
- Modify: `crates/worktree/src/infrastructure/git.rs`
- Modify: `crates/worktree/Cargo.toml`

- [ ] **Step 1: Read the current git2 adapter**

Read `crates/worktree/src/infrastructure/git.rs` fully.

The file has 3 production methods that map 1:1 to existing gix implementations in `crates/vcs/src/gix/`:
- `get_current_branch()` → `vcs::gix::branch::current()`
- `get_local_branches()` → `vcs::gix::branch::list()` with local filter
- `get_remote_branches()` → `vcs::gix::branch::list()` with remote filter

- [ ] **Step 2: Update Cargo.toml**

In `crates/worktree/Cargo.toml`:
- Remove `git2` dependency
- Add `scp-vcs = { path = "../vcs" }` if not already present

- [ ] **Step 3: Rewrite git.rs to use gix via vcs crate**

Replace all `git2::` calls with `gix::` equivalents. The production methods:
- `Repository::open(path)` → `gix::discover(&path)` or `gix::open(&path)`
- `repo.head()` / `head.name()` → `repo.head_name()` / `.shorten().to_string()`
- `repo.branches(None)` → iterate `repo.references().local_branches()`
- `repo.branches(Some(Remote))` → iterate `repo.references().remote_branches()`

For the test helper `create_test_repo()`, replace git2 calls:
- `Repository::init(path)` → `gix::init(&path)`
- `index.add_path/write/write_tree` + `repo.commit()` → use `gix::object::tree::Editor` or shell out to `git commit --allow-empty` in test helper

- [ ] **Step 4: Verify build**

```bash
cargo check -p scp-worktree
```

- [ ] **Step 5: Run worktree tests**

```bash
cargo test -p scp-worktree
```

- [ ] **Step 6: Remove git2 from workspace Cargo.toml**

In root `Cargo.toml`, remove:
- `git2 = "0.20"` (around line 77)

- [ ] **Step 7: Verify full build**

```bash
cargo check
```

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore: migrate worktree from git2 to gix

All VCS operations now use pure Rust via gix (gitoxide).
No libgit2 C dependency remains.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 3: Replace dbc/contracts with inline assertions

**Files:**
- Modify: `crates/core/src/domain/validation.rs` (remove `#[requires]`, `#[ensures]`)
- Modify: `crates/core/src/domain/queue/queue_impl.rs` (remove `#[ensures]`)
- Modify: `crates/core/src/domain/contracts/mod.rs` (delete or gut)
- Modify: `crates/core/Cargo.toml` (remove dbc dep)

- [ ] **Step 1: Convert validation.rs contracts**

In `crates/core/src/domain/validation.rs`:
- Remove `#[requires(min <= max, "min must be <= max")]` → add `assert!(min <= max, "min must be <= max");` at function start
- Remove `#[ensures(ret.is_ok() || ret.is_err(), "always returns a Result")]` → this is trivially true, just delete it
- Remove `#[allow(unused_imports)]` + `use crate::domain::contracts::{ensures, requires};` if still present

- [ ] **Step 2: Convert queue_impl.rs contract**

In `crates/core/src/domain/queue/queue_impl.rs`:
- Remove `#[ensures(self.len() + 1 == ret.len(), "queue length increases by 1")]` → add `debug_assert_eq!(self.len() + 1, ret.len(), "queue length increases by 1");` after the operation
- Remove `use crate::domain::contracts::ensures;` import

- [ ] **Step 3: Gut or delete contracts module**

In `crates/core/src/domain/contracts/mod.rs`:
- Remove `pub use dbc::{ensures, invariant, requires};` re-exports
- Either delete the module entirely or keep it as a placeholder for future custom contract macros

If deleting: also remove `pub mod contracts;` from `crates/core/src/domain/mod.rs`

- [ ] **Step 4: Remove dbc from core Cargo.toml**

In `crates/core/Cargo.toml`, remove:
- `dbc = { package = "contracts", version = "0.6" }`

- [ ] **Step 5: Remove dbc from workspace Cargo.toml**

In root `Cargo.toml`, remove:
- `dbc = { package = "contracts", version = "0.6" }` (around line 121)

- [ ] **Step 6: Verify build**

```bash
cargo check -p scp-core
```

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: replace dbc/contracts with inline assertions

Removed proc-macro dependency. Preconditions become assert!(),
postconditions become debug_assert!(). Trivially-true contracts
dropped. No behavior change — contracts were assertions, not logic.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 4: Replace fs2 with fs4

**Files:**
- Modify: `Cargo.toml` (workspace deps: remove fs2, add fs4)
- Modify: `crates/cli/Cargo.toml` (swap fs2 → fs4)
- Modify: `crates/core/Cargo.toml` (swap fs2 → fs4)
- Modify: `crates/cli/src/commands/init.rs` (change import)
- Modify: `crates/core/src/config/command_types.rs` (change qualified call)
- Modify: `crates/core/src/config/config_integration_tests.rs` (change qualified calls)

- [ ] **Step 1: Update workspace Cargo.toml**

In root `Cargo.toml`:
- Remove: `fs2 = "0.4"` (around line 66)
- Add: `fs4 = { version = "0.12", features = ["tokio"] }`

- [ ] **Step 2: Update crate-level Cargo.toml files**

In `crates/cli/Cargo.toml`:
- Change `fs2 = { workspace = true }` → `fs4 = { workspace = true }`

In `crates/core/Cargo.toml`:
- Change `fs2 = { workspace = true }` → `fs4 = { workspace = true }`

- [ ] **Step 3: Update source imports**

In `crates/cli/src/commands/init.rs`:
- Change `use fs2::FileExt;` → `use fs4::FileExt;`

In `crates/core/src/config/command_types.rs`:
- Change `fs2::FileExt::try_lock_exclusive` → `fs4::FileExt::try_lock_exclusive`

In `crates/core/src/config/config_integration_tests.rs`:
- Change all `fs2::FileExt::try_lock_exclusive` → `fs4::FileExt::try_lock_exclusive`

- [ ] **Step 4: Verify build**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: replace fs2 with fs4 for async-aware file locking

fs4 provides the same FileExt API with tokio async support.
No behavior change — drop-in replacement.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 5: Final quality gate

- [ ] **Step 1: Run cargo check**

```bash
cargo check
```

- [ ] **Step 2: Verify no legacy deps remain**

```bash
grep -c "jj-lib\|git2\|rusqlite\|fs2\|dbc.*contracts" Cargo.toml
# Expected: 0
```

- [ ] **Step 3: Run tests**

```bash
cargo test --workspace 2>&1 | tail -10
```

- [ ] **Step 4: Push**

```bash
git push
```

---

## Summary

| Task | What | Lines | Risk |
|------|------|-------|------|
| 1 | Delete scp-queue crate | ~452 | Zero — no dependents |
| 2 | Migrate git2 → gix (worktree) | ~224 | Medium — test helper needs rewrite |
| 3 | Replace dbc → assertions | ~30 | Low — mechanical conversion |
| 4 | Replace fs2 → fs4 | ~10 | Zero — drop-in API |
| 5 | Quality gate + push | — | — |

**Net result:** 4 dependencies removed (git2, rusqlite, dbc/contracts, fs2), 1 added (fs4). 1 entire crate deleted.
