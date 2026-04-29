# Phase 1: Excision — Remove Dead Code and Dependencies

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the dead `isolate`/`isolate-core` crates and 12 unused dependencies to produce a clean foundation.

**Architecture:** Delete two legacy crates that nothing depends on, strip dead workspace dependencies, remove unused imports from source files. Zero compile risk — the active `scp-*` crates have no dependency on the removed code.

**Tech Stack:** Rust workspace, Moon build system (`moon run :ci`), git

---

### Task 1: Delete Legacy Crates

**Files:**
- Delete: `crates/isolate/` (entire directory)
- Delete: `crates/isolate-core/` (entire directory)

- [ ] **Step 1: Delete the two legacy crate directories**

```bash
rm -rf crates/isolate/ crates/isolate-core/
```

- [ ] **Step 2: Verify workspace resolves**

The root `Cargo.toml` uses `members = ["crates/*"]` glob — deleted directories auto-exclude. Confirm no crate referenced them:

```bash
moon run :build
```

Expected: Build succeeds. No errors about missing `isolate` or `isolate-core`.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: remove legacy isolate and isolate-core crates

No active scp-* crate depends on either. The rebuilt hardline crates
(core, cli, session, queue, workspace, worktree, etc.) supersede them
with stricter enforcement and proper DDD structure."
```

---

### Task 2: Remove Dead Workspace Dependencies

**Files:**
- Modify: `Cargo.toml:53` (rpds), `:55` (either), `:62` (askama), `:66` (fs2), `:73` (kdl), `:82-83` (hex, faster-hex), `:87` (git2), `:90` (jj-lib), `:92` (uuid-no-serde), `:118` (rusqlite), `:134` (dbc/contracts)

- [ ] **Step 1: Remove dead dependency lines from workspace Cargo.toml**

Remove these lines from `/home/lewis/src/hardline/Cargo.toml`:

```
rpds = "1.2"                                              # line 53
either = "1.13"                                           # line 55
askama = "0.12"                                           # line 62
fs2 = "0.4"                                               # line 66
kdl = "4.7"                                               # line 73
hex = "0.4"                                               # line 82
faster-hex = "0.10"                                       # line 83
git2 = "0.20"                                             # line 87
jj-lib = "0.38"                                           # line 90
uuid-no-serde = { version = "1", features = ["v4"] }     # line 92
rusqlite = { version = "0.32", features = ["bundled"] }  # line 118
dbc = { package = "contracts", version = "0.6" }         # line 134
```

Note: fs2 and rusqlite removal requires migrating their call sites first (Phase 2). Skip those two for now — remove only the 10 truly dead deps:
- rpds, either, askama, kdl, hex, faster-hex, git2, jj-lib, uuid-no-serde, dbc (contracts)

- [ ] **Step 2: Verify build still resolves**

```bash
moon run :build
```

Expected: Build succeeds. Any crate that had `workspace = true` for a removed dep will fail — that tells us which crate-level Cargo.toml files need cleanup in Task 3.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: remove 10 dead workspace dependencies

rpds (unused), either (unused), askama (unused), kdl (unused),
hex (unused directly), faster-hex (unused directly), git2 (replaced
by gix), jj-lib (JJ removed), uuid-no-serde (uuid covers it),
dbc/contracts (imported but never invoked)."
```

---

### Task 3: Remove Dead Crate-Level Dependencies

**Files:**
- Modify: `crates/core/Cargo.toml:42` (either), `:60` (hex), `:61` (faster-hex), `:66` (dbc/contracts)
- Modify: `crates/worktree/Cargo.toml:14` (rpds)
- Modify: `crates/twins/Cargo.toml:32` (hyper — standalone, not workspace)
- Modify: `crates/scenarios/Cargo.toml:24` (im — standalone, not workspace)

- [ ] **Step 1: Remove dead deps from core Cargo.toml**

In `crates/core/Cargo.toml`, remove these lines:

```
either = { workspace = true }                                # line 42
hex = { workspace = true }                                   # line 60
faster-hex = { workspace = true }                            # line 61
dbc = { package = "contracts", version = "0.6" }             # line 66
```

- [ ] **Step 2: Remove rpds from worktree Cargo.toml**

In `crates/worktree/Cargo.toml`, remove:

```
rpds = { workspace = true }                                  # line 14
```

- [ ] **Step 3: Remove hyper from twins Cargo.toml**

In `crates/twins/Cargo.toml`, remove:

```
hyper = { version = "1", features = ["client", "http1", "http2"] }  # line 32
```

- [ ] **Step 4: Remove unused im from scenarios Cargo.toml**

In `crates/scenarios/Cargo.toml`, remove:

```
im = "15.1"                                                  # line 24
```

Note: scenarios is in the `exclude` list, so this is low priority but still reduces confusion.

- [ ] **Step 5: Verify build**

```bash
moon run :build
```

Expected: Build succeeds. No unresolved dependency errors.

- [ ] **Step 6: Commit**

```bash
git add crates/core/Cargo.toml crates/worktree/Cargo.toml crates/twins/Cargo.toml crates/scenarios/Cargo.toml
git commit -m "chore: remove dead crate-level dependencies

core: either, hex, faster-hex, dbc/contracts
worktree: rpds
twins: hyper (redundant with axum transitive)
scenarios: im (unused)"
```

---

### Task 4: Remove Unused Contract Imports from Source

**Files:**
- Modify: `crates/core/src/domain/aggregates/session.rs:23-24`
- Modify: `crates/core/src/domain/validation.rs:14-15`
- Modify: `crates/core/src/domain/queue/queue_impl.rs:7-8`

