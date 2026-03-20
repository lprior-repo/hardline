# ADR-001: Hardline CLI Architecture Decisions

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline is a unified CLI for workspace isolation and source control, designed for:
1. AI agents as primary users (JSON-first)
2. Human developers as secondary (human-optimized output)
3. Full workspace isolation (not git worktrees - full clones)
4. Using JJ as the primary VCS with 1:1 git translation

---

## Decisions

### 1. No Shell Out - Pure Rust Implementations

**Decision:** Hardline MUST NOT shell out to external commands. All operations use pure Rust libraries.

**Rationale:**
- Predictable behavior across environments
- No dependency on git/jj binaries being installed
- Full control over error handling
- Better for AI agents (no unexpected shell behavior)

**Libraries:**
- `jj-lib` - Jujutsu's native library (primary VCS)
- `gix` (gitoxide) - Pure Rust git for git-specific operations

---

### 2. Dual Interface: AI Agents + Humans

**Decision:** Hardline defaults to JSON output (AI-first) with `-ho` flag for human output.

**Output Modes:**
```rust
// Default: JSON for AI agents
enum OutputMode {
    Json,   // Machine-parseable, deterministic
    Human,  // Pretty, colored, multi-line
}

// Global flags
--json        // JSON output (default for AI)
--ho          // Human output (default for CLI)
--quiet       // Suppress output
--verbose     // Debug info
```

**JSON Response Format:**
```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

---

### 3. Full Workspace Isolation via JJ

**Decision:** Workspaces use JJ's workspace concept for true isolation.

**Model:**
```
Repository
├── .scp/                    # Hardline metadata
│   ├── state.db            # SQLite database
│   └── workspaces/         # Workspace storage
│       ├── feature-x/     # Full isolated copy
│       └── bug-fix/        # Another workspace
└── .jj/                    # JJ store (shared)
```

**JJ Features Used:**
- Workspaces (isolated working copies)
- Bookmarks (JJ's branches, 1:1 with git)
- Operation log (undo/redo/rollback)
- Rebase and merge
- Git sync (push/fetch to git remotes)

---

### 4. Final Command Hierarchy

**Decision:** Consolidate to essential commands only.

| Category | Commands | Description |
|----------|----------|-------------|
| **Core** | `init`, `status`, `context`, `whereami`, `doctor` | Basic operations |
| **Workspace** | `spawn`, `switch`, `list`, `forget`, `update-stale` | JJ workspace management |
| **Branch** | `list`, `create`, `delete`, `set` | Bookmark management (JJ's branches) |
| **Commit** | `commit <message>` | Gathers ALL commits and commits (JJ-style) |
| **Sync** | `fetch`, `push`, `pull` | Git remote sync |
| **Operation** | `checkpoint <name>`, `undo`, `revert <name>`, `log` | Snapshot/rollback via JJ op log |
| **Queue** | `list`, `enqueue`, `dequeue`, `process` | Merge queue for workspaces |
| **Agent** | `create`, `list`, `kill`, `status`, `register`, `heartbeat` | Multi-agent awareness |
| **Maintenance** | `integrity`, `clean`, `prune-invalid`, `query`, `validate`, `whatif` | System health |

**Removed:**
- Session (duplicate of Workspace)
- Beads (removed for now)
- Stash, Tag (not core)
- Config subcommands (file-based)
- 2500+ lines of unwired dead code

**Total: ~30 commands**

---

### 5. Commit Command (JJ-Style)

**Decision:** `commit` gathers ALL changes and commits automatically.

**Behavior:**
```bash
# JJ-style: commits everything automatically
hardline commit "Fix bug in auth"

# Equivalent to:
# 1. jj status (get all changes)
# 2. jj new敛 commit (create commit with all changes)
# 3. Automatically handles conflict markers, empty commits, etc.
```

---

### 6. Operation/Checkpoint System

**Decision:** Use JJ's operation log for snapshot/rollback.

**Commands:**
```bash
hardline operation checkpoint <name>   # Create named snapshot
hardline operation undo              # Undo last operation
hardline operation revert <name>     # Revert to named checkpoint
hardline operation log              # Show operation history
```

**Implementation:** Maps to JJ's operation commands:
- `checkpoint` → `jj snapshot` (if available) or bookmark current state
- `undo` → `jj op undo`
- `revert` → `jj op restore <operation_id>`

---

### 7. Queue = Merge Queue

**Decision:** Queue manages workspaces ready to merge to main.

```bash
hardline queue list        # Show queued workspaces
hardline queue enqueue    # Add current workspace to queue
hardline queue dequeue    # Remove from queue
hardline queue process    # Process next in queue (merge to main)
```

---

### 8. Agent System for Awareness

**Decision:** Agent commands for multi-agent coordination, not work execution.

```bash
hardline agent create <name>     # Create agent identity
hardline agent list              # List active agents
hardline agent kill <id>        # Terminate agent
hardline agent status <id>      # Agent status
hardline agent register          # Register with coordination service
hardline agent heartbeat        # Send heartbeat
```

---

## Command Reference

```
hardline init [--vcs git|jj]     Initialize repository
hardline status [--short]       Show status
hardline context                Show environment context
hardline whereami               Show current location
hardline doctor [--full]        Run diagnostics

