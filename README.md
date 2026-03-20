# Hardline

Workspace isolation for AI agent swarms. Built on JJ.

## The Problem

Running 8-12 agents in parallel is chaos. Without proper isolation:

- **Lost code** — changes overwritten, gone forever
- **Duplicate work** — the same feature re-implemented 3-4x
- **Bead stealing** — agents claiming work already in progress
- **Detached HEAD** — constantly stuck in broken states
- **Broken main** — always blocked, always broken

We tried to fix this:

- **File locking (Agentail/MCP)** — good first attempt, but didn't work. Too fragile. Doesn't prevent duplicate work, doesn't help when things go wrong, doesn't scale.
- **Git Worktrees** — work fine at 1-3 agents. Break completely at 4+.

## The Solution

**File locking treats symptoms, not causes.**

Real solution: **complete workspace isolation**. Each agent gets their own isolated environment. No shared state to corrupt, no coordination needed between agents.

---

## FAQ: JJ, Git, and Why This Matters

### Why JJ instead of Git?

JJ is a fundamentally better VCS for multi-agent workflows:

| Feature | Git | JJ (Why it matters) |
|---------|-----|---------------------|
| **Concurrency** | Locking required, can corrupt at scale | Lock-free — multiple agents can run in parallel without repo corruption. They just see "divergent changes" and resolve later. |
| **Undo** | Destructive — reset is permanent | Operation log — undo ANY operation, even non-recent ones. Recover from any mistake. |
| **Conflicts** | Block merges until resolved | First-class — can commit conflicts, resolve later. No blocking. |
| **Branches** | Required for everything (branch pollution) | Anonymous — workspaces don't need branch names. No namespace pollution at 8-12 agents. |
| **State** | Index/staging area is confusing | Working copy auto-committed — simpler, consistent model |
| **Rebasing** | Manual, can lose work | Auto-rebase — descendants automatically follow rewritten commits |
| **Merges** | Evil merges exist | No evil merges — merge commits handled correctly |

**The big ones for agent swarms:**

- **Lock-free concurrency** means agents don't corrupt each other's work
- **Operation log** means you can always recover from mistakes
- **Anonymous commits** means no branch name collisions at scale

### Why not Git Worktrees?

Git Worktrees work great at small scale. They break at agent scale:

| Problem | What happens |
|---------|-------------|
| **Detached HEAD** | At 4+ agents, you spend half your time in detached HEAD state |
| **Branch pollution** | 8-12 agents = 8-12 branches to manage. Name collisions are constant. |
| **No concurrency** | Concurrent worktrees can corrupt the repo |
| **No operation log** | Mistake = permanent loss |
| **File locking doesn't scale** | We tried it. It didn't work. |

**The honest threshold:**
- 1-3 agents with human coordination: Git Worktrees are fine
- 4+ autonomous agents: You're hitting a wall. We know because we lived it.

### Why not file locking?

File locking treats symptoms, not causes:

- **Doesn't prevent duplicate work** — two agents can implement the same feature on different files
- **Doesn't prevent logical conflicts** — agents stepping on each other's toes across the codebase
- **Doesn't help when things go wrong** — no recovery mechanism
- **Doesn't scale** — more agents = more contention, more contention = more failures

We tried Agentail/MCP. It was a good first crack at the problem. But file locking is fundamentally the wrong abstraction for multi-agent coordination.

**Real solution:** Complete workspace isolation. Each agent has their own environment. No shared state to corrupt.

---

## What Hardline Adds on Top of JJ

- **CLI ergonomics** — `spawn`, `done`, `sync`, `abort` commands
- **Session state tracking** — knows who's working where
- **Bead claiming** — atomic ownership of tasks
- **Recovery logic** — robust handling of interrupted sessions
- **Clean merge workflow** — easy to sync and merge back to main

## Key Commands

```bash
# Initialize Hardline in a repo
hardline init

# Spawn a new isolated workspace for a task
hardline spawn <bead-id>

# Switch between workspaces
hardline switch <workspace-name>

# List all workspaces
hardline list

# Sync workspace with main
hardline sync

# Merge completed work back to main
hardline done

# Abort and clean up a workspace
hardline abort

# Check status of your workspace
hardline status
```

## Requirements

- **JJ (Jujutsu)** must be installed. Hardline is built on top of JJ and requires it to function.
- Install via: `cargo install jj-cli` or `brew install jj`

## Tradeoffs

- **JJ learning curve** — new mental model, different from Git
- **Ecosystem integration** — GitHub, CI, code review tools expect Git (JJ interop exists but isn't first-class everywhere)
- **But:** your main stays clean, your agents don't destroy each other's work, and you can actually run 8-12 agents in parallel without losing code

## Why JJ?

JJ provides the perfect foundation for isolation:

- **Anonymous commits** — workspaces don't need to share branch names
- **Undo capability** — completely safe operations with easy rollback via operation log
- **Sparse checkouts** — only see what you need
- **Conflict resolution** — sane merge handling with first-class conflicts
- **Operation log** — full history of workspace changes, can recover from any state

## Installation

```bash
cargo install hardline
```

Or build from source:

```bash
cargo install --path crates/hardline
```

## Getting Started

```bash
# Initialize in your repo
cd your-project
hardline init

# Create an isolated workspace for a task
hardline spawn feature-123

# Do your work...

# Sync with main if needed
hardline sync

# When done, merge back
hardline done
```

## Documentation

See the `docs/` directory for:

- [AI Agent Guide](./docs/AI_AGENT_GUIDE.md) — how to use Hardline with AI agents
- [Rollout/Rollback](./docs/ROLLOUT_ROLLBACK.md) — deployment strategies
- [Error Troubleshooting](./docs/ERROR_TROUBLESHOOTING.md) — common issues and fixes

## License

MIT
