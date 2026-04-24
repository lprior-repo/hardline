# AI Agent Instructions

This repository relies on strict agent workflows. For the comprehensive set of rules and architectural guidelines, always refer to `AGENTS.md`.

## Build and Test Commands

**CRITICAL**: DO NOT use raw `cargo` commands (`cargo build`, `cargo test`, `cargo clippy`, etc.). Always use `moon`.

- **Build**: `moon run :build`
- **Test**: `moon run :test`
- **Quick Check**: `moon run :quick`
- **CI Pipeline**: `moon run :ci`
- **Format & Fix**: `moon run :fmt-fix`

## Development Protocols

- **MANDATORY: GoMasterOrchestrator Pipeline**: ALL development work (new features, bug fixes, refactors) MUST use the `go-skill` (GoMasterOrchestrator). This is non-negotiable. The pipeline is: `bd ready` → claim bead → `go-skill` → implement → review → land. Do NOT write implementation code outside this pipeline.
- **Functional Rust**: ALWAYS invoke the `functional-rust` skill for Rust implementation. We strictly follow zero-panic architectural purity (`Data->Calc->Actions`) in the `src/` directory (tests are exempt).
- **Manual Verification**: After implementation, you must manually test via CLI and verify actual behavior. Do not mock reality.

## Documentation and Architecture

- **Location**: See the `docs/` directory and `architecture-spec.md` at the project root.
- **Domain-Driven Design**: Model domain logic explicitly. Separate domain from infrastructure. Follow principles like Bounded Contexts, Aggregates, and Value Objects.

## Issue Tracking

We use **bd (beads)** for ALL task tracking. 
- **DO NOT** use markdown TODO lists. 
- **DO NOT** use external issue trackers.
- See `AGENTS.md` for the specific `bd` commands (`bd ready --json`, `bd update`, etc.).

## Landing the Plane

Do not finish a task until code passes all quality gates and is pushed to the remote repository. 
```bash
git pull --rebase
bd dolt push
git push
```


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

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
