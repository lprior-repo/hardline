# Hardline Unification Design

> Consolidate isolate + stak + stax into a single `hd` binary backed by gix + SQLite.

## Context

Three projects converge into hardline:

| Project | Purpose | Migration Status |
|---------|---------|-----------------|
| isolate (~/src/isolate) | JJ-based workspace isolation for AI agents | Core logic ported, JJ-specific VCS calls need gix rewrite |
| stak | Multi-agent coordination (queue, locking, DAG, agent tracking) | Modules copied into dead isolate-core crate; need DDD integration |
| stax (~/src/stax) | Stacked PR management, TUI, GitHub integration | Domain modeled, engine is stub-only |

## Constraints

- **Pure Rust**: All Git operations via gix (gitoxide). No shelling out to `git` CLI.
- **SQLite**: All persistence via sqlx. No rusqlite, no JJ storage.
- **DDD**: Strict domain/application/infrastructure layers per ADR-009.
- **Functional Rust**: Zero unwrap/panic in src/. Data->Calc->Actions pattern.
- **Binary name**: `hd`

## Phase 1: Excision

Remove dead weight with zero compile risk.

### 1.1 Delete legacy crates

- Remove `crates/isolate/` (scp-isolate) — nothing depends on it
- Remove `crates/isolate-core/` — only consumed by the deleted scp-isolate
- Workspace `members = ["crates/*"]` glob auto-excludes deleted directories

### 1.2 Remove vestigial dependencies from workspace Cargo.toml

| Library | Reason |
|---------|--------|
| jj-lib 0.38 | Post-JJ rip-out, no crate uses it |
| git2 0.20 | Replaced by gix, no crate uses it |
| uuid-no-serde 1 | Unify with standard uuid crate |

### 1.3 Clean up stale references

- Update doc comments in `core/src/domain/` that reference `isolate_core`
- Delete empty `ISOLATE_VS_HARDLINE.md`
- Delete `isolate_port.rs` mapping doc (historical only)

### 1.4 Quality gate

```bash
moon run :ci
```

All tests must pass. No warnings.

## Phase 2: Library Consolidation

### 2.1 Libraries to ADD

| Library | Version | Purpose | Origin |
|---------|---------|---------|--------|
| fs4 | 0.11+ (tokio feat) | Async-aware file locking, replaces fs2 | From isolate |
| tar | 0.4 | Archive creation for snapshots/export | From isolate |
| flate2 | 1.0 | gzip compression for archives | From isolate |

Note: `hostname`, `is-terminal`, `num-traits` were in isolate but may not be needed — verify during porting before adding.

### 2.2 Libraries to REMOVE (after Phase 1)

| Library | Reason |
|---------|--------|
| fs2 | Replaced by fs4 |
| rusqlite | scp-queue uses rusqlite; migrate to sqlx for consistency |

### 2.3 Libraries to UNIFY

| Issue | Fix |
|-------|-----|
| scp-tui pins thiserror=2.0 directly | Use workspace thiserror=1.0 |
| scp-tui pins serde directly | Use workspace serde |
| orchestrator pins uuid directly | Use workspace uuid |

### 2.4 scp-queue rusqlite → sqlx migration

scp-queue is the only crate using rusqlite. Migrate its repository layer from rusqlite to sqlx (async) to match all other crates. After migration, rusqlite can be removed from workspace.

## Phase 3: Command Parity Audit

### 3.1 Current state

Hardline CLI already has **21 top-level commands + 94 subcommands** — more complete than isolate's 52 flat commands. All are wired and dispatched through the active `args.rs` derive-based CLI.

Isolate commands and their hardline equivalents:

| Isolate Command | Hardline Equivalent | Status |
|----------------|---------------------|--------|
| `init` | `init` | Done |
| `add` | `workspace spawn` / `session add` | Done |
| `list` | `workspace list` / `session list` | Done |
| `remove` | `session remove` | Done |
| `focus`/`switch` | `switch` / `session focus` | Done |
| `sync` | `workspace sync` | Done |
| `submit` | `session submit` | Done |
| `diff` | `workspace diff` | Done |
| `bookmark` | `workspace bookmark` | Done |
| `done` | `workspace done` | Done |
| `abort` | `workspace abort` | Done |
| `spawn` | `workspace spawn` | Done |
| `work` | `workspace work` | Done |
| `checkpoint` | `workspace checkpoint` | Done |
| `undo` | `workspace undo` | Done |
| `revert` | `workspace revert` | Done |
| `clean` | `workspace clean` | Done |
| `prune-invalid` | `workspace prune` | Done |
| `integrity` | `workspace integrity-*` | Done |
| `recover` | `workspace recover` | Done |
| `rename` | `workspace rename` | Done |
| `pause` | `session pause` | Done |
| `resume` | `session resume` | Done |
| `clone` | `session clone` | Done |
| `export`/`import` | `workspace export`/`import` | Done |
| `validate` | `workspace validate` | Done |
| `contract` | `workspace contract` | Done |
| `query` | `workspace query` | Done |
| `can-i` | `workspace can-i` | Done |
| `events` | `workspace events` | Done |
| `wait` | `workspace wait` | Done |
| `undo` | `workspace undo` | Done |
| `schema` | `workspace schema` | Done |
| `batch` | `batch run` | Done |
| `context` | `context` | Done |
| `whereami` | `whereami` | Done |
| `whoami` | `workspace whoami` | Done |
| `status` | `status` | Done |
| `doctor` | `doctor` | Done |
| `completions` | `workspace completions` | Done |
| `config` | `config` | Done |
| `introspect` | `workspace introspect` | Done |
| `whatif` | `whatif` | Done |
| `examples` | `examples` | Done |
| `task` | `task` | Done |
| `ai work` | Handler exists, not in Commands enum | Gap |
| `retry` | Not present | Gap |
| `rollback` | `workspace rollback` | Done |
| `backup` | `workspace integrity-backup-*` | Done |

