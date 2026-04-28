# Isolate AI Agent Guide

Complete reference for AI agents working with Isolate, beads, and the isolate toolchain.

---

## Quick Start (3 commands)

```bash
isolate whereami        # Check: "main" or "workspace:<name>"
isolate work my-task    # Start: Create workspace
isolate done            # Finish: Merge and cleanup
```

Or use the AI helpers:
```bash
isolate ai status       # Full status with guided next action
isolate ai workflow     # 7-step parallel agent workflow
isolate ai quick-start  # Minimum commands reference
```

---

## 7 Absolute Mandatory Rules

### 1. NO_CLIPPY_EDITS
**NEVER modify clippy or linting configuration files. EVER.**

Files: `.clippy.toml`, `clippy.toml`, `#![allow(...)]`, `#![deny(...)]`, `Cargo.toml` lint sections, `moon.yml` lint rules.

**Fix the code, not the rules.**

### 2. MOON_ONLY
**NEVER use raw cargo commands. ALWAYS use Moon.**

- ❌ NEVER: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build`
- ✅ ALWAYS: `moon run :quick`, `moon run :test`, `moon run :build`, `moon run :ci`, `moon run :fmt-fix`, `moon run :check`

**Why:** Moon provides 98.5% faster builds via persistent caching and parallel execution.

### 3. CODANNA_ONLY
**NEVER use Grep/Glob/Read for exploration. ALWAYS use Codanna.**

- ❌ NEVER: `Grep`, `Glob`, `Read` for code exploration
- ✅ ALWAYS: `mcp__codanna__semantic_search_with_context`, `find_symbol`, `search_symbols`, `search_documents`, `get_calls`, `find_callers`, `analyze_impact`

**Why:** Codanna is pre-indexed with semantic understanding. 90% fewer tokens, 10x faster.

### 4. ZERO_UNWRAP_ZERO_EXPECT_ZERO_PANIC
**ZERO unwrap. ZERO expect. ZERO panic. EVER.**

**COMPLETELY FORBIDDEN:**
- ❌ `unwrap()` - under NO circumstances
- ❌ `unwrap_or()` - NO variants allowed
- ❌ `unwrap_or_else()` - NO variants allowed
- ❌ `unwrap_or_default()` - NO variants allowed
- ❌ `expect()` - under NO circumstances
- ❌ `expect_err()` - NO variants allowed
- ❌ `panic!()` - under NO circumstances
- ❌ `todo!()` - under NO circumstances
- ❌ `unimplemented!()` - under NO circumstances

**REQUIRED:**
- ✅ `Result<T, Error>` for all fallible operations
- ✅ `map`, `and_then`, `?` operator for error propagation
- ✅ Railway-Oriented Programming patterns
- ✅ **USE functional-rust-generator SKILL for ALL Rust implementation**

**Why:** Panic-based code is unmaintainable and crashes in production. Unwrap variants are panics in disguise. Expect is just panic with a message.

### 5. GIT_PUSH_MANDATORY
**Work is NOT done until `git push` succeeds.**

- ❌ NEVER: Stop before pushing, say "ready to push when you are", leave work stranded locally
- ✅ ALWAYS: Push yourself, verify `git status` shows "up to date", resolve and retry on failure

**Why:** Unpushed work = lost work. "I'll push later" = stranded commits.

### 6. BR_SYNC
**`br` never runs git commands. You must commit bead changes manually.**

After `br sync --flush-only`, you MUST run:
```bash
git add .beads/
git commit -m "sync beads"
```

**Why:** Beads is non-invasive by design. It modifies JSONL only. You must commit those changes.

### 7. FUNCTIONAL_RUST_SKILL
**ALWAYS load and use functional-rust-generator skill for ANY Rust implementation.**

- ✅ ALWAYS: Load `functional-rust-generator` skill before writing Rust code
- ✅ This skill enforces zero unwrap/expect/panic patterns
- ✅ Uses Railway-Oriented Programming
- ✅ Provides functional patterns: `map`, `and_then`, `?`

**Why:** The skill exists for a reason - it enforces these patterns automatically. Don't reinvent the wheel.

---

## Quick Reference: 90% of Workflows

### Code Search (Codanna)

```jsonl
{"tool": "semantic_search_with_context", "when": "Natural language query → full context", "call": "mcp__codanna__semantic_search_with_context(query=\"your intent\", limit: 5)"}
{"tool": "find_symbol", "when": "Know exact name", "call": "mcp__codanna__find_symbol(name=\"ExactName\")"}
{"tool": "search_symbols", "when": "Fuzzy pattern search", "call": "mcp__codanna__search_symbols(query=\"pattern\", kind:\"Struct|Function|Trait\", lang:\"rust\", limit: 10)"}
{"tool": "search_documents", "when": "Search markdown docs", "call": "mcp__codanna__search_documents(query=\"topic\", limit: 5)"}
{"tool": "analyze_impact", "when": "Dependency graph", "call": "mcp__codanna__analyze_impact(symbol_id: 123)"}
```

**Workflow:** semantic_search → find_symbol → search_symbols → search_documents → analyze_impact

**When to reindex:** Codanna returns no results, search seems outdated, or after large code changes.
```bash
codanna index && codanna documents index --collection docs
# Current stats: 5,592 symbols, 817 doc chunks
```

### Build (Moon)

```jsonl
{"cmd": "moon run :quick", "use": "Fast check (6-7ms cached)", "frequency": "Every edit"}
{"cmd": "moon run :ci", "use": "Full pipeline (parallel)", "frequency": "Before commit"}
{"cmd": "moon run :fmt-fix", "use": "Auto-fix formatting", "frequency": "Before commit"}
{"cmd": "moon run :test", "use": "Run tests", "frequency": "After changes"}
{"cmd": "moon run :check", "use": "Type check only", "frequency": "Quick validation"}
```

### Issue Tracking (Beads)

```jsonl
{"cmd": "bv --robot-triage", "use": "Find what to work on (entry point)", "frequency": "Start of session"}
{"cmd": "bv --robot-next", "use": "Top pick command", "frequency": "Quick pick"}
{"cmd": "br ready", "use": "List available work", "frequency": "As needed"}
{"cmd": "br show <id>", "use": "View issue details", "frequency": "Before starting"}
{"cmd": "br update <id> --status in_progress", "use": "Start work", "frequency": "When starting"}
{"cmd": "br close <id>", "use": "Complete work", "frequency": "When done"}
```

### Workspace (isolate)

```jsonl
{"cmd": "isolate add <name>", "use": "Create session + Zellij tab", "frequency": "New work"}
{"cmd": "isolate remove <name>", "use": "Close tab + workspace", "frequency": "Work complete"}
{"cmd": "isolate list", "use": "Show all sessions", "frequency": "Status check"}
{"cmd": "isolate whereami", "use": "Check current location", "frequency": "Orient yourself"}
{"cmd": "isolate work <name>", "use": "Create workspace (simpler than add)", "frequency": "New work"}
{"cmd": "isolate done", "use": "Complete and merge work", "frequency": "Finish work"}
```

**For complete command reference, see [COMMANDS.md](COMMANDS.md)**

---

## 7-Step Parallel Agent Workflow

Each autonomous agent follows this pipeline:

```jsonl
{"step": "1", "name": "TRIAGE", "cmd": "bv --robot-triage --robot-triage-by-track", "output": "Parallel execution tracks", "tool": "bv"}
{"step": "2", "name": "CLAIM", "cmd": "br update <bead-id> --status in_progress", "output": "Reserve bead", "tool": "br"}
{"step": "3", "name": "ISOLATE", "skill": "isolate", "output": "Spawn isolated JJ workspace + Zellij tab", "tool": "Skill tool"}
{"step": "4", "name": "IMPLEMENT", "skill": "functional-rust-generator (Rust) | tdd15-gleam (Gleam)", "output": "ZERO unwrap/expect/panic, Railway-Oriented Programming", "tool": "Skill tool"}
{"step": "5", "name": "REVIEW", "skill": "red-queen", "output": "Adversarial QA, regression hunting", "tool": "Skill tool"}
{"step": "6", "name": "LAND", "skill": "landing-skill", "output": "Moon quick check, commit, sync, push (MANDATORY)", "tool": "Skill tool"}
{"step": "7", "name": "MERGE", "skill": "isolate", "output": "jj rebase -d main, cleanup, tab switch", "tool": "Skill tool"}
```

### Subagent Template

```markdown
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