hardline workspace spawn <name>      Create workspace
hardline workspace switch <name>    Switch to workspace
hardline workspace list               List workspaces
hardline workspace forget <name>     Remove workspace
hardline workspace update-stale       Update stale workspace

hardline branch list                  List branches
hardline branch create <name>        Create branch
hardline branch delete <name>        Delete branch
hardline branch set <name>           Set current branch

hardline commit <message>            Commit all changes

hardline fetch [--all]              Fetch from remotes
hardline push [--force]             Push to remote
hardline pull                       Pull from remote

hardline operation checkpoint <name>  Create snapshot
hardline operation undo              Undo last operation
hardline operation revert <name>     Revert to checkpoint
hardline operation log               Show operation history

hardline queue list                 List queue
hardline queue enqueue              Add to queue
hardline queue dequeue              Remove from queue
hardline queue process              Process next

hardline agent create <name>        Create agent
hardline agent list                 List agents
hardline agent kill <id>           Kill agent
hardline agent status <id>          Agent status
hardline agent register             Register agent
hardline agent heartbeat            Send heartbeat

hardline integrity                  Check integrity
hardline clean                      Clean stale data
hardline prune-invalid              Remove invalid entries
hardline query <expr>              Query system
hardline validate <target>         Validate target
hardline whatif <command>           Dry run
```

---

## Consequences

### Positive
- Clean, minimal command set (~30 commands)
- AI-first (JSON default) but human-usable (-ho flag)
- True isolation via JJ workspaces
- Snapshot/rollback via JJ operation log
- Queue for ordered merging

### Negative
- JJ is the primary VCS (git is secondary/translated)
- Requires jj-lib integration (not shelling out to jj binary)

### Risks
- jj-lib API stability (mitigate: version pinning)
- Migration path for git-only users

---

## Related ADRs

- ADR-002: Workspace State Machine
- ADR-003: Queue Priority Ordering
- ADR-004: Agent Registry

---

## Notes

- VCS: JJ primary, git translated 1:1
- Workspaces: JJ workspace concept
- Checkpoints: JJ operation log snapshots
- Queue: Ordered merge of workspaces to main
- Output: JSON default, `-ho` for human
- Workflow: Event-sourced saga pattern for durable execution

---

## ADR-002: Durable Workflow Execution (Saga Pattern)

**Date:** 2026-03-20  
**Status:** Proposed  
**Existing Implementations:** Isolate, Seshat

---

### Context

Hardline needs to guarantee that ANY operation completes successfully, even if the process crashes mid-execution. This requires:

1. Event write logging (append-only event log)
2. Saga pattern for multi-step workflows
3. Full recovery from any failure point
4. Compensation (undo) on failure

This mirrors Temporal/Restate's durable execution model.

---

### Existing Implementations to Port

**From Isolate (`/home/lewis/src/isolate`):**

| Pattern | File | Description |
|---------|------|-------------|
| Domain Events | `crates/isolate-core/src/domain/events.rs` | DDD event sourcing |
| Saga Plan | `durable_tasks.jsonl` | 5-task plan for durable execution |
| Compensation | `crates/isolate/src/commands/add/atomic.rs` | Two-phase rollback |
| Event Locks | `sql_schemas/05_event_store_locks.sql` | Distributed locking |
| Recovery | `crates/isolate-core/src/recovery.rs` | Multiple recovery policies |
| Pipeline State | `crates/orchestrator/src/state.rs` | PipelineState machine |
| Pipeline Persistence | `crates/orchestrator/src/persistence.rs` | JSON file persistence |
| Pipeline Recovery | `crates/orchestrator/src/phases.rs` | `recover_pipeline()` |

**From Seshat (`/home/lewis/src/seshat`):**

| Pattern | File | Description |
|---------|------|-------------|
| Event Sourcing | `design/CRATE_SQLX_EVENT_SOURCING.md` | SQLite WAL event store |
| OperationRecord | `diagram_tool/src/store/types/durable_types.rs` | OperationState tracking |
| Step Journal | `durable_types.rs` | StepRecord with status |
| Crash Recovery | `docs/12_SINGLE_LOG_ARCHITECTURE.md` | Step journal for resume |
| Recovery Mode | `fetch.rs` | `open_recovery_mode_async` |
| Snapshots | `bootstrap.rs` | Snapshot recovery |
| LKG Fallback | `cli_persistence.rs` | Last Known Good |
| Conditional Appends | `12_SINGLE_LOG_ARCHITECTURE.md` | Human priority |

---

### Decision

**Port existing implementations from isolate and seshat, then extend for hardline.**

---

### Event Write Logging

Every operation is logged BEFORE execution:

```rust
// Event structure (from isolate events.rs)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum DomainEvent {
    WorkspaceWillSpawn { name: String },
    WorkspaceDirectoryCreated { name: String, path: PathBuf },
    VcsRepositoryInitialized { name: String },
    WorkspaceBookmarkCreated { name: String },
    WorkspaceSpawned { name: String },
    WorkspaceCompensation { name: String },
}

