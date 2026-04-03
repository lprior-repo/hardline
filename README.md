# Hardline

Workspace isolation for AI agent swarms. Built on Git.

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

## FAQ: Why This Matters

### Why complete workspace isolation?

Running multiple AI agents against a shared repository creates chaos:

| Problem | What happens without isolation |
|---------|-------------------------------|
| **Lost code** | Changes overwritten by concurrent agents |
| **Duplicate work** | Multiple agents implement the same feature |
| **Bead stealing** | Agents claim work already in progress |
| **Detached HEAD** | Agents stuck in broken Git states |
| **Broken main** | Main branch constantly blocked and broken |

### Why not Git Worktrees alone?

Git Worktrees work fine at small scale (1-3 agents). They break at agent scale:

| Problem | What happens |
|---------|-------------|
| **Detached HEAD** | At 4+ agents, you spend half your time in detached HEAD state |
| **Branch pollution** | 8-12 agents = 8-12 branches to manage. Name collisions are constant. |
| **No concurrency** | Concurrent worktrees can corrupt the repo |
| **File locking doesn't scale** | We tried it. It didn't work. |

### Why not file locking?

File locking treats symptoms, not causes:

- **Doesn't prevent duplicate work** — two agents can implement the same feature on different files
- **Doesn't prevent logical conflicts** — agents stepping on each other's toes across the codebase
- **Doesn't help when things go wrong** — no recovery mechanism
- **Doesn't scale** — more agents = more contention, more contention = more failures

We tried Agentail/MCP. It was a good first crack at the problem. But file locking is fundamentally the wrong abstraction for multi-agent coordination.

**Real solution:** Complete workspace isolation via full Git clones. Each agent has their own environment. No shared state to corrupt.

---

## What Hardline Adds on Top of Git

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

- **Git** must be installed. Hardline uses Git for all version control operations.
- Install via: `sudo apt install git` or `brew install git`

## Tradeoffs

- **Workspace overhead** — each agent gets a full clone, which uses disk space
- **Merge coordination** — merging multiple agent branches back to main requires careful sequencing
- **But:** your main stays clean, your agents don't destroy each other's work, and you can actually run 8-12 agents in parallel without losing code

## Why Git?

Git provides a solid, battle-tested foundation for workspace isolation:

- **Full clones** — each workspace is a complete, independent repository
- **Branching** — standard Git branches for each agent workspace
- **Merging** — proven merge and rebase workflows for integrating agent work
- **Ecosystem** — GitHub, CI, code review tools all work natively
- **Reliability** — the most widely used VCS with decades of production testing

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
