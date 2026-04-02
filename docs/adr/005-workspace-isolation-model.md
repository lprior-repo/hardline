# ADR-005: Workspace Isolation Model

**Date:** 2026-03-20
**Revised:** 2026-04-02
**Status:** Accepted
**Deciders:** Lewis

---

## Context

Hardline provides workspace isolation for AI agents and developers. Each workspace is a complete, isolated development environment that can be created, switched between, and destroyed without affecting other workspaces or the main repository.

The key question: **What does "workspace isolation" mean?**

Options considered:
- **Git worktrees** - Lightweight, shares objects, partial isolation
- **Full clones** - Complete replica, fully isolated
- **Full clones with object deduplication** - Full isolation with lower disk usage (future)

This ADR establishes the workspace isolation model for Hardline.

---

## Decision

### Workspace Definition

A **Workspace** is:

1. **A complete Git clone** of the repository at a specific commit
2. **An isolated working directory** with its own:
   - Working copy (files on disk)
   - `.git` directory (independent object store, refs, index)
   - Branch/ref pointers (invisible to other workspaces)
3. **A named entity** with state tracking

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: WorkspaceName,
    pub path: AbsolutePath,
    pub state: WorkspaceState,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceState {
    /// Workspace created but not yet initialized
    Created,
    /// Workspace is active and ready for work
    Active,
    /// Workspace is syncing with main
    Syncing,
    /// Workspace is paused (agent disconnected)
    Paused,
    /// Workspace completed successfully
    Completed,
    /// Workspace failed or abandoned
    Failed,
}
```

### Isolation Model: Full Git Clones (NOT Worktrees)

**Decision: Hardline uses full `git clone` for workspace isolation, NOT Git worktrees.**

#### Why NOT Git Worktrees

Git worktrees share the `.git` directory between the main repository and all worktrees. This means:

1. **Shared refs** - Branches are visible across all worktrees. Agent A's branches appear in Agent B's `git branch` output.
2. **Shared object store** - Corruption in the object store affects all worktrees.
3. **One working tree per branch** - Cannot have multiple agents on the same branch simultaneously.
4. **Locking conflicts** - Concurrent worktree operations compete for the same `.git/index.lock`.

These are fundamental limitations of the worktree model. They cannot be worked around.

#### Why Full Clones

Each workspace is a complete, independent Git repository created via `git clone`:

```
/workspaces/
  agent-123/       # Full clone: own .git, own refs, own objects
    .git/
    src/
    Cargo.toml
  agent-456/       # Full clone: own .git, own refs, own objects
    .git/
    src/
    Cargo.toml
```

**Benefits:**
1. **True isolation** - No shared refs, objects, or state between workspaces
2. **Crash safety** - Workspace corruption cannot spread to other workspaces
3. **Branch independence** - Multiple workspaces can have the same branch checked out
4. **No locking conflicts** - Each workspace has its own `.git/index.lock`
5. **Clean lifecycle** - `rm -rf /workspaces/agent-123` destroys everything

#### Trade-offs

| Aspect | Full Clones | Git Worktrees |
|--------|-------------|---------------|
| Disk space | Higher (duplicated .git) | Lower (shared objects) |
| Creation speed | Slower (full clone) | Faster (reflink) |
| Isolation | Complete | Partial (shared refs) |
| Corruption risk | Isolated to one workspace | Can spread via shared store |
| Branch independence | Full (same branch in multiple workspaces) | Limited (one worktree per branch) |
| Concurrent locking | Independent locks | Shared lock contention |

### How Full Clones Solve Agent Concurrency

With full clones, each agent workspace operates on an independent Git repository. There are no shared files between workspaces. Agents can:

- `git commit` simultaneously without lock contention
- `git rebase` on the same branch without conflicts
- `git push` independently to different remotes or branches
- Fail without corrupting other agents' work

The only coordination point is the shared remote (e.g., GitHub). Agents push to separate branches and merge via pull requests.

### Workspace State Machine

```rust
impl Workspace {
    pub fn transition_to(&mut self, new_state: WorkspaceState)
        -> Result<(), WorkspaceTransitionError>
    {
        match (&self.state, &new_state) {
            // Valid transitions
            (Created, Active) => {},
            (Created, Failed) => {},
            (Active, Syncing) => {},
            (Active, Paused) => {},
            (Active, Completed) => {},
            (Active, Failed) => {},
            (Syncing, Active) => {},
            (Syncing, Failed) => {},
            (Paused, Active) => {},
            (Paused, Failed) => {},

            // Terminal states
            (Completed, _) => return Err(TerminalState),
            (Failed, _) => return Err(TerminalState),

            // Invalid transitions
            _ => return Err(InvalidTransition {
                from: self.state,
                to: new_state,
            }),
        }

        self.state = new_state;
        Ok(())
    }
}
```

### Valid State Transitions

```
+-----------+
| Created   |  Workspace record created, clone not yet started
+-----+-----+
      | activate()
      v
+-----------+
| Active    |  Clone complete, ready for work
+-----+-----+
      |
      +------------------------------+
      | sync()                       | pause()
      v                              v
+-----------+                   +-----------+
| Syncing   |                   | Paused    |
+-----+-----+                   +-----+-----+
      |                              |
      | (success)                    | resume()
      v                              v
      |                    +---------------------+
      |<-------------------+                     |
      |                                          |
      | complete()                              | (agent dies)
      v                                          v
