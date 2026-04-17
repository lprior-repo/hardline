# ADR-008: Queue Processing - Priority + FIFO Merge Queue

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline needs a merge queue to process multiple agent workspaces efficiently. The queue must:

1. **Handle priority** - Critical/High beads merge before Low/Backlog
2. **Preserve order** - FIFO within same priority (no starvation)
3. **Track state** - Each entry has a lifecycle (pending → merged/failed)
4. **Recover from failure** - Failed entries can be retried or cancelled
5. **Scale to 600+** - Concurrent agents enqueueing and dequeuing

This ADR defines the queue processing model.

---

## Decision

### Queue Entry Structure

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: QueueEntryId,
    pub workspace_id: WorkspaceId,
    pub priority: Priority,
    pub status: QueueStatus,
    pub enqueued_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub claimed_by: Option<AgentId>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub position: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical = 0,  // P0 - Security, data loss, broken builds
    High = 1,      // P1 - Major features, important bugs
    Medium = 2,    // P2 - Default
    Low = 3,       // P3 - Polish, optimization
    Backlog = 4,   // P4 - Future ideas
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Critical => "critical",
            Priority::High => "high",
            Priority::Medium => "medium",
            Priority::Low => "low",
            Priority::Backlog => "backlog",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    Pending,           // In queue, waiting
    Claimed,           // Agent picked up, processing
    Rebase,            // Rebasing onto main
    Testing,           // Running tests
    ReadyToMerge,      // All checks passed
    Merging,           // Merge in progress
    Merged,            // Successfully merged
    FailedRetryable,   // Failed but can retry
    FailedTerminal,    // Failed permanently
    Cancelled,         // Manually cancelled
}
```

### Queue State Machine

```
                    ┌─────────────────────────────────────────┐
                    │                                         │
                    ▼                                         │
┌──────────┐   claim()   ┌──────────┐   rebase()   ┌────────┐
│ Pending  │ ──────────► │ Claimed  │ ───────────► │ Rebase │
└──────────┘             └──────────┘              └────────┘
     │                        │                         │
     │ cancel()               │ abort()                 │
     ▼                        ▼                         ▼
┌───────────┐          ┌──────────┐              ┌─────────┐
│ Cancelled │          │ Failed*  │              │ Testing │
└───────────┘          └──────────┘              └────┬────┘
                                                       │
                    ┌──────────────────────────────────┘
                    │
                    ▼
             ┌──────────────┐   merge()   ┌──────────┐
             │ ReadyToMerge │ ───────────► │ Merging  │
             └──────────────┘             └────┬─────┘
                                               │
                    ┌──────────────────────────┘
                    │ success
                    ▼
             ┌──────────┐
             │ Merged   │ (terminal)
             └──────────┘
```

### Priority Ordering

```rust
impl QueueEntry {
    /// Order by: priority ASC, then enqueued_at ASC
    /// Smallest priority (0=Critical) first, oldest first within priority
    pub fn compare_queue_position(&self, other: &QueueEntry) -> Ordering {
        (self.priority, self.enqueued_at)
            .cmp(&(other.priority, other.enqueued_at))
    }
}
```

### Queue Operations

```rust
pub trait QueueRepository: Send + Sync {
    /// Add entry to queue (at correct position based on priority)
    fn enqueue(&self, entry: QueueEntry) -> Result<QueueEntry>;
    
    /// Remove entry from queue
    fn dequeue(&self, entry_id: &QueueEntryId) -> Result<()>;
    
    /// Claim next entry for processing
    fn claim_next(&self, agent_id: &AgentId) -> Result<Option<QueueEntry>>;
    
    /// Update entry status
    fn update_status(&self, entry_id: &QueueEntryId, status: QueueStatus) -> Result<()>;
    
    /// List all entries (ordered by priority + time)
    fn list(&self, filter: Option<QueueFilter>) -> Result<Vec<QueueEntry>>;
    
    /// Get entry by ID
    fn get(&self, entry_id: &QueueEntryId) -> Result<Option<QueueEntry>>;
    
