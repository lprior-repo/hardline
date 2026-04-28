# Why JJ?

Isolate uses JJ (Jujutsu) because it's fundamentally better for multi-agent workflows.

## The Problem with Git at Scale

Running 8-12 agents in parallel with Git:

- **Detached HEAD** — constant broken states at 4+ agents
- **Branch pollution** — 8-12 branches to manage
- **Lost code** — changes overwritten
- **No undo** — destructive operations are permanent
- **Blocking merges** — conflicts block until resolved

Git Worktrees work at 1-3 agents. They break at 4+.

## Why JJ Works

| Feature | Git | JJ |
|---------|-----|-----|
| **Concurrency** | Locking required, can corrupt | Lock-free — runs in parallel safely |
| **Undo** | Destructive — reset is permanent | Operation log — undo ANY operation |
| **Conflicts** | Block merges until resolved | First-class — commit and resolve later |
| **Branches** | Required for everything | Anonymous — no branch names needed |
| **State** | Index/staging is confusing | Working copy auto-committed |
| **Rebasing** | Manual, can lose work | Auto-rebase — descendants follow |

---

## Key Benefits for Multi-Agent

### 1. Lock-Free Concurrency

Multiple agents can run `jj` commands in parallel without repo corruption.

```bash
# Agent 1 and Agent 2 run simultaneously:
# Agent 1
jj describe -m "feat: part one"

# Agent 2
jj describe -m "feat: part two"

# Later, both sync without corruption
jj git fetch --all-remotes
```

### 2. Operation Log

Every operation is logged. You can undo anything.

```bash
# See operation history
jj op log

# Undo the last operation
jj undo

# Undo a specific operation
jj undo <operation-id>
```

### 3. Anonymous Workspaces

No branch names required.

```bash
# Create workspace - no branch name needed
jj workspace add feature-123

# Work normally
jj describe -m "feat: something"

# Push - bookmarks created automatically
jj git push
```

### 4. First-Class Conflicts

Conflicts can be committed and resolved later. No blocking.

```bash
# Sync - conflicts recorded, not blocking
jj git fetch --all-remotes

# Check for conflicts
jj status

# Resolve when ready
vim conflicted_file.rs
jj describe -m "resolve: merge conflict"
```

---

## Why Not Git Worktrees?

| Problem | What Happens |
|---------|-------------|
| **Detached HEAD** | At 4+ agents, constant broken states |
| **Branch pollution** | 8-12 agents = 8-12 branches |
| **No concurrency** | Concurrent worktrees corrupt repo |
| **No operation log** | Mistake = permanent loss |

---

## Why Not File Locking?

File locking treats symptoms, not causes:

- Doesn't prevent duplicate work
- Doesn't prevent logical conflicts
- Doesn't help when things go wrong
- Doesn't scale

We tried Agentail/MCP. It didn't work.

---

## Summary

**JJ enables:**
- Running 8-12 agents in parallel
- Safe auto-rebase on sync
- Recovery from any mistake via operation log
- Clean workspaces without branch pollution

**Git at that scale:**
- Constant broken states
- Lost code
- Merge conflicts
- No recovery

That's why Isolate is built on JJ.
