# AGENTS.md

## Mandatory Rules

```jsonl
{"rule":"NO_CLIPPY_EDITS","action":"Fix code, not lint config"}
{"rule":"MOON_ONLY","cmds":["moon run :quick","moon run :test","moon run :build","moon run :ci","moon run :fmt-fix"],"never":["cargo fmt","cargo test","cargo clippy","cargo build"]}
{"rule":"ZERO_UNWRAP_PANIC_SOURCE_ONLY","action":"Source code (src/) ONLY: zero unwrap/panic. Tests (_tests.rs, #[test]): NO restrictions - anything goes","banned_source":["unwrap()","unwrap_or()","unwrap_or_else()","unwrap_or_default()","expect()","panic!()","todo!()","unimplemented!()"],"allowed_tests":["ALL unwrap/panic allowed - tests are exempt"]}
{"rule":"GIT_PUSH_MANDATORY","action":"Not done until git push succeeds"}
{"rule":"FUNCTIONAL_RUST_SKILL","action":"Load functional-rust-generator skill for ALL Rust implementation"}
{"rule":"DOMAIN_DRIVEN_DESIGN","patterns":["Bounded contexts","Aggregates","Value objects","Domain events","Repository pattern","Factory pattern"],"action":"Model domain logic explicitly; separate domain from infrastructure"}
{"rule":"MANUAL_TESTING","action":"After implementation: manually test via CLI; verify actual behavior; no mocking reality"}
```

## Sub-Agent Development Protocol

- **New Code**: We use sub-agent development. ALWAYS use the `go-skill` (GoMasterOrchestrator) for writing new code.
- **Code Review**: ALL code must be reviewed by the `qa-enforcer` agent and the `black-hat-reviewer` agent to ensure perfect code hygiene and architectural compliance.
- **Test Review**: If writing tests, they must ALWAYS be reviewed by the `test-reviewer` agent (enforcing Beck, North, Farley, and the Testing Trophy pattern).
- **Functional Rust**: ALWAYS invoke the `functional-rust` skill when implementing Rust code to enforce zero-panic architectural purity (Data->Calc->Actions).

## Documentation

- **Location**: All documentation, architectural specs, and codebase guidelines can be found in the `docs/` directory and `architecture-spec.md` at the project root.

## Workflow

```jsonl
{"step":"IMPLEMENT","cmds":["Load functional-rust-generator skill","Implement with Result<T,E> + DDD patterns","moon run :quick","moon run :test"],"output":"Code passes all checks"}
{"step":"MANUAL_TEST","cmds":["Run actual CLI commands","Verify real behavior","Test edge cases","Document findings"],"output":"Manual verification complete"}
{"step":"REVIEW","cmds":["moon run :ci"],"output":"All quality gates pass"}
{"step":"LAND","cmds":["git add .","git commit -m '<msg>'","git push"],"output":"Changes pushed to remote"}
```

## Functional Rust

```jsonl
{"pattern":"Railway-Oriented Programming","use":["Result<T,E> everywhere","? operator for propagation","map/and_then for transformation","Early returns with Err"],"avoid":["unwrap variants","panic variants","null/option where Result fits"]}
{"pattern":"Pure Functions","use":["Input -> Output","No side effects in domain logic","Deterministic behavior"],"avoid":["Global state","Hidden mutations","Unpredictable behavior"]}
{"pattern":"Type Safety","use":["Newtype pattern","Phantom types","Type-driven design","Compile-time guarantees"],"avoid":["Stringly-typed APIs","Primitive obsession","Runtime validation only"]}
{"pattern":"Immutability","use":["Immutable by default","let instead of let mut","Clone when needed"],"avoid":["Unnecessary mut","Shared mutable state"]}
```

## Domain-Driven Design

```jsonl
{"pattern":"Bounded Context","action":"Each module is a clear boundary; explicit interfaces between contexts"}
{"pattern":"Aggregates","action":"Cluster entities and value objects; enforce invariants at aggregate root"}
{"pattern":"Value Objects","action":"Immutable types for domain concepts; equality by value not identity"}
{"pattern":"Domain Events","action":"Model state changes as events; enable event sourcing patterns"}
{"pattern":"Repository Pattern","action":"Abstract persistence; domain doesn't know about storage details"}
{"pattern":"Factory Pattern","action":"Complex object creation logic; validate invariants at construction"}
{"pattern":"Ubiquitous Language","action":"Code uses exact domain terminology; types mirror domain concepts"}
```

## Banned Commands

```jsonl
{"banned":["cat .env","printenv | grep -i token","echo $DATABASE_URL","cargo fmt","cargo test","cargo clippy","cargo build","git reset --hard","git checkout -- ."]}
{"allowed":["moon run :check|:test|:build|:quick|:ci|:fmt-fix"]}
```

<!-- BEGIN BEADS INTEGRATION v:1 profile:full hash:d4f96305 -->
## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Dolt-powered version control with native sync
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json
bd create "Issue title" --description="What this issue is about" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update <id> --claim --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task atomically**: `bd update <id> --claim`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details about what was found" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Auto-Sync

bd automatically syncs via Dolt:

- Each write auto-commits to Dolt history
- Use `bd dolt push`/`bd dolt pull` for remote sync
- No manual export/import needed!

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and docs/QUICKSTART.md.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

<!-- END BEADS INTEGRATION -->