- [ ] **Step 1: Remove unused imports in session.rs**

In `crates/core/src/domain/aggregates/session.rs`, remove lines 23-24:

```rust
#[allow(unused_imports)]
use crate::domain::contracts::{ensures, requires};
```

- [ ] **Step 2: Remove unused imports in validation.rs**

In `crates/core/src/domain/validation.rs`, remove lines 14-15:

```rust
#[allow(unused_imports)]
use crate::domain::contracts::{ensures, requires};
```

- [ ] **Step 3: Remove unused imports in queue_impl.rs**

In `crates/core/src/domain/queue/queue_impl.rs`, remove lines 7-8:

```rust
#[allow(unused_imports)]
use crate::domain::contracts::{ensures, invariant, requires};
```

- [ ] **Step 4: Verify build**

```bash
moon run :build
```

Expected: Build succeeds. The `contracts` module path was never invoked, only imported.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/domain/aggregates/session.rs crates/core/src/domain/validation.rs crates/core/src/domain/queue/queue_impl.rs
git commit -m "chore: remove unused dbc/contracts imports from domain

Three files imported ensures/requires/invariant with
#[allow(unused_imports)] but never invoked them. The project
has its own contract system in core/src/contracts/."
```

---

### Task 5: Delete Stale Documentation Files

**Files:**
- Delete: `ISOLATE_VS_HARDLINE.md`
- Delete: `crates/cli/src/commands/isolate_port.rs`

- [ ] **Step 1: Delete empty ISOLATE_VS_HARDLINE.md**

```bash
rm ISOLATE_VS_HARDLINE.md
```

- [ ] **Step 2: Delete historical isolate_port.rs**

```bash
rm crates/cli/src/commands/isolate_port.rs
```

- [ ] **Step 3: Remove the module reference**

Find where `isolate_port` is declared as a module in the commands directory and remove the `mod isolate_port;` declaration. Check:

```bash
grep -rn "isolate_port" crates/cli/src/commands/
```

Remove any `mod isolate_port;` or `pub mod isolate_port;` lines found.

- [ ] **Step 4: Verify build**

```bash
moon run :build
```

Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: remove stale migration documentation

ISOLATE_VS_HARDLINE.md was empty. isolate_port.rs was a historical
command mapping document no longer needed post-migration."
```

---

### Task 6: Clean Up Doc Comments Referencing isolate_core

**Files:**
- Modify: `crates/core/src/domain/macros.rs` (doc comments referencing `isolate_core`)
- Modify: `crates/core/src/domain/builders/mod.rs` (doc comments)
- Modify: `crates/core/src/domain/repository/mod.rs` (doc comments)
- Modify: `crates/core/src/domain/mod.rs` (doc comments)

- [ ] **Step 1: Find all doc comment references**

```bash
grep -rn "isolate_core" crates/core/src/domain/ --include="*.rs"
```

- [ ] **Step 2: Replace references in doc comments**

For each match found, update the doc comment:
- `isolate_core` → `scp_core` or `crate` as appropriate for context
- These are `/// use isolate_core::...` example snippets in doc strings — update the examples to reference `scp_core` or `crate::` instead

- [ ] **Step 3: Verify build**

```bash
moon run :build
```

Expected: Build succeeds. Doc comment changes don't affect compilation.

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/domain/
git commit -m "docs: update domain doc comments from isolate_core to scp_core"
```

---

### Task 7: Add Holzmann Rule Enforcement to clippy.toml

**Files:**
- Modify: `.clippy.toml`

- [ ] **Step 1: Add 60-line function limit**

Append to `.clippy.toml`:

```toml
too-many-lines-threshold = 60
```

- [ ] **Step 2: Remove Isolate from doc-valid-idents**

In `.clippy.toml`, find the `doc-valid-idents` list and remove `"Isolate"` from it (line 25 area).

- [ ] **Step 3: Verify clippy runs**

```bash
moon run :quick
```

Expected: May produce new warnings for functions exceeding 60 lines. These are acceptable to defer to Phase 2/3 fixes — the limit is now enforced.

- [ ] **Step 4: Commit**

```bash
git add .clippy.toml
git commit -m "chore: enforce Holzmann Rule 4 — 60-line function limit via clippy"
```

---

### Task 8: Final Quality Gate

- [ ] **Step 1: Run full CI pipeline**

```bash
moon run :ci
```

Expected: All tests pass. Zero warnings from new clippy rules.

- [ ] **Step 2: Verify no dead deps remain**

```bash
grep -c "jj-lib\|git2\|rpds\|uuid-no-serde\|either\|askama\|kdl\|dbc.*contracts" Cargo.toml
```

Expected: 0 matches.

- [ ] **Step 3: Verify legacy crates are gone**

```bash
ls crates/isolate crates/isolate-core 2>&1
```

Expected: "No such file or directory" for both.

- [ ] **Step 4: Push to remote**

```bash
git push
```

---

## Summary

| Task | What | Risk |
|------|------|------|
| 1 | Delete isolate + isolate-core | Zero — nothing depends on them |
| 2 | Remove 10 dead workspace deps | Low — workspace = true references need cleanup in Task 3 |
| 3 | Remove dead crate-level deps | Zero — verified unused |
| 4 | Remove unused contract imports | Zero — already `#[allow(unused)]` |
| 5 | Delete stale docs | Zero — no code impact |
| 6 | Clean up doc comments | Zero — documentation only |
| 7 | Add clippy 60-line limit | Low — may surface existing violations |
| 8 | Quality gate + push | — |

**Estimated time:** 30-45 minutes for an agentic worker.
