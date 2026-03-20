# ADR-005: Workspace Isolation Model

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline provides workspace isolation for AI agents and developers. Each workspace is a complete, isolated development environment that can be created, switched between, and destroyed without affecting other workspaces or the main repository.

The key question: **What does "workspace isolation" mean?**

Options considered:
- **Git worktrees** - Lightweight, shares objects
- **Full clones** - Complete replica, fully isolated
- **JJ workspaces** - JJ's native workspace concept with shared storage

This ADR establishes the workspace isolation model for hardline.

---

## Decision

### Workspace Definition

A **Workspace** is:

1. **A complete checkout** of the repository at a specific commit
2. **An isolated working directory** with its own:
   - Working copy (files on disk)
   - Index/staging area
   - Branch/ref pointers
3. **A named entity** with state tracking

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: WorkspaceName,
    pub path: AbsolutePath,
    pub backend: VcsType,
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

### Isolation Model: Full Clones (NOT Worktrees)

**Decision: Hardline uses full clones for workspace isolation, NOT Git worktrees.**

#### Why NOT Git Worktrees

```rust
// Git worktrees share:
// - .git/objects (object store)
// - refs (branches are shared!)
// - HEAD (but have separate working copy)

// Problems:
let worktree_path = repo.worktree_add("feature-1", &branch, false).unwrap();
// The branch EXISTS in the main repo
// If I delete the worktree, the branch remains in main
// But: git worktree list shows ALL worktrees
// And: git branch -vv shows tracking info across worktrees
```

**Problems with worktrees:**
1. **Shared refs** - Branches are visible across all worktrees, not truly isolated
2. **Shared object store** - Corruption in one worktree can affect others
3. **Limited to one working tree per branch** - Can't have multiple worktrees on same branch
4. **JJ incompatibility** - JJ doesn't use worktrees

#### Why Full Clones

```rust
let workspace_path = "/workspaces/agent-123";
// Each workspace is a complete .git directory
// No shared state between workspaces
// Workspace deletion removes everything
```

**Benefits:**
1. **True isolation** - No shared refs, objects, or state
2. **JJ-native** - Works with JJ's workspace concept
3. **Crash safety** - Workspace corruption doesn't affect others
4. **Flexible** - Multiple workspaces on same branch allowed

#### Trade-offs

| Aspect | Full Clones | Git Worktrees |
|--------|-------------|--------------|
| Disk space | Higher (duplicated .git) | Lower (shared objects) |
| Creation speed | Slower (full copy) | Faster (reflink) |
| Isolation | Complete | Partial (shared refs) |
| JJ support | Native | Not supported |
| Corruption risk | Isolated | Can spread |

### JJ Workspace Concept

JJ has native workspace support that maps well to our model:

```rust
// JJ workspace structure:
// .jj/             - Local JJ state (not shared)
// .jj/repo/        - Shared repository (object store, refs)
// working_copy/    - This workspace's files

// Key insight: JJ workspaces share the object store but have:
// - Separate working copies
// - Separate operation log
// - Separate view of the world

// This is PERFECT for agent isolation:
jj workspace add --name agent-123 ../repo
// Creates: working_copy/agent-123/ with its own files
// But shares: .jj/repo/ object store
```

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
┌─────────┐
│ Created │ ← Workspace initialized, files created
└────┬────┘
     │ activate()
     ▼
┌─────────┐
│ Active  │ ← Ready for work
└────┬────┘
     │
     ├──────────────────────────────┐
     │ sync()                       │ pause()
     ▼                              ▼
┌─────────┐                   ┌─────────┐
│ Syncing │                   │ Paused  │
└────┬────┘                   └────┬────┘
     │                              │
     │ (success)                    │ resume()
     ▼                              ▼
     │                    ┌─────────────────────┐
     │◄───────────────────┘                     │
     │                                          │
     │ complete()                              │ (agent dies)
     ▼                                          ▼
┌─────────────┐                         ┌─────────┐
│ Completed  │ (terminal)               │ Failed  │ (terminal)
└─────────────┘                         └─────────┘
```

---

## Variants

### Variant A: Git Worktrees (REJECTED)

```rust
struct Workspace {
    worktree_path: PathBuf,
    branch_name: BranchName,
    // Worktree is linked to main repo
}
```

**Rejected because:**
- Shared refs violate isolation requirement
- JJ doesn't support worktrees
- Not true isolation for 600+ concurrent agents

### Variant B: Full Clones with Shared Storage (CHOSEN)

```rust
struct Workspace {
    id: WorkspaceId,
    path: AbsolutePath,       // /workspaces/agent-123
    git_dir: PathBuf,        // /workspaces/agent-123/.git (full copy)
    state: WorkspaceState,
}
```

**Chosen because:**
- True isolation - no shared state
- Works with both Git and JJ
- Simple mental model
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
- Platform-specific (reflink on Linux, clone on Windows)
- Complexity not yet justified

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

/// INVARIANT: .git or .jj directory exists for non-Created workspaces
match workspace.backend {
    Git => assert!(workspace.path.join(".git").exists()),
    JJ => assert!(workspace.path.join(".jj").exists()),
}

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

### Concurrency Invariants

```rust
/// INVARIANT: At most one agent can hold a lock on a workspace
assert!(workspace.lock.holder().is_none() || workspace.lock.holder() == current_agent);

/// INVARIANT: Active workspace count doesn't exceed limit
assert!(active_workspaces().count() <= MAX_CONCURRENT_WORKSPACES);
```

---

## Consequences

### Positive

1. **True isolation** - No shared refs or object stores between workspaces
2. **JJ-native** - Works seamlessly with JJ's workspace concept
3. **Crash containment** - One workspace's corruption doesn't affect others
4. **Simple model** - Easy to reason about workspace behavior
5. **Security** - Agent A can't access Agent B's workspace directly

### Negative

1. **Disk usage** - Full .git directory per workspace (~50-100MB each)
2. **Creation time** - Full clone takes longer than worktree (~5-10 seconds)
3. **Maintenance** - Must clean up stale workspaces manually

### Scale Considerations

For 600+ concurrent agents:
- **Disk**: 600 × 100MB = 60GB (acceptable for workspace servers)
- **Creation**: Parallel creation with rate limiting
- **Cleanup**: Automatic cleanup of abandoned workspaces after timeout

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/workspace/src/domain/workspace.rs` | Workspace entity + state machine |
| `crates/workspace/src/domain/state.rs` | WorkspaceState enum + transitions |
| `crates/workspace/src/infrastructure/filesystem.rs` | Clone/cleanup operations |
| `crates/vcs/src/backend/jj.rs` | JJ workspace operations |

---

## Related ADRs

- ADR-004: VCS Abstraction (workspace operations in trait)
- ADR-001: CLI Architecture (workspace commands: spawn, switch, list, forget)
- ADR-002: Durable Workflow Execution (workspace operations with recovery)