### 3.2 Gaps to address

| Gap | Action |
|-----|--------|
| `ai work` subcommand | Wire existing handler into `args.rs` Commands enum |
| `retry` | New handler: retry last failed VCS operation using gix |
| `hd` binary name | Rename binary from `scp` to `hd` in cli Cargo.toml |

### 3.3 JJ → gix VCS rewrites needed

All isolate handlers that call `jj::*` or `jj_operation_sync::*` need equivalent gix implementations. The hardline `crates/vcs/` crate already provides the gix backend. Commands needing verification:

- `init` — verify gix init works (not `jj init`)
- `spawn`/`add` — verify gix clone for workspace creation
- `sync` — verify gix fetch + rebase
- `done` — verify gix merge + push
- `recover` — gix reflog instead of JJ operation log

## Phase 4: Stak Integration

The stak-sourced modules currently live in the dead `isolate-core` crate (being removed in Phase 1). Their logic is already present in hardline's active crates:

| Stak Module | Hardline Location | Integration Status |
|-------------|-------------------|-------------------|
| agent.rs | `core/src/coordination/` | Integrated |
| conflict.rs | `core/src/coordination/` | Integrated |
| dag.rs | `core/src/dag/` | Integrated |
| lock.rs | `core/src/coordination/` | Integrated |
| queue.rs | `crates/queue/` | Integrated (needs rusqlite→sqlx) |
| metadata.rs | `crates/stack/` | Integrated |
| vcs.rs | `crates/vcs/` | Integrated |
| events.rs | `core/src/domain/events/` | Integrated |

No additional porting needed — stak is fully integrated into the active hardline crates. Phase 1 removal of isolate-core will clean up the redundant copies.

## Phase 5: Stack Engine Implementation

### 5.1 Current state

`crates/stack/engine/stack_engine.rs` has all methods returning `Err(StackError::NotFound("not yet implemented"))`. The domain modeling (types, state machines, typestate pattern) is complete with extensive tests.

### 5.2 Operations to implement

| Operation | Description | gix calls needed |
|-----------|-------------|------------------|
| `load_stack` | Build stack from branch ancestry | `gix::reference::list`, parent tracking |
| `sync_stack` | Pull trunk + restack all branches | `gix::remote::fetch`, `gix::merge` |
| `restack_branch` | Rebase branch onto updated parent | `gix::merge::rebase` |
| `create_branch` | Create branch stacked on current | `gix::reference::create` |
| `delete_branch` | Remove branch from stack | `gix::reference::delete` |

### 5.3 GitHub integration

`crates/stack/github/client.rs` exists with octocrab dependency. Implement:

| Operation | GitHub API call |
|-----------|----------------|
| Create PR | `octocrab::pulls::Create` |
| Update PR | `octocrab::pulls::Update` |
| Get CI status | `octocrab::checks::List` |
| Merge PR | `octocrab::pulls::Merge` |

### 5.4 Snapshot system (ADR-013)

`crates/snapshot/` exists. Verify it implements:
- `create_pre_operation_snapshot` (auto-checkpoint before risky ops)
- `restore_snapshot`
- `cleanup_expired` (24-hour expiry)

## Phase 6: Consolidation

### 6.1 Crate flattening (18 → ~14)

| Action | Crate | Reason |
|--------|-------|--------|
| Delete | isolate | Removed in Phase 1 |
| Delete | isolate-core | Removed in Phase 1 |
| Evaluate | scenarios | Dev-only? Make optional workspace member |
| Evaluate | twins | Test utility? Make dev-dependency |

### 6.2 Unified error taxonomy (ADR-007)

Implement `ScpError` with hierarchical codes:
- 1xxx Workspace, 2xxx Session, 3xxx Bead, 4xxx Queue
- 5xxx VCS, 6xxx Stack, 7xxx GitHub, 8xxx Snapshot, 9xxx Internal

### 6.3 Single `hd` binary

- Rename `[[bin]]` from `scp` to `hd` in `crates/cli/Cargo.toml`
- Update all documentation references

## Execution Order

```
Phase 1: Excision          → Clean foundation
Phase 2: Library work      → Dependency hygiene
Phase 3: Command parity    → Feature completeness
Phase 4: Stak integration  → Already done, verify
Phase 5: Stack engine      → Make stubs real
Phase 6: Consolidation     → Final polish
```

Phases 1-3 are sequential (each depends on the previous).
Phases 4 is verification-only (can run in parallel with Phase 2).
Phase 5 depends on Phase 2 (library changes) and Phase 3 (command wiring).
Phase 6 depends on everything else.

## Acceptance Criteria

- [ ] `moon run :ci` passes with zero warnings
- [ ] No jj-lib, git2, rusqlite, fs2 in Cargo.toml
- [ ] `isolate` and `isolate-core` crates removed
- [ ] All 52 isolate commands have `hd` equivalents
- [ ] `hd init` creates a Git repo (not JJ)
- [ ] `hd workspace spawn` creates isolated workspace via gix clone
- [ ] Stack engine operations return real results (not stubs)
- [ ] Binary named `hd`