### Parallel Execution Example

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

## Session Completion (Landing the Plane)

### CRITICAL: Work is NOT Done Until `git push` Succeeds

**MANDATORY WORKFLOW (All 7 Steps Required)**

```jsonl
{"step": "1", "action": "File issues", "details": "Create issues for anything that needs follow-up", "tool": "br"}
{"step": "2", "action": "Run quality gates", "details": "If code changed: moon run :quick (6-7ms) OR moon run :ci (full)", "tool": "moon"}
{"step": "3", "action": "Update issues", "details": "Close finished work, update in-progress items", "tool": "br"}
{"step": "4", "action": "COMMIT AND PUSH (MANDATORY)", "details": "git add <files> | git commit -m 'desc' | br sync | git pull --rebase | git push | git status (must show 'up to date')", "tool": "git, br"}
{"step": "5", "action": "Verify cache", "details": "systemctl --user is-active bazel-remote (expect 'active')", "tool": "systemctl"}
{"step": "6", "action": "Clean up", "details": "Clear stashes, prune remote branches", "tool": "git"}
{"step": "7", "action": "Hand off", "details": "Provide context for next session", "output": "Summary message"}
```

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

### What "Ready to Push When You Are" Means

**DO NOT SAY THIS.** It means:
- You're offloading responsibility
- Work might never get pushed
- Next session starts with stranded commits
- Potential merge conflicts accumulate

**Instead:** Push yourself. Verify `git status` shows "up to date". Only then is work complete.

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

Exit codes:
- 0: Success
- 1: Validation error (user input)
- 2: Not found
- 3: System error
- 4: External command error
- 5: Lock contention

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

- Don't use `isolate spawn` for simple workflows (use `isolate work`)
- Don't forget `--idempotent` when retrying
- Don't skip `isolate whereami` before operations
- Don't modify files outside your workspace
- Don't use unwrap/expect/panic in Rust code
- Don't use raw cargo commands (use Moon)
- Don't use Grep/Glob for code exploration (use Codanna)
- Don't skip git push

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

## Safe Flags (Always Use These)

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
systemctl --user is-active bazel-remote # Should output: active
curl http://localhost:9090/status | jq  # Should show cache stats
```

If inactive: `systemctl --user start bazel-remote`

---

## Reference

- Full documentation: `isolate --help`
- Command details: `isolate introspect <command>`
- AI status: `isolate ai status`
- Core docs: [docs/INDEX.md](INDEX.md)