    /// Reorder entry (change priority)
    fn reorder(&self, entry_id: &QueueEntryId, new_priority: Priority) -> Result<()>;
}
```

### Claim Algorithm

```rust
/// Claim the next entry for an agent.
/// Returns the highest priority entry that:
/// 1. Is in Pending status
/// 2. Has no active claims (claimed_by IS NULL)
/// 3. All dependencies are merged
pub fn claim_next(&self, agent_id: &AgentId) -> Result<Option<QueueEntry>> {
    let entries = self.list(Some(QueueFilter {
        status: Some(QueueStatus::Pending),
        include_claimed: false,
        dependencies_met: true,
    }))?;
    
    let entry = entries.into_iter().next();
    
    if let Some(entry) = entry {
        self.update_status(entry.id, QueueStatus::Claimed)?;
        self.claim(entry.id, agent_id)?;
    }
    
    Ok(entry)
}

/// Check if all dependencies are merged
fn dependencies_met(&self, entry: &QueueEntry) -> Result<bool> {
    let workspace = self.workspace_repo.get(&entry.workspace_id)?;
    let session = self.session_repo.get(workspace.session_id)?;
    
    for dep_bead_id in session.bead.dependencies() {
        let dep_bead = self.bead_repo.get(dep_bead_id)?;
        if dep_bead.state != BeadState::Merged {
            return Ok(false);
        }
    }
    
    Ok(true)
}
```

### Status Transition Validation

```rust
impl QueueStatus {
    pub fn valid_transitions(&self) -> Vec<QueueStatus> {
        match self {
            QueueStatus::Pending => vec![
                QueueStatus::Claimed,
                QueueStatus::Cancelled,
            ],
            QueueStatus::Claimed => vec![
                QueueStatus::Rebase,
                QueueStatus::FailedRetryable,
                QueueStatus::FailedTerminal,
            ],
            QueueStatus::Rebase => vec![
                QueueStatus::Testing,
                QueueStatus::FailedRetryable,
                QueueStatus::FailedTerminal,
            ],
            QueueStatus::Testing => vec![
                QueueStatus::ReadyToMerge,
                QueueStatus::FailedRetryable,
                QueueStatus::FailedTerminal,
            ],
            QueueStatus::ReadyToMerge => vec![
                QueueStatus::Merging,
                QueueStatus::FailedRetryable,
            ],
            QueueStatus::Merging => vec![
                QueueStatus::Merged,
                QueueStatus::FailedRetryable,
            ],
            // Terminal states
            QueueStatus::Merged => vec![],
            QueueStatus::FailedRetryable => vec![
                QueueStatus::Pending,  // Retry
                QueueStatus::Cancelled,
            ],
            QueueStatus::FailedTerminal => vec![
                QueueStatus::Cancelled,
            ],
            QueueStatus::Cancelled => vec![],
        }
    }
    
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            QueueStatus::Merged | QueueStatus::FailedTerminal | QueueStatus::Cancelled
        )
    }
}
```

---

## Variants

### Variant A: Pure Priority Queue (REJECTED)

```rust
// Only priority matters, no FIFO within priority
// Problem: Starvation - low priority entries may never get processed
struct PriorityQueue<T> {
    buckets: Vec<Vec<T>>,  // One bucket per priority
}
```

**Rejected because:** Low priority entries could starve.

### Variant B: Priority + Timestamp (CHOSEN)

```rust
// Priority primary, timestamp secondary (FIFO within priority)
ORDER BY priority ASC, enqueued_at ASC
```

**Chosen because:**
- No starvation (FIFO prevents it)
- Critical items always first
- Simple, predictable ordering

### Variant C: Weighted Fair Queuing (DEFERRED)

```rust
// Every entry gets some share of bandwidth
// Prevents starvation AND respects priority
```

**Deferred because:**
- Complexity not justified yet
- FIFO is good enough for current scale

---

## Invariants

### Queue Entry Invariants

```rust
/// INVARIANT: Priority is within valid range
assert!(matches!(entry.priority, Priority::Critical | Priority::High 
    | Priority::Medium | Priority::Low | Priority::Backlog));

/// INVARIANT: Claimed entries have claimed_by and claimed_at set
assert!(if entry.status == QueueStatus::Claimed {
    entry.claimed_by.is_some() && entry.claimed_at.is_some()
} else {
    true
});

