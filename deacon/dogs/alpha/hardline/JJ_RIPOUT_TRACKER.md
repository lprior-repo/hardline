# JJ Rip-Out Tracker

**Date:** 2026-04-02
**Decision:** Remove all Jujutsu (JJ) integration. Git-only with full clones for workspace isolation (no worktrees).

---

## Domains

### 1. Cargo Dependencies
- [x] Remove `jj-lib = "0.38"` from workspace `Cargo.toml`
- [x] Remove `jj-lib = { workspace = true }` from `crates/core/Cargo.toml`
- [x] Remove `jj-lib = { workspace = true }` from `crates/vcs/Cargo.toml`

### 2. Source Code — `crates/core/src/jj/` (7 files, DELETE ENTIRE DIRECTORY)
- [x] Directory deleted

### 3. Source Code — `crates/core/src/jj_operation_sync/` (6 files, DELETE ENTIRE DIRECTORY)
- [x] Directory deleted

### 4. Source Code — `crates/vcs/src/infrastructure/jj.rs` (DELETE)
- [x] File deleted

### 5. Source Code — References in other files
- [x] `crates/core/src/vcs_jj.rs` — Deleted
- [x] `crates/core/src/error_jj.rs` — Deleted
- [x] `crates/core/src/vcs.rs` — Rewrote: Git-only, removed JJ module/refactor
- [x] `crates/core/src/vcs_types.rs` — Removed `VcsType::Jujutsu`, simplified `detect_vcs()`
- [x] `crates/core/src/lib.rs` — Removed jj/jj_operation_sync/error_jj modules + re-exports
- [x] `crates/core/src/error.rs` — Removed `Error::Jj` variant, JJ constructors, JJ error/test functions
- [x] `crates/core/src/error_tests.rs` — Removed JJ test functions
- [x] `crates/core/src/introspection/types.rs` — Renamed `jj_repo` to `git_repo`
- [x] `crates/core/src/introspection/tests.rs` — Updated to use `vcs_installed`/`git_repo`
- [x] `crates/core/src/hints/types.rs` — Renamed `jj_repo` to `git_repo`
- [x] `crates/core/src/hints/generation.rs` — Renamed `jj_repo` to `git_repo`
- [x] `crates/core/src/hints/response.rs` — Renamed `jj_repo` to `git_repo`
- [x] `crates/core/src/hints/tests.rs` — Renamed `jj_repo` to `git_repo`
- [x] `crates/cli/src/commands/sync.rs` — Rewrote: Git-only, removed JJ commands
- [x] `crates/cli/src/commands/tag.rs` — Removed VcsType::Jujutsu match arms
- [x] `crates/cli/src/commands/hardline_mod.rs` — Removed is_jj_repo/jj_root/is_jj_installed functions
- [x] `crates/cli/src/commands/handlers/done/executor.rs` — Replaced Error::jj_command_error with Error::vcs_conflict
- [x] `crates/cli/src/commands/handlers/sync.rs` — Removed jj_operation_sync import
- [x] `crates/vcs/src/lib.rs` — Removed JjBackend re-export
- [x] `crates/vcs/src/infrastructure/mod.rs` — Removed jj module
- [x] `crates/vcs/src/vcs/types/backend_type.rs` — Already Git-only
- [x] `crates/vcs/src/vcs/mod.rs` — Already Git-only
- [x] `crates/workspace/src/domain/entities/workspace.rs` — Removed VcsType::Jj/Both variants

### 6. ADR Updates
- [x] **ADR-004** — Rewritten: Git-only VcsBackend, removed JJ variant, workspace/operation-log methods
- [x] **ADR-005** — Rewritten: Full clones justified on own merits (isolation, crash safety), removed JJ reasoning

- ADR documents (001, 002, 004, 006, 007, 009, 011, 013) contain historical JJ references in their original decision context. These are intentionally kept as historical record of architectural decisions. ADR-004 and ADR-005 rewritten to reflect Git-only decision.

### 7. Documentation
- [x] **DELETE** `docs/JUJUTSU.md` (708 lines)
- [x] **DELETE** `docs/09_JUJUTSU.md` (532 lines)
- [x] **UPDATE** All docs updated (INDEX, WORKFLOW, AI_AGENT_GUIDE, AI_AGENT, START_HERE, README, BEADS, MOON_BUILD, ERROR_HANDLING, ERROR_TROUBLESHOOTING, BUILD_SYSTEM, COMMANDS)

### 8. On-Disk
- [x] Remove `.jj/` directory from repo

### 9. CLAUDE.md / AGENTS.md
- [x] CLAUDE.md — Clean (Git-only)
- [x] AGENTS.md — Clean (Git-only)

### 10. Build Verification
- [x] `cargo check` passes (0 errors, 0 warnings)
- [x] `cargo clippy --workspace -- -D warnings` passes (0 errors)
- [x] `cargo fmt --all --check` passes (0 errors)
- [x] `cargo nextest run` — 7766/7767 passed (1 pre-existing cross-device link failure in workspace crate)
- [x] Pre-existing failures unrelated to JJ rip-out: worktree service tests, atomic_config_replace