struct EventLog {
    events: Vec<DomainEvent>,  // Append-only
}
```

---

### Saga Pattern (from isolate atomic.rs)

Multi-step workflows with compensation:

```rust
// State machine for saga (from atomic.rs)
enum AddAtomicState {
    Start,
    DbRecordCreated,
    MetadataUpdated,
    WorkspaceCreateStarted,
    WorkspaceCreateSucceeded,
    WorkspaceRollbackStarted,
    WorkspaceRollbackSucceeded,
    DatabaseRollbackStarted,
    DatabaseRollbackSucceeded,
}

struct Saga {
    steps: Vec<SagaStep>,
    compensating: Vec<CompensationStep>,
}

impl Saga {
    fn execute(&self, ctx: &Context) -> Result<()> {
        for step in &self.steps {
            let output = (step.execute)(ctx)?;
            self.log_event(Event::StepCompleted(step.name, output))?;
            self.compensating.push(CompensationStep { step, output });
        }
        self.log_event(Event::WorkflowCompleted(self.id))?;
        Ok(())
    }

    fn compensate(&self) -> Result<()> {
        for step in self.compensating.iter().rev() {
            (step.compensate)(ctx, step.output)?;
            self.log_event(Event::StepCompensated(step.name))?;
        }
        self.log_event(Event::WorkflowCompensated(self.id))?;
        Ok(())
    }
}
```

---

### Step Journal (from seshat durable_types.rs)

Track individual steps for crash recovery:

```rust
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

pub struct StepRecord {
    pub operation_id: String,
    pub step_index: u32,
    pub step_name: String,
    pub status: StepStatus,
    pub event_revision: Option<i64>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub error_message: Option<String>,
}
```

---

### Recovery (from isolate recovery.rs)

On startup, scan for incomplete workflows:

```rust
pub enum RecoveryPolicy {
    FailFast,  // Fail on corruption
    Warn,      // Repair with warning
    Silent,    // Auto-repair silently
}

fn recover_incomplete_workflows(event_log: &EventLog) -> Vec<WorkflowRecovery> {
    let incomplete = event_log.events
        .filter(|e| matches!(e, Event::WillExecute(_) | Event::StepCompleted(_)))
        .group_by(|e| e.workflow_id())
        .filter(|(_, events)| !events.any(|e| matches!(e, Event::WorkflowCompleted(_))));

    incomplete.map(|wf_id| {
        let last_step = events.iter()
            .find_last(|e| matches!(e, Event::StepCompleted(_)));
        WorkflowRecovery { wf_id, resume_from: last_step }
    }).collect()
}
```

---

### Commands

```bash
hardline workflow list                    # List running workflows
hardline workflow status <id>           # Status of workflow
hardline workflow cancel <id>           # Cancel running workflow
hardline workflow replay <id>           # Replay from event log
hardline workflow log <id>             # Show event log for workflow
```

---

### Consequence

- All multi-step operations become durable
- Can recover from ANY failure (crash, power loss, etc.)
- Event log enables debugging and audit trail
- Compensation allows safe rollback
- Similar to Temporal/Restate but self-hosted
- **Existing implementations in isolate/seshat provide foundation**