+-------------+                         +-----------+
| Completed   | (terminal)              | Failed    | (terminal)
+-------------+                         +-----------+
```

---

## Variants Considered

### Variant A: Git Worktrees (REJECTED)

```rust
struct Workspace {
    worktree_path: PathBuf,
    branch_name: BranchName,
    // Worktree is linked to main repo's .git
}
```

**Rejected because:**
- Shared refs violate isolation requirement
- Shared object store allows corruption to spread
- One worktree per branch limits concurrency
- Lock contention on shared `.git/index`

### Variant B: Full Clones (CHOSEN)

```rust
struct Workspace {
    id: WorkspaceId,
    path: AbsolutePath,       // /workspaces/agent-123
    git_dir: PathBuf,         // /workspaces/agent-123/.git (independent)
    state: WorkspaceState,
}
```

**Chosen because:**
- True isolation - no shared state
- Simple mental model - each workspace is just a directory
- Crash containment per workspace
- Disk space is cheap, isolation is critical

### Variant C: Full Clones with Object Deduplication (FUTURE)

```rust
struct Workspace {
    id: WorkspaceId,
    path: AbsolutePath,
    // Use reflink/copy-on-write for .git directory
    // to save disk space while maintaining isolation
}
```

**Deferred because:**
- Platform-specific (reflink on Linux/btrfs, clonefile on macOS/APFS)
- Complexity not yet justified at current scale
- May revisit when disk usage becomes a bottleneck

---

## Invariants

### Workspace Identity Invariants

```rust
/// INVARIANT: Workspace ID is globally unique
assert!(workspace.id != any_other_workspace.id);

/// INVARIANT: Workspace name is unique within a repository
assert!(repo.workspaces().iter().all_unique_by(|w| &w.name));

/// INVARIANT: Workspace path does not overlap with other workspaces
for w1 in workspaces {
    for w2 in workspaces {
        if w1.id != w2.id {
            assert!(!w1.path.starts_with(w2.path));
            assert!(!w2.path.starts_with(w1.path));
        }
    }
}
```

### State Machine Invariants

```rust
/// INVARIANT: State transitions are valid
pub fn is_valid_transition(from: WorkspaceState, to: WorkspaceState) -> bool {
    matches!(
        (from, to),
        (Created, Active) | (Created, Failed) |
        (Active, Syncing) | (Active, Paused) | (Active, Completed) | (Active, Failed) |
        (Syncing, Active) | (Syncing, Failed) |
        (Paused, Active) | (Paused, Failed)
    )
}

/// INVARIANT: Terminal states are final
assert!(matches!(Completed, TerminalState));
assert!(matches!(Failed, TerminalState));
```

### Filesystem Invariants

```rust
/// INVARIANT: Workspace directory exists if and only if state != Created
match workspace.state {
    Created => assert!(!workspace.path.exists()),
    _ => assert!(workspace.path.exists()),
}

/// INVARIANT: .git directory exists for non-Created workspaces
assert!(workspace.path.join(".git").exists());

/// INVARIANT: No workspace path is a prefix of another workspace path
fn no_nested_workspaces(workspaces: &[Workspace]) -> bool {
    for w1 in workspaces {
        for w2 in workspaces {
            if w1.id != w2.id {
                assert!(!w2.path.starts_with(w1.path));
            }
        }
    }
}
```

### Isolation Invariants

```rust
/// INVARIANT: Each workspace has an independent .git directory
for w in workspaces {
    assert!(w.path.join(".git").is_dir());
    assert!(w.path.join(".git/HEAD").exists());
}

/// INVARIANT: Active workspace count doesn't exceed limit
assert!(active_workspaces().count() <= MAX_CONCURRENT_WORKSPACES);
```

---

## Consequences

### Positive

1. **True isolation** - No shared refs or object stores between workspaces
2. **Crash containment** - One workspace's corruption cannot affect others
3. **Simple model** - Each workspace is a directory; delete it to clean up
4. **Security** - Agent A cannot access Agent B's workspace files
5. **Independent lifecycle** - Create, destroy, and manage workspaces without side effects
6. **No lock contention** - Each workspace has its own Git lock files

### Negative

1. **Disk usage** - Full .git directory per workspace (~50-100MB each)
2. **Creation time** - Full clone takes longer than worktree creation (~5-10 seconds)
3. **Maintenance** - Must clean up stale workspaces periodically

### Scale Considerations

For 600+ concurrent agents:
- **Disk**: 600 x 100MB = 60GB (acceptable for workspace servers)
- **Creation**: Parallel cloning with rate limiting
- **Cleanup**: Automatic cleanup of abandoned workspaces after timeout
- **Optimization**: Variant C (reflink deduplication) can reduce disk usage when needed

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/workspace/src/domain/workspace.rs` | Workspace entity + state machine |
| `crates/workspace/src/domain/state.rs` | WorkspaceState enum + transitions |
| `crates/workspace/src/infrastructure/filesystem.rs` | Clone/cleanup operations |
| `crates/vcs/src/backend/git.rs` | Git clone operations for workspace creation |

---

## Related ADRs

- ADR-004: VCS Abstraction (Git-only backend, GitBackend struct)
- ADR-001: CLI Architecture (workspace commands: spawn, switch, list, forget)
- ADR-002: Durable Workflow Execution (workspace operations with recovery)
