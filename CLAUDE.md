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

- **Sub-Agent Workflow**: Use the `go-skill` (GoMasterOrchestrator) for writing new code.
- **Code Reviews**: ALL code must be reviewed by the `qa-enforcer` agent and the `black-hat-reviewer` agent.
- **Test Reviews**: Tests must ALWAYS be reviewed by the `test-reviewer` agent to enforce Beck, North, Farley, and Testing Trophy patterns.
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