/// INVARIANT: Terminal entries have no claims
assert!(if entry.status.is_terminal() {
    entry.claimed_by.is_none()
} else {
    true
});
```

### Queue Ordering Invariants

```rust
/// INVARIANT: Queue list is ordered by priority, then enqueued_at
fn assert_queue_order(entries: &[QueueEntry]) {
    for window in entries.windows(2) {
        let (a, b) = (&window[0], &window[1]);
        
        // Either priority is lower (higher priority) for a
        // Or same priority and a was enqueued first
        assert!(
            a.priority < b.priority ||
            (a.priority == b.priority && a.enqueued_at <= b.enqueued_at)
        );
    }
}

/// INVARIANT: Position values are contiguous
fn assert_contiguous_positions(entries: &[QueueEntry]) {
    let positions: Vec<u32> = entries.iter().map(|e| e.position).collect();
    let expected: Vec<u32> = (0..entries.len() as u32).collect();
    assert_eq!(positions, expected);
}
```

### Status Transition Invariants

```rust
/// INVARIANT: Status transitions are valid
fn assert_valid_transition(from: QueueStatus, to: QueueStatus) {
    assert!(
        from.valid_transitions().contains(&to),
        "Invalid transition: {:?} -> {:?}",
        from,
        to
    );
}

/// INVARIANT: Terminal states cannot transition
fn assert_terminal_no_transition(status: QueueStatus) {
    assert!(
        status.is_terminal(),
        "Non-terminal status {:?} should not transition",
        status
    );
}
```

### Claim Invariants

```rust
/// INVARIANT: At most one agent can claim an entry
fn assert_single_claim(entry: &QueueEntry) {
    // If claimed, must have exactly one claim
    assert_eq!(
        entry.claimed_by.is_some(),
        entry.status == QueueStatus::Claimed
    );
}

/// INVARIANT: Claimed entries have recent claimed_at timestamp
fn assert_recent_claim(entry: &QueueEntry) {
    if let Some(claimed_at) = entry.claimed_at {
        let age = Utc::now() - claimed_at;
        assert!(
            age < Duration::from_secs(CLAIM_TIMEOUT_SECONDS),
            "Claim is stale: {:?}",
            age
        );
    }
}

const CLAIM_TIMEOUT_SECONDS: i64 = 3600; // 1 hour
```

### Dependency Invariants

```rust
/// INVARIANT: Entry can only be dequeued when dependencies are met
fn assert_dependencies_met(entry: &QueueEntry, beads: &BeadRegistry) {
    let deps = entry.dependencies();
    for dep in deps {
        let dep_bead = beads.get(dep);
        assert!(
            dep_bead.status == BeadStatus::Merged,
            "Dependency {:?} not merged",
            dep
        );
    }
}

/// INVARIANT: Circular dependencies are prevented
fn assert_no_cycle(entries: &[QueueEntry]) -> Result<()> {
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    
    for entry in entries {
        if has_cycle(entry, &mut visited, &mut stack, entries)? {
            return Err(Error::BeadDependencyCycle { /* ... */ });
        }
    }
    Ok(())
}
```

---

## Consequences

### Positive

1. **Fair ordering** - FIFO within priority prevents starvation
2. **State machine** - Clear lifecycle for each entry
3. **Retry support** - FailedRetryable can be re-queued
4. **Dependency tracking** - Prevents merging before deps
5. **Claim mechanism** - Prevents duplicate processing

### Negative

1. **Single queue** - No separate queues per priority (could be added later)
2. **SQLite limitations** - Concurrent writes serialized

### CLI Commands

```bash
hardline queue list                    # List all queue entries
hardline queue enqueue <workspace>    # Add to queue
hardline queue dequeue <id>           # Remove from queue
hardline queue claim                  # Claim next entry
hardline queue status <id>            # Check status
hardline queue priority <id> <0-4>    # Change priority
hardline queue retry <id>             # Retry failed entry
hardline queue cancel <id>            # Cancel entry
```

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/queue/src/domain/mod.rs` | QueueEntry, QueueStatus, Priority |
| `crates/queue/src/domain/state_machine.rs` | Status transitions |
| `crates/queue/src/infrastructure/repository.rs` | SQLite persistence |
| `crates/queue/src/application/service.rs` | Queue operations |

---

## Related ADRs

- ADR-001: CLI Architecture (queue commands)
- ADR-002: Durable Workflow Execution (queue entries for operations)
- ADR-006: Database Schema (queue_entries table)
- ADR-007: Error Taxonomy (Queue errors 4xxx)
