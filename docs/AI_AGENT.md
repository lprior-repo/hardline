# Isolate AI Agent Guide

> **Comprehensive reference for AI agents working with Isolate, beads, and the isolate toolchain.**

This document serves as the complete reference for autonomous AI agents operating within the Isolate system. It covers mandatory rules, workflows, commands, and best practices.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Mandatory Rules](#mandatory-rules)
3. [Quick Reference](#quick-reference)
4. [Parallel Agent Workflow](#parallel-agent-workflow)
5. [Session Completion](#session-completion)
6. [Environment Variables](#environment-variables)
7. [Error Handling](#error-handling)
8. [Agent Lifecycle](#agent-lifecycle)
9. [Common Patterns](#common-patterns)
10. [Skills Reference](#skills-reference)
11. [Cache Health](#cache-health)
12. [Reference](#reference)

---

## Quick Start

### Three Essential Commands

```bash
isolate whereami        # Check: "main" or "workspace:<name>"
isolate work my-task   # Start: Create workspace
isolate done           # Finish: Merge and cleanup
```

### AI Helper Commands

```bash
isolate ai status       # Full status with guided next action
isolate ai workflow     # 7-step parallel agent workflow
isolate ai quick-start  # Minimum commands reference
```

---

## Mandatory Rules

> ⚠️ **These rules are non-negotiable. Violations will result in failed reviews.**

### Rule 1: NO_CLIPPY_EDITS

**NEVER modify clippy or linting configuration files. EVER.**

**Forbidden files:**
- `.clippy.toml`
- `clippy.toml`
- `#![allow(...)]`
- `#![deny(...)]`
- `Cargo.toml` lint sections
- `moon.yml` lint rules

**Fix the code, not the rules.**

---

### Rule 2: MOON_ONLY

**NEVER use raw cargo commands. ALWAYS use Moon.**

| ❌ NEVER (Forbidden) | ✅ ALWAYS (Required) |
|---------------------|---------------------|
| `cargo fmt` | `moon run :fmt-fix` |
| `cargo clippy` | `moon run :quick` |
| `cargo test` | `moon run :test` |
| `cargo build` | `moon run :build` |
| `cargo check` | `moon run :check` |
| `cargo` (any) | `moon run :<target>` |

**Why:** Moon provides 98.5% faster builds via persistent caching and parallel execution.

---

### Rule 3: CODANNA_ONLY

**NEVER use Grep/Glob/Read for exploration. ALWAYS use Codanna.**

| ❌ NEVER (Forbidden) | ✅ ALWAYS (Required) |
|---------------------|---------------------|
| `Grep` | `mcp__codanna__semantic_search_with_context` |
| `Glob` | `find_symbol` |
| `Read` (for exploration) | `search_symbols` |
| Manual search | `search_documents` |
| | `analyze_impact` |

**Why:** Codanna is pre-indexed with semantic understanding. 90% fewer tokens, 10x faster.

---

### Rule 4: ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC

**ZERO unwrap. ZERO expect. ZERO panic. EVER.**

**Completely Forbidden:**
- ❌ `unwrap()` — under NO circumstances
- ❌ `unwrap_or()` — NO variants allowed
- ❌ `unwrap_or_else()` — NO variants allowed
- ❌ `unwrap_or_default()` — NO variants allowed
- ❌ `expect()` — under NO circumstances
- ❌ `expect_err()` — NO variants allowed
- ❌ `panic!()` — under NO circumstances
- ❌ `todo!()` — under NO circumstances
- ❌ `unimplemented!()` — under NO circumstances

**Required Patterns:**
- ✅ `Result<T, Error>` for all fallible operations
- ✅ `map`, `and_then`, `?` operator for error propagation
- ✅ Railway-Oriented Programming patterns
- ✅ **USE functional-rust-generator SKILL for ALL Rust implementation**

**Why:** Panic-based code is unmaintainable and crashes in production. Unwrap variants are panics in disguise.

---

### Rule 5: GIT_PUSH_MANDATORY

**Work is NOT done until `git push` succeeds.**

| ❌ NEVER | ✅ ALWAYS |
|----------|-----------|
| Stop before pushing | Push yourself |
| Say "ready to push when you are" | Verify `git status` shows "up to date" |
| Leave work stranded locally | Resolve and retry on failure |

**Why:** Unpushed work = lost work. "I'll push later" = stranded commits.

---

### Rule 6: BR_SYNC

**`br` never runs git commands. You must commit bead changes manually.**

After `br sync --flush-only`, you MUST run:

```bash
git add .beads/
git commit -m "sync beads"
```

**Why:** Beads is non-invasive by design. It modifies JSONL only. You must commit those changes.

---

### Rule 7: FUNCTIONAL_RUST_SKILL

**ALWAYS load and use functional-rust-generator skill for ANY Rust implementation.**

- ✅ ALWAYS: Load `functional-rust-generator` skill before writing Rust code
- ✅ This skill enforces zero unwrap/expect/panic patterns
- ✅ Uses Railway-Oriented Programming
- ✅ Provides functional patterns: `map`, `and_then`, `?`

**Why:** The skill exists for a reason - it enforces these patterns automatically. Don't reinvent the wheel.

---

## Quick Reference

### Code Search (Codanna)

| Tool | When to Use | Call |
|------|-------------|------|
| `semantic_search_with_context` | Natural language query → full context | `mcp__codanna__semantic_search_with_context(query="your intent", limit: 5)` |
| `find_symbol` | Know exact name | `mcp__codanna__find_symbol(name="ExactName")` |
| `search_symbols` | Fuzzy pattern search | `mcp__codanna__search_symbols(query="pattern", kind:"Struct|Function|Trait", lang:"rust", limit: 10)` |
| `search_documents` | Search markdown docs | `mcp__codanna__search_documents(query="topic", limit: 5)` |
| `analyze_impact` | Dependency graph | `mcp__codanna__analyze_impact(symbol_id: 123)` |

**Workflow:** semantic_search → find_symbol → search_symbols → search_documents → analyze_impact

**When to reindex:** Codanna returns no results, search seems outdated, or after large code changes.

```bash
codanna index && codanna documents index --collection docs
# Current stats: 5,592 symbols, 817 doc chunks
```

---

### Build (Moon)

| Command | Use | Frequency |
|---------|-----|-----------|
| `moon run :quick` | Fast check (6-7ms cached) | Every edit |
| `moon run :ci` | Full pipeline (parallel) | Before commit |
| `moon run :fmt-fix` | Auto-fix formatting | Before commit |
| `moon run :test` | Run tests | After changes |
| `moon run :check` | Type check only | Quick validation |

---

### Issue Tracking (Beads)

| Command | Use | Frequency |
|---------|-----|-----------|
| `bv --robot-triage` | Find what to work on (entry point) | Start of session |
| `bv --robot-next` | Top pick command | Quick pick |
| `br ready` | List available work | As needed |
| `br show <id>` | View issue details | Before starting |
| `br update <id> --status in_progress` | Start work | When starting |
| `br close <id>` | Complete work | When done |

---

### Workspace (isolate)

| Command | Use | Frequency |
|---------|-----|-----------|
| `isolate add <name>` | Create session + Zellij tab | New work |
| `isolate remove <name>` | Close tab + workspace | Work complete |
| `isolate list` | Show all sessions | Status check |
| `isolate whereami` | Check current location | Orient yourself |
| `isolate work <name>` | Create workspace (simpler than add) | New work |
| `isolate done` | Complete and merge work | Finish work |

**For complete command reference, see [COMMANDS.md](COMMANDS.md)**

---

## Parallel Agent Workflow

### 7-Step Pipeline

Each autonomous agent follows this pipeline:

| Step | Name | Command/Action | Output |
|------|------|----------------|--------|
| 1 | TRIAGE | `bv --robot-triage --robot-triage-by-track` | Parallel execution tracks |
| 2 | CLAIM | `br update <bead-id> --status in_progress` | Reserve bead |
| 3 | ISOLATE | Skill: `isolate` | Spawn isolated JJ workspace + Zellij tab |
| 4 | IMPLEMENT | Skill: `functional-rust-generator` (Rust) or `tdd15-gleam` (Gleam) | ZERO unwrap/expect/panic, Railway-Oriented Programming |
| 5 | REVIEW | Skill: `red-queen` | Adversarial QA, regression hunting |
| 6 | LAND | Skill: `landing-skill` | Moon quick check, commit, sync, push (MANDATORY) |
| 7 | MERGE | Skill: `isolate` | jj rebase -d main, cleanup, tab switch |

---

### Subagent Template

```
**BEAD**: <bead-id> - "<title>"

**WORKFLOW**:
1. CLAIM: `br update <bead-id> --status in_progress`
2. ISOLATE: isolate skill → "<session-name>"
3. IMPLEMENT: functional-rust-generator skill (Rust) or tdd15-gleam skill (Gleam)
   - **ZERO unwrap(), unwrap_or(), unwrap_or_else(), unwrap_or_default()**
   - **ZERO expect(), expect_err()**
   - **ZERO panic!(), todo!(), unimplemented!()**
   - Railway-Oriented Programming
   - map, and_then, ? operator
4. REVIEW: red-queen skill (adversarial QA)
5. LAND: landing-skill (quality gates, sync, push)
6. MERGE: isolate skill (merge to main)

**CRITICAL CONSTRAINTS**:
- **ZERO unwrap/expect/panic variants** (see rule 4)
- Zero unwraps/panics, Moon only, work NOT done until git push succeeds
- **ALWAYS use functional-rust-generator skill for Rust** (rule 7)

Report final status with bead ID.
```

---

### Parallel Execution

```bash
# Get parallel tracks
bv --robot-triage --robot-triage-by-track

# Spawn 8 agents via Task tool
# Each gets unique bead from different track
# All run simultaneously in isolated workspaces
# Orchestrator monitors from clean context
```

### Benefits

| Benefit | Description |
|---------|-------------|
| **Isolation** | Each agent works in separate JJ workspace |
| **Parallel** | 8x throughput with no conflicts |
| **Deterministic** | bv precomputes dependencies and execution tracks |
| **Quality** | Red-queen ensures adversarial testing on each change |
| **Clean handoff** | landing-skill guarantees all work pushed before completion |

---

## Session Completion

### CRITICAL: Work is NOT Done Until `git push` Succeeds

**MANDATORY WORKFLOW (All 7 Steps Required)**

| Step | Action | Details |
|------|--------|---------|
| 1 | File issues | Create issues for anything that needs follow-up |
| 2 | Run quality gates | If code changed: `moon run :quick` (6-7ms) OR `moon run :ci` (full) |
| 3 | Update issues | Close finished work, update in-progress items |
| 4 | **COMMIT AND PUSH (MANDATORY)** | `git add <files> | git commit -m 'desc' | br sync | git pull --rebase | git push | git status` (must show 'up to date') |
| 5 | Verify cache | `systemctl --user is-active bazel-remote` (expect 'active') |
| 6 | Clean up | Clear stashes, prune remote branches |
| 7 | Hand off | Provide context for next session |

---

### Example Session End

```bash
# 1. File any follow-up work
br create "Need to add error handling for edge case"

# 2. Quality gates
moon run :quick

# 3. Update issues
br close 123
br update 456 --status done

# 4. COMMIT AND PUSH (MANDATORY)
git add crates/isolate-core/src/error.rs
git commit -m "fix: add error handling for edge case"
br sync --flush-only
git add .beads/
git commit -m "sync beads"
git pull --rebase
git push
git status # MUST show: "Your branch is up to date with 'origin/main'"

# 5. Verify cache
systemctl --user is-active bazel-remote # Output: active

# 6. Clean up
git stash clear
git remote prune origin

# 7. Hand off
# "Completed bead #123 (error handling). Bead #456 in progress. Cache healthy. No conflicts."
```

---

### What "Ready to Push When You Are" Means

**DO NOT SAY THIS.** It means:
- You're offloading responsibility
- Work might never get pushed
- Next session starts with stranded commits
- Potential merge conflicts accumulate

**Instead:** Push yourself. Verify `git status` shows "up to date". Only then is work complete.

---

### Failure Recovery

If `git push` fails:

1. Check network: `ping github.com`
2. Check auth: `git remote -v` && `ssh -T git@github.com`
3. Pull rebase: `git pull --rebase`
4. Resolve conflicts if any
5. Push again: `git push`
6. Repeat until success
7. Only then report completion

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `Isolate_AGENT_ID` | Your agent ID (set by register) |
| `Isolate_SESSION` | Current session name |
| `Isolate_WORKSPACE` | Current workspace path |
| `Isolate_BEAD_ID` | Associated bead ID |
| `Isolate_ACTIVE` | "1" when in workspace |

---

## JSON Output Pattern

All commands support `--json` and return:

```json
{
  "$schema": "isolate://<command>-response/v1",
  "_schema_version": "1.0",
  "schema_type": "single",
  "success": true,
  ...
}
```

---

## Error Handling

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Validation error (user input) |
| 2 | Not found |
| 3 | System error |
| 4 | External command error |
| 5 | Lock contention |

### Error Format

Errors include suggestions:

```json
{
  "success": false,
  "error": {
    "code": "SESSION_NOT_FOUND",
    "message": "...",
    "suggestion": "Use 'isolate list' to see available sessions"
  }
}
```

---

## Introspection

```bash
isolate introspect              # All capabilities
isolate introspect <cmd>        # Command details
isolate introspect --env-vars   # Environment variables
isolate introspect --workflows  # Workflow patterns
```

---

## Agent Lifecycle

```bash
# Register (optional but recommended)
isolate agent register

# Send heartbeat while working
isolate agent heartbeat --command "implementing"

# Check your status
isolate agent status

# Unregister when done
isolate agent unregister
```

---

## Common Patterns

### Start Fresh

```bash
isolate whereami                        # Should return "main"
isolate work feature-auth --idempotent
```

### Continue Existing Work

```bash
isolate whereami                        # Returns "workspace:feature-auth"
# Already in workspace, continue working
```

### Abandon and Start Over

```bash
isolate abort --dry-run                 # Preview
isolate abort                           # Execute
isolate work feature-auth-v2            # Start fresh
```

### Multiple Sessions

```bash
isolate list --json                     # List all sessions
isolate sync --all                      # Sync all with main
```

---

## What NOT to Do

- ❌ Don't use `isolate spawn` for simple workflows (use `isolate work`)
- ❌ Don't forget `--idempotent` when retrying
- ❌ Don't skip `isolate whereami` before operations
- ❌ Don't modify files outside your workspace
- ❌ Don't use unwrap/expect/panic in Rust code
- ❌ Don't use raw cargo commands (use Moon)
- ❌ Don't use Grep/Glob for code exploration (use Codanna)
- ❌ Don't skip git push

---

## Skills Reference

Load these skills for specialized tasks:

| Skill | Purpose | When to Use |
|-------|---------|-------------|
| `functional-rust-generator` | Rust with zero panics, zero unwraps, Railway-Oriented Programming | **ALL Rust implementation** |
| `tdd15-gleam` | 15-phase TDD workflow for Gleam | Gleam implementation |
| `red-queen` | Adversarial evolutionary QA, regression hunting | Code review/testing |
| `landing-skill` | Session completion with quality gates, sync, push | **Before ending session** |
| `isolate` | Workspace isolation and management | Workspace operations |
| `coding-rigor` | TDD-first development, clean boundaries | Code design |
| `rust-contract` | Design-by-contract, test planning | Planning Rust features |

**Why JJ for Multi-Agent?**

See [09_JUJUTSU.md](09_JUJUTSU.md) — JJ enables 8-12 parallel agents without corruption, unlike Git which breaks at 4+.

---

## Quick Queries

```bash
isolate query location              # Where am I?
isolate query can-spawn             # Can I start work?
isolate query pending-merges        # What needs merging?
```

---

## Safe Flags

> **Always use these flags for safer operations.**

| Flag | Effect |
|------|--------|
| `--idempotent` | Succeed even if already exists |
| `--dry-run` | Preview without executing |
| `--json` | Machine-readable output |

---

## Minimal Workflow

```bash
# 1. Check location
isolate whereami

# 2. Start work (safe to retry with --idempotent)
isolate work my-task --idempotent

# 3. Enter workspace
cd $(isolate context --json | jq -r '.location.path // empty')

# 4. Do work...

# 5. Complete
isolate done
```

---

## Cache Health

Cache must be active for fast builds:

```bash
# Check cache status
systemctl --user is-active bazel-remote
# Expected output: active

# View cache statistics
curl http://localhost:9090/status | jq
```

If inactive, start the cache:

```bash
systemctl --user start bazel-remote
```

---

## Reference

- Full documentation: `isolate --help`
- Command details: `isolate introspect <command>`
- AI status: `isolate ai status`
- Core docs: [docs/INDEX.md](INDEX.md)
- Command reference: [COMMANDS.md](COMMANDS.md)
- Jujutsu workflow: [09_JUJUTSU.md](09_JUJUTSU.md)

---

*This document is maintained as part of the Isolate system. Last updated: 2024.*
