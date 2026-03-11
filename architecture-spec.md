# SCP Architecture Specification

## Source Control Plane - Unified Architecture

**Version:** 1.0.0  
**Date:** 2026-03-10  
**Status:** Ready for Planner

---

## 1. Problem Space (Diamond 1)

### 1.1 EARS Requirements

#### Ubiquitous Requirements
- THE SYSTEM SHALL provide complete workspace isolation for 600+ AI agents operating concurrently
- THE SYSTEM SHALL guarantee zero data loss through snapshotted operations with forward/backward recovery
- THE SYSTEM SHALL abstract JJ and Git behind a unified VCS trait
- THE SYSTEM SHALL persist all state in SQLite with WAL durability
- THE SYSTEM SHALL output JSON-only for all operations to enable AI agent consumption

#### Event-Driven Requirements
- WHEN an agent spawns, THE SYSTEM SHALL create an isolated workspace with atomic bead claiming
- WHEN an agent completes, THE SYSTEM SHALL merge changes to main with conflict resolution
- WHEN main advances, THE SYSTEM SHALL auto-rebase all agent workspaces
- WHEN conflicts occur, THE SYSTEM SHALL store resolution state and allow later completion
- WHEN agents operate concurrently, THE SYSTEM SHALL prevent bead stealing through atomic claims

#### State-Driven Requirements
- WHILE an agent holds a bead, THE SYSTEM SHALL prevent other agents from claiming it
- WHILE a workspace is active, THE SYSTEM SHALL track its sync state with main
- WHILE queue entries exist, THE SYSTEM SHALL enforce priority ordering

#### Optional Requirements
- WHERE TUI is requested, THE SYSTEM SHALL provide interactive terminal UI
- WHERE GitHub integration is configured, THE SYSTEM SHALL sync PRs and CI status

### 1.2 Interview Matrix

|               | USER | DEVELOPER | OPS | SECURITY | BUSINESS |
|---------------|------|-----------|-----|----------|----------|
| CORE INTENT   | Agents don't step on each other | 600 concurrent agents with isolation | Recover from any failure state | No data corruption or loss | Main always buildable |
| ERROR CASES   | Lost work, broken main | Deadlocks, race conditions | Disk full, DB corruption | Token exposure | Revenue loss from downtime |
| EDGE CASES    | 1000 agents, network partition | Cyclic dependencies | WAL overflow | Malformed tokens | Weekend merges |
| SECURITY      | Data safety | Audit trails | Monitoring | Auth bypass, injection | Liability |
| OPERATIONS    | Instant recovery | 99.999% durability | Horizontal scaling | Access control | Cost optimization |

---

## 2. Solution Architecture (Diamond 2)

### 2.1 Target Crate Structure

```
scp/
├── Cargo.toml (workspace)
├── crates/
│   ├── cli/              # Main CLI entry point
│   ├── core/             # Core domain (existing, refactor)
│   │
│   ├── vcs/              # VCS abstraction (existing, enhance)
│   │   ├── git.rs       # Git backend
│   │   └── jj.rs        # JJ backend
│   │
│   ├── queue/            # Merge queue (existing, enhance)
│   │   ├── domain/      # QueueStatus state machine
│   │   ├── persistence # SQLite-backed queue
│   │   └── engine/     # Queue processing logic
│   │
│   ├── workspace/        # Workspace management (existing)
│   │   ├── domain/      # Workspace state machine
│   │   └── persistence # Workspace storage
│   │
│   ├── session/         # NEW: Session management (from isolate)
│   │   ├── domain/     # Session aggregate
│   │   └── persistence # Session storage
│   │
│   ├── bead/            # Task/bead tracking (existing, enhance)
│   │   ├── domain/    # Bead aggregate, claims
│   │   └── persistence
│   │
│   ├── stack/           # NEW: Stacked PRs (from stax)
│   │   ├── domain/    # StackBranch, parent-child relationships
│   │   ├── github/   # GitHub integration
│   │   └── engine/   # Stack operations
│   │
│   ├── worktree/        # NEW: Worktree management (from stax)
│   │   ├── dev/       # Developer worktrees
│   │   └── agent/     # Agent worktrees
│   │
│   ├── snapshot/        # NEW: Snapshot/undo system (from stax)
│   │   ├── domain/    # Snapshot state
│   │   └── storage   # Git ref-based backup
│   │
│   ├── tui/             # NEW: Terminal UI (from stax)
│   │   ├── views/     # Stack tree, diff, details
│   │   └── input/    # Key bindings
│   │
│   ├── server/          # NEW: Agent coordination API (from isolate twins)
│   │   └── api/       # HTTP API for swarm
│   │
│   └── orchestrator/    # NEW: Multi-step workflows (from isolate)
│       └── phases/     # Phase execution
│
└── sql_schemas/        # All DB schemas
```

### 2.2 Core Domain Types

#### Session (from isolate)
```
SessionState: Created -> Active -> Syncing -> Synced -> Paused -> Completed/Failed
Session {
  id: SessionId,
  name: SessionName,
  workspace: WorkspaceId,
  bead: BeadId,
  state: SessionState,
  created_at: DateTime<Utc>,
}
```

#### Workspace (from isolate)
```
WorkspaceState: Created -> Working -> Ready -> Merged/Abandoned/Conflict
Workspace {
  id: WorkspaceId,
  path: AbsolutePath,
  vcs_backend: BackendType,  // Git or JJ
  state: WorkspaceState,
  session: SessionId,
}
```

#### Bead (from isolate)
```
BeadState: Open -> Claimed -> InProgress -> Ready -> Merged/Abandoned
Bead {
  id: BeadId,           // bd-xxx format
  title: String,
  description: Option<String>,
  state: BeadState,
  claimed_by: Option<AgentId>,
  dependencies: Vec<BeadId>,
}
```

#### QueueEntry (from stak)
```
QueueStatus: Pending -> Claimed -> Rebasing -> Testing -> ReadyToMerge -> Merging -> Merged
             -> FailedRetryable -> FailedTerminal -> Cancelled

QueueEntry {
  id: QueueEntryId,
  session: SessionId,
  priority: u8,
  enqueued_at: DateTime<Utc>,
  status: QueueStatus,
}
```

#### StackBranch (from stax)
```
StackBranch {
  name: BranchName,
  parent: Option<BranchName>,
  pr_info: Option<PrInfo>,     // GitHub PR
  revision: CommitHash,        // Last known good
}
```

### 2.3 VCS Abstraction (Unified Interface)

```rust
trait VcsBackend {
    // Repository
    fn init(&self, path: &Path) -> Result<()>;
    fn clone(&self, url: &str, path: &Path) -> Result<()>;
    
    // Branches
    fn current_branch(&self) -> Result<Option<BranchName>>;
    fn list_branches(&self) -> Result<Vec<BranchName>>;
    fn create_branch(&self, name: &BranchName) -> Result<()>;
    fn delete_branch(&self, name: &BranchName) -> Result<()>;
    
    // Commits
    fn status(&self) -> Result<VcsStatus>;
    fn add(&self, paths: &[&Path]) -> Result<()>;
    fn commit(&self, message: &str) -> Result<CommitHash>;
    fn log(&self, count: usize) -> Result<Vec<Commit>>;
    
    // Workspaces (JJ-specific, no-op for Git)
    fn workspace_create(&self, name: &str) -> Result<()>;
    fn workspace_list(&self) -> Result<Vec<WorkspaceInfo>>;
    fn workspace_switch(&self, name: &str) -> Result<()>;
    
    // Operations
    fn rebase(&self, onto: &BranchName) -> Result<()>;
    fn merge(&self, source: &BranchName) -> Result<()>;
    
    // Operation log (JJ-specific)
    fn operation_log(&self) -> Result<Vec<Operation>>;
    fn undo(&self, operation_id: &str) -> Result<()>;
}
```

---

## 3. KIRK Contracts

### 3.1 Session Management

**Preconditions:**
- `Session::create()` requires: valid SessionName, existing Workspace
- `Session::transition_to(Active)` requires: state == Created
- `Session::claim_bead()` requires: bead.state == Open

**Postconditions:**
- `Session::create()` returns: Session with state == Created
- `Session::complete()` returns: Session with state == Completed OR Failed

**Invariants:**
- Session.state is always valid for Session type
- Workspace.id is always valid if Session.workspace is Some

### 3.2 Queue Processing

**Preconditions:**
- `Queue::enqueue()` requires: entry.status == Pending
- `Queue::transition()` requires: valid state transition per QueueStatus

**Postconditions:**
- `Queue::enqueue()` returns: new Queue with entry added at correct priority position
- `Queue::dequeue()` returns: (Queue, Option<QueueEntry>) where entry has lowest priority

**Invariants:**
- All entries sorted by priority ascending
- No duplicate session IDs
- Status transitions are valid

### 3.3 Stack Operations

**Preconditions:**
- `Stack::restack()` requires: no conflicts in working copy
- `Stack::push()` requires: all parent PRs merged or in queue

**Postconditions:**
- `Stack::restack()` returns: new Stack with all branches rebased
- `Stack::cascade()` returns: Stack with all PRs submitted

---

## 4. Error Taxonomy

### 4.1 Error Codes

| Range | Category | Examples |
|-------|----------|----------|
| 1xxx | Workspace | WorkspaceNotFound, WorkspaceLocked, WorkspaceCorrupt |
| 2xxx | Session | SessionNotFound, SessionAlreadyActive, SessionExpired |
| 3xxx | Bead | BeadNotFound, BeadAlreadyClaimed, BeadDependencyCycle |
| 4xxx | Queue | QueueFull, QueuePriorityConflict, QueueStaleEntry |
| 5xxx | VCS | VcsNotFound, VcsConflict, VcsDetachedHead |
| 6xxx | Stack | StackNotFound, StackOrphaned, StackCyclicDependency |
| 7xxx | GitHub | GitHubAuthFailed, GitHubPrClosed, GitHubRateLimited |
| 8xxx | Snapshot | SnapshotNotFound, SnapshotCorrupt, SnapshotExpired |
| 9xxx | Internal | InternalError, DatabaseCorrupt, UnexpectedNull |

### 4.2 Error Handling Patterns

```rust
// All errors implement this trait
trait ScpError: std::error::Error + Send + Sync {
    fn code(&self) -> u16;
    fn category(&self) -> ErrorCategory;
    fn fix(&self) -> Option<Fix>;
    fn is_retryable(&self) -> bool;
}

struct Fix {
    command: String,
    description: String,
    risk: FixRisk,  // Safe, Moderate, Dangerous
}
```

---

## 5. Inversion Analysis

### 5.1 Security Inversions

| Trigger | Expected | Error Variant |
|---------|----------|---------------|
| Invalid token | 401 response | GitHubAuthFailed |
| Expired token | 401 response | GitHubTokenExpired |
| Rate limited | 429 response | GitHubRateLimited |
| SQL injection | Reject input | InputValidationError |
| Path traversal | Reject path | PathValidationError |

### 5.2 Usability Inversions

| Trigger | Expected | Error Variant |
|---------|----------|---------------|
| Workspace not found | Clear error | WorkspaceNotFound |
| Bead not found | Clear error | BeadNotFound |
| Queue empty | Empty result | QueueEmpty |
| No PR for branch | None returned | PrNotFound |

### 5.3 Integration Inversions

| Trigger | Expected | Error Variant |
|---------|----------|---------------|
| Network timeout | Retry with backoff | NetworkTimeout |
| JJ not installed | Clear install guide | VcsNotInstalled |
| Disk full | Error + cleanup suggestion | IoError(DiskFull) |
| Concurrent write | Retry or queue | WorkspaceLocked |

---

## 6. Second-Order Consequences

### 6.1 Agent Spawn Cascade

1. **First-order:** Agent spawn creates workspace
2. **Second-order:** Bead claimed, other agents blocked
3. **Third-order:** Session state tracked, heartbeat starts
4. **Fourth-order:** If agent dies, heartbeat timeout triggers cleanup
5. **Verification:** Check bead state, session state, workspace existence

### 6.2 Merge Queue Cascade

1. **First-order:** Entry transitions to ReadyToMerge
2. **Second-order:** CI status polled
3. **Third-order:** On success, merge executes
4. **Fourth-order:** All child stacks rebased
5. **Verification:** Check main branch, child branch parents, CI history

### 6.3 Restack Cascade

1. **First-order:** Snapshot created
2. **Second-order:** Each branch rebased in topological order
3. **Third-order:** Git refs backed up
4. **Fourth-order:** If failure, rollback from snapshot
5. **Verification:** Check branch ancestry, parent links, PR states

---

## 7. Pre-Mortem Analysis

### 7.1 Failure Modes

| Scenario | Probability | Severity | Mitigation |
|----------|-------------|----------|------------|
| DB corruption | LOW | CRITICAL | WAL + backup + restore |
| Agent 600+ deadlock | MEDIUM | HIGH | Lock-free JJ backend |
| Network partition | MEDIUM | MEDIUM | Queue persisted locally |
| Disk full during merge | LOW | HIGH | Pre-check disk space |
| GitHub rate limit | HIGH | MEDIUM | Exponential backoff |
| Cyclic bead dependency | LOW | HIGH | Compile-time cycle detection |
| Lost commits | VERY LOW | CRITICAL | JJ operation log |

### 7.2 Recovery Procedures

| Failure | Recovery |
|---------|----------|
| DB corrupt | Restore from WAL checkpoint |
| Lost commits | JJ operation log recovery |
| Stuck agent | Force-complete via session management |
| Broken stack | Snapshot rollback + manual intervention |
| GitHub token | Re-authenticate + retry queue |

---

## 8. Migration Strategy

### Phase 1: Stabilize SCP (Week 1)
- Fix broken crates: scenarios, orchestrator, twins
- Ensure current code compiles with zero warnings
- Apply DDD structure to existing code
- Study reference implementations (effectum, triagebot, git-stack)

### Phase 2: Consolidate Crates (Week 2)
- Move isolate-core → crates/session
- Move isolate → crates/cli (integrate commands)
- Move stak-core → crates/queue
- Move stak → crates/cli (integrate commands)
- Apply functional Rust rules (zero unwrap, no mut)

### Phase 3: Add Stax Features (Week 3)
- Create crates/stack with GitHub integration
- Create crates/tui for terminal UI
- Create crates/snapshot for undo/redo
- Apply Scott Wlaschin DDD strictly
- Migrate git2 → gix (gitoxide)

### Phase 4: Integration (Week 4)
- Unify error handling across all crates
- Ensure single binary with all commands
- Test 600+ agent scenarios
- Enforce file/function line limits

### Phase 5: Polish (Week 5)
- Add comprehensive tests (Testing Trophy)
- Generate documentation
- Release v1.0

---

## 13. Non-Functional Requirements

### 13.1 Code Standards

All code follows:
- **Functional Rust**: Data → Calc → Actions, no mut, zero unwrap/panic
- **Scott Wlaschin DDD**: Domain layer (pure) → Application layer → Infrastructure
- **Railway-Oriented Programming**: All functions return `Result<T, Error>`
- **Bitter Truth**: Output-focused, disposable code, massive compute for solution space

### 13.2 File/Function Limits
- **Max 300 lines per file** - enforced by linter
- **Max 40 lines per function** - enforced by linter

### 13.3 DDD Layer Structure
```
domain/           # Pure types, state machines, no I/O
├── entities/   # Aggregate roots
├── value_objects/
├── events/
└── state/      # State machines
application/     # Use cases, orchestrates domain
infrastructure/ # DB, VCS, network I/O
api/            # HTTP/CLI endpoints
```

### 13.4 Functional Rust Rules

```rust
// ENFORCED:
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

// ZERO unwrap/panic - use:
match result {
    Ok(v) => v,
    Err(e) => return Err(e),
}

// NO mut - use:
let new_state = old_state.transition(...);

// NO primitive obsession:
struct BeadId(String);     // not String
struct SessionName(String); // not String
enum WorkspaceState { Created, Working, Ready, Merged }  // not Option + bool
```

### 13.5 Moon CI/CD (with Remote Cache)

```yaml
# .moon/tasks.yml - based on isolate's battle-tested config

$schema: "https://moonrepo.dev/schemas/tasks.json"

tasks:
  # ========================================================================
  # STAGE 1: CODE FORMATTING & LINTING
  # ========================================================================

  fmt:
    command: "cargo fmt --all --check"
    inputs: ["crates/**/*.rs", "Cargo.toml", "rustfmt.toml"]
    options:
      cache: true
      runInCI: true

  fmt-fix:
    command: "cargo fmt --all"
    inputs: ["crates/**/*.rs"]
    options:
      cache: false

  clippy:
    command: "cargo clippy --workspace --all-targets -- -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -A clippy::missing_const_for_fn -W clippy::pedantic"
    description: "Lint with Clippy (strict mode)"
    deps: ["check"]
    inputs: ["crates/**/*.rs", "Cargo.toml", ".clippy.toml"]
    options:
      cache: true

  deny:
    command: "cargo deny check"
    options:
      cache: true

  # ========================================================================
  # STAGE 2: TESTING
  # ========================================================================

  test:
    command: "cargo nextest run --workspace --all-features"
    deps: ["build"]
    inputs: ["crates/**/*.rs", "Cargo.toml"]
    options:
      cache: true
      runInCI: true

  test-doc:
    command: "cargo test --doc"
    deps: ["build"]
    options:
      cache: true

  # ========================================================================
  # STAGE 3: BUILD
  # ========================================================================

  check:
    command: "cargo check --workspace --all-features"
    inputs: ["crates/**/*.rs", "Cargo.toml"]
    options:
      cache: true

  build:
    command: "cargo build --release --workspace"
    deps: ["check"]
    outputs: ["/target/release/scp"]
    options:
      cache: true
      runInCI: true

  build-docs:
    command: "cargo doc --no-deps --document-private-items"
    deps: ["build"]
    options:
      cache: true

  # ========================================================================
  # STAGE 4: SECURITY & QUALITY
  # ========================================================================

  audit:
    command: "cargo audit"
    options:
      cache: false

  semgrep:
    command: "semgrep scan --config auto"
    inputs: ["crates/**/*.rs"]
    options:
      cache: true

  # ========================================================================
  # STAGE 5: COVERAGE
  # ========================================================================

  coverage:
    command: "cargo llvm-cov --workspace --fail-under-lines 90"
    deps: ["build"]
    options:
      cache: false
      runInCI: true

  # ========================================================================
  # STAGE 6: LINE LIMITS (Architectural Drift)
  # ========================================================================

  check-lines:
    command: "./scripts/check-line-limits.sh"
    inputs: ["crates/**/*.rs"]
    options:
      cache: false

  # ========================================================================
  # COMPOSITE PIPELINES
  # ========================================================================

  quick:
    command: "cargo fmt --all --check && cargo clippy --workspace -- -D warnings"
    options:
      cache: false

  ci:
    command: "cargo fmt --all --check && cargo clippy --workspace -- -D warnings && cargo nextest run --workspace"
    options:
      cache: false
      runInCI: true

  # ========================================================================
  # CONVENIENCE
  # ========================================================================

  install:
    command: "cargo build --release --bin scp && cp target/release/scp ~/.local/bin/scp"
    deps: ["build"]
    options:
      cache: false

  clean:
    command: "cargo clean"
    options:
      cache: false
```

**Moon config benefits:**
- Remote cache via moonrepo (your existing setup)
- Task dependencies auto-resolved
- Parallel execution
- `moon run :ci` = full pipeline
- `moon run :quick` = fast feedback
- Custom line limit enforcement
- Semantic grep for security

### 13.6 Testing Strategy (Testing Trophy)

All tests follow:
- **Kent Beck TDD**: Tests are isolated, deterministic, read like a story
- **Dan North BDD**: Given-When-Then structure, names describe behavior
- **Dave Farley ATDD**: Tests as executable specs, separate intent from execution
- **Testing Trophy**: Real execution first, integration/E2E heavy

**Test requirements:**
- Every state transition tested
- Every error variant tested
- Happy path + unhappy path + edge cases
- Property-based tests for invariants

---

## 14. Architectural Drift Enforcement

### 14.1 File/Function Line Limits

```bash
# scripts/check-lines.sh
find crates -name "*.rs" -exec wc -l {} \; | awk '$1 > 300 { print $2 }'
```

### 14.2 DDD Layer Enforcement

```rust
// In domain layer - compile-time checks:
// No tokio, sqlx, reqwest imports allowed
#[cfg(test)]
mod tests {
    // Domain tests only - no I/O
}

// infrastructure/ layer can depend on tokio, sqlx
// domain/ layer has ZERO external crate dependencies
```

### 14.3 Clippy Lints for DDD

```toml
[lints.rust]
# Structural
max_size_of_struct = "warn"
struct_excessive_bools = "warn"
struct_field_names_should_be_sentence_case = "allow"

# Functional
unused_must_use = "deny"
unwrap_used = "deny"
panicking_doc_comments = "deny"

# DDD
option_as_state = "deny"
fn_params_excessive_bools = "deny"
```

### 14.4 Automated Gates

```yaml
# .moon/tasks.yml
tasks:
  check-lines:
    command: ./scripts/check-line-limits.sh
    fail: true

  check-ddd:
    command: cargo check --message-format=json | jq -s 'map(select(.reason == "compiler-message"))'

  architectural-drift:
    deps: [check-lines, check-ddd, lint, fmt]
    command: echo "Architectural gates passed"
```

---

## 15. Library Strategy

### 15.1 Core Dependencies (Already Present)

| Library | Purpose | Status |
|---------|---------|--------|
| tokio | Async runtime | Keep |
| sqlx | DB + async | Keep |
| petgraph | DAG algorithms | Keep |
| jj-lib | JJ VCS integration | Keep |
| thiserror | Error enums | Keep |
| anyhow | Boundary errors | Keep |
| serde | Serialization | Keep |
| chrono | Time | Keep |
| clap | CLI | Keep |
| im, rpds | Persistent data structures | Keep |
| tap | Method chaining | Keep |

### 15.2 Replace: git2 → gix (gitoxide - Pure Rust)

```toml
# BEFORE (C dependency)
git2 = "0.19"

# AFTER (Pure Rust - gitoxide)
gix = "0.78"
```

**gitoxide (gix) benefits:**
- Pure Rust, no C dependencies (removes libssh2, openssl-sys)
- Faster compile times
- Better error messages
- Actively maintained
- Used by Jujutsu itself

### 15.3 Suggested Additions

| Library | Purpose | Benefit |
|---------|---------|---------|
| **nom** | Parsing combinators | Replace regex for performance |
| **proptest** | Property-based testing | Exhaustive test generation |
| **criterion** | Benchmarking | Performance validation |
| **pretty_assertions** | Better test diffs | Easier debugging |
| **tempfile** | Test temp files | Safe test I/O |
| **cargo-deny** | Dependency audits | Block unsafe deps |
| **cargo-mutants** | Mutation testing | Verify test quality |
| **insta** | Snapshot testing | Lock API contracts |
| **kani** | Model checking | Proofs for invariants |
| **dylint** | Custom lints | Architectural enforcement |

### 15.4 Consider for Future

| Library | Purpose | When Needed |
|---------|---------|-------------|
| **restate** | Durable execution | If implementing long-running workflows |
| **ratatui** | TUI | When building interactive UI |
| **octocrab** | GitHub API | When GitHub integration needed |

### 15.5 Remove/Batch

| Library | Action | Reason |
|---------|--------|--------|
| askama | Remove if unused | Template overhead |
| kdl | Keep if used | Good for config |
| fs2 | Remove | Use std::fs |
| walkdir | Keep | Useful |

---

## 16. Reference Implementations to Study

### 16.1 Queue/Merge Queue

| Repo | Purpose | What to Steal |
|------|---------|---------------|
| **rust-lang/triagebot** | Rust's merge queue | GitHub PR queue state machine, async handlers |
| **dimfeld/effectum** | SQLite-backed local queue | Worker loop, job locking, pending→complete |
| **ayys/fang** | Background job processor | WorkerPool, Runnable trait, concurrent tasks |

### 16.2 Stacked Branches

| Repo | Purpose | What to Steal |
|------|---------|---------------|
| **git-stack** (epage) | Local stacked branch management | Git DAG manipulation, rebase automation |
| **jj-spr** | JJ + GitHub PR bridge | JJ commits → Stacked PRs conversion |
| **arxanas/git-branchless** | Git branchless workflow | DAG manipulation, conflict handling |

### 16.3 Implementation Patterns

**Queue Worker (from effectum):**
```rust
// Pending -> Processing -> Complete loop
async fn worker_loop(&self) -> Result<()> {
    loop {
        let job = self.db.claim_next_pending().await?;
        if let Some(job) = job {
            self.process(job).await?;
            self.db.mark_complete(job.id).await?;
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
```

**GitHub Integration (from triagebot):**
```rust
// Listen for PR events, update queue state
async fn handle_pull_request(&self, event: PullRequestEvent) -> Result<()> {
    match event.action {
        Action::Opened | Action::Synchronize => self.queue.enqueue(event.pr),
        Action::Closed => self.queue.mark_merged(event.pr),
        _ => Ok(()),
    }
}
```

**Stack Management (from git-stack):**
```rust
// Rebase stack in topological order
fn restack(&self, stack: &Stack) -> Result<()> {
    for branch in stack.topological_order() {
        self.git.rebase(branch, stack.parent(branch))?;
    }
}
```

---

## 16. VCS Abstraction (Pure Rust)

### 16.1 JJ Backend (jj-lib)

```rust
// Pure Rust - jj-lib handles operation log
use jj_lib::repo::Repo;
use jj_lib::workspace::Workspace;
```

### 16.2 Git Backend (gix - Pure Rust)

```rust
// Replace git2 with gix (pure Rust)
use gix::Repository;
use gix::actor::Actor;
```

### 16.3 Unified Trait

```rust
pub trait VcsBackend {
    // Workspaces - JJ specific
    fn workspace_create(&self, name: &str) -> Result<WorkspaceId>;
    fn workspace_list(&self) -> Result<Vec<WorkspaceInfo>>;
    fn workspace_switch(&self, id: &WorkspaceId) -> Result<()>;
    
    // Branches - common
    fn current_branch(&self) -> Result<Option<BranchName>>;
    fn create_branch(&self, name: &BranchName) -> Result<()>;
    fn delete_branch(&self, name: &BranchName) -> Result<()>;
    
    // Commits - common
    fn status(&self) -> Result<VcsStatus>;
    fn commit(&self, message: &str) -> Result<CommitHash>;
    fn log(&self, count: usize) -> Result<Vec<Commit>>;
    
    // Operation log - JJ only
    fn operation_log(&self) -> Result<Vec<Operation>>;
    fn undo(&self, operation_id: &str) -> Result<()>;
}
```

---

## 18. Reduced Maintenance Strategies

### 18.1 Zero-Maintenance Patterns

1. **Immutable data structures** - No defensive copying, rpds/im handle it
2. **Parser combinators (nom)** - Generate parsers, not maintain them
3. **Property-based tests** - Test invariants, not every case
4. **Contract tests** - Test interfaces, not implementations

### 18.2 Code Generation

```rust
// Generate error variants from macro
scp_error! {
    SessionNotFound(SessionId),
    WorkspaceNotFound(WorkspaceId),
    BeadNotFound(BeadId),
}

// Generate state transitions
state_machine! {
    WorkspaceState: Created -> Working -> Ready -> Merged
                                   \-> Conflict -> Working
                                   \-> Abandoned
}
```

### 18.3 Batched Changes

- **Trunk-based flow**: Small changes, daily merges
- **Delete aggressively**: Code is liability
- **Regenerate over maintain**: If complex, regenerate

### 18.4 Observability

```rust
// Structured logging only in infrastructure
tracing::info!(session_id = %id, state = ?state, "Session state changed");

// Domain: pure, no logging
fn transition(&self, event: Event) -> Result<State, Error> { ... }
```

---

## 19. Engineering Harness for AI Agents

### 19.1 Crate Boundaries as Architectural Walls

```text
workspace/
├── Cargo.toml              # [workspace] members
├── moon.toml               # Moon workspace config
├── .moon/
│   └── tasks.yml          # Task definitions with caching
├── crates/
│   ├── domain/             # Pure business logic, no I/O
│   ├── application/        # Use cases, orchestration
│   ├── infrastructure/     # DB, HTTP, external services
│   └── cli/                # CLI handlers
├── deny.toml               # cargo-deny config
└── .cargo/mutants.toml    # cargo-mutants config
```

```toml
# moon.toml
projects = ['crates/*']
tasksTag = ':'

[environments]
ci = { platform = 'linux' }
local = { platform = 'linux' }
```

**DDD layer enforcement:**
- Domain crate's `Cargo.toml` lists ZERO dependencies
- Application depends only on domain
- Infrastructure can depend on anything
- Compiler enforces: if domain references infra → compilation fails

### 19.2 Typestate Pattern (Invalid States Unrepresentable)

```rust
use std::marker::PhantomData;

struct Created;
struct Working;
struct Ready;
struct Merged;

struct Workspace<S> {
    id: WorkspaceId,
    name: WorkspaceName,
    _state: PhantomData<S>,
}

impl Workspace<Created> {
    fn activate(self) -> Workspace<Working> {
        Workspace { id: self.id, name: self.name, _state: PhantomData }
    }
}

impl Workspace<Working> {
    fn mark_ready(self) -> Workspace<Ready> { ... }
}

// Workspace<Created>.mark_ready() won't compile - invalid transition!
```

**Benefits:**
- Invalid transitions are compile errors
- IDE suggests only valid methods for current state
- Zero runtime cost (PhantomData is zero-sized)

### 19.3 Railway-Oriented Programming

```rust
async fn handle_create_session(input: CreateSessionInput) -> Result<Session, UseCaseError> {
    let validated = validate_input(input)?;
    let checked = check_bead_available(validated, &self.repo).await?;
    let session = build_session(checked)?;
    save_session(session, &self.repo).await
}
```

### 19.4 Cargo Lints for Enforcement

```toml
# Cargo.toml
[lints.rust]
unsafe_code = "forbid"
unwrap_used = "deny"
panic = "deny"
todo = "deny"
unused_must_use = "deny"

[workspace.lints.rust]
# Domain: strict
domain = { level = "deny", priority = 10 }

# Infrastructure: relaxed
infrastructure = { level = "warn", priority = 1 }
```

### 19.5 Mutation Testing (cargo-mutants)

```toml
# .cargo/mutants.toml
examine_globs = ["crates/domain/**/*.rs"]
exclude_re = ["impl Debug", "impl Display"]
```

```bash
# CI gate - fail if any mutant survives
cargo mutants --in-diff pr.diff
```

### 19.6 Snapshot Testing (insta)

```rust
#[test]
fn test_session_json_output() {
    let session = Session { id: SessionId::new(), ... };
    insta::assert_json_snapshot!(session);
}
```

### 19.7 Custom Lints (dylint)

```rust
// lints/no_unwrap_in_domain/src/lib.rs
dylint_linting::declare_lint! {
    pub NO_UNWRAP_IN_DOMAIN, Deny,
    "`.unwrap()` is forbidden in domain layer"
}
```

### 19.8 Ralph Wiggum Loop (Deterministic Verification)

```bash
# Verify script - deterministic cage for agents
# Run via: moon run :ci

#!/bin/bash
set -e

echo "Running verification..."

# 1. Format (always run)
moon run :fmt

# 2. Lint (cached)
moon run :clippy

# 3. Dependency audit (cached)  
moon run :deny

# 4. Type check (cached)
moon run :check

# 5. Tests (cached)
moon run :test

# 6. Architectural drift (line limits)
moon run :check-lines

# 7. Mutations (expensive, skip in fast path)
moon run :mutants || echo "Mutation check failed"

# 8. Coverage
moon run :coverage

echo "✓ All gates passed"
```

```json
// .claude/hooks.json - Claude Code hooks
{
  "hooks": {
    "Stop": [{
      "hooks": [{ "type": "command", "command": "moon run :ci" }]
    }]
  }
}
```

**Ralph Wiggum Loop pattern (from continuous-deployment skill):**
```bash
# After every small slice:
jj diff
moon run :ci

# If ci fails unrelated:
moon run :<crate>:test

# Keep jj diff tiny - validate often
```

### 19.9 AGENTS.md Structure

```markdown
# SCP - Source Control Plane

## Architecture
- DDD layers: domain/ → application/ → infrastructure/ → cli/
- Dependency rule: inner NEVER depend on outer
- All code: Functional Rust, zero unwrap/panic

## Commands
- Build: `cargo build --workspace`
- Test: `cargo nextest run`
- Verify: `./verify.sh`

## Conventions
- Error handling: thiserror in domain, anyhow in shell
- State machines: use typestate pattern
- Tests: BDD naming, property-based for invariants

## Gotchas
- Don't use .unwrap() in domain crate
- Don't import infrastructure in domain
- All functions return Result<T, Error>
```

### 19.10 Verification Stack Summary

| Layer | Tool | What it Catches |
|-------|------|-----------------|
| 1 | Crate boundaries | Dependency violations |
| 2 | Compiler | Type errors |
| 3 | cargo fmt | Style |
| 4 | cargo clippy | Idioms |
| 5 | cargo dylint | Custom rules |
| 6 | cargo nextest | Unit/property tests |
| 7 | insta | API drift |
| 8 | cargo-mutants | Weak tests |
| 9 | cargo llvm-cov | Dead code |
| 10 | verify.sh hook | Pre-commit cage |
```

### 13.5 Performance
- Agent spawn: < 100ms
- Queue processing: < 50ms per entry
- Stack restack: < 1s per branch
- TUI render: 60fps

### 13.2 Durability
- **Single SQLite database** with async tokio runtime
- WAL mode with periodic checkpoints for crash recovery
- Batched non-critical writes for performance
- fsync on critical commits
- Event-sourcing via append-only operation_log table
- Doctor command for integrity checking and recovery
- PostgreSQL migration path supported (same schema)
- 99.999% data durability

### 13.3 Scalability
- 600+ concurrent agents
- 1000+ stacked branches
- 10,000+ queue entries
- Horizontal scaling via worktree isolation

### 13.4 Security
- No secrets in logs
- Input validation at all boundaries
- SQL injection prevention
- Token encryption at rest

---

## 10. Open Risks

| Risk | Status | Resolution |
|------|--------|------------|
| JJ vs Git worktree model difference | OPEN | Abstract at VCS trait level |
| 600+ agent coordination overhead | OPEN | Stateless server, local persistence |
| GitHub API rate limiting | OPEN | Aggressive caching, queuing |
| Stack rebase performance | OPEN | Parallel rebase, DAG optimization |
| Database corruption | LOW | WAL + doctor command |
| WAL unbounded growth | MEDIUM | Auto-checkpoint + monitoring |
| Stale agent detection | MEDIUM | Heartbeat timeout |
| Cross-agent visibility | HIGH | Agent registry + JSON API |

---

## 11. Durability Architecture

### 11.1 Single Database Design

```
Single SQLite database at: .scp/state.db (or PostgreSQL when scaled)

Tables:
├── sessions          # Agent session state
├── workspaces       # Workspace lifecycle
├── beads            # Task/bead tracking + claims
├── queue_entries    # Merge queue
├── stack_branches   # Stacked PR metadata
├── snapshots       # Undo/redo backups
├── agent_heartbeat # Agent health tracking
├── operation_log   # All state changes (event sourcing)
└── locks           # Distributed lock tracking
```

### 11.2 SQLite Configuration

```rust
// WAL mode for concurrency
// Batched writes for performance
// Async with tokio

let pool = SqlitePool::connect(&db_url).await?;
sqlx::query("PRAGMA journal_mode = WAL")
    .execute(&pool).await?;
sqlx::query("PRAGMA synchronous = NORMAL")
    .execute(&pool).await?;
```

### 11.3 Durability Guarantees

| Operation | Guarantee | Implementation |
|-----------|-----------|----------------|
| Session create | Atomic | DB transaction |
| Bead claim | Atomic | UPDATE with WHERE state=Open |
| Queue enqueue | Atomic | DB transaction + unique constraint |
| State change | Event-sourced | Append-only operation_log |
| Snapshot | Atomic | Git ref + receipt in DB |

### 11.4 Doctor Command

```rust
enum DoctorCheck {
    DatabaseIntegrity,      // PRAGMA integrity_check
    WalHealth,             // wal_autocheckpoint OK
    StaleSessions,          // heartbeat_timeout exceeded
    OrphanedWorkspaces,     # no session references
    OrphanedBeads,          # no session claims
    QueueStuck,             # status not changed > 24h
    SnapshotCorrupt,        # git ref missing
    VcsStateMismatch,       # JJ/Git out of sync
}

struct DoctorReport {
    checks: Vec<DoctorCheckResult>,
    severity: Severity,  // Healthy, Warning, Critical
    fixes: Vec<Fix>,     // Auto-fixable issues
}
```

**Auto-fix capabilities:**
- Prune stale sessions
- Clean orphaned workspaces
- Reset stuck queue entries
- Rebuild WAL from corruption
- Restore snapshots

---

## 12. Observability for AI Agents

### 12.1 Agent Registry

```rust
struct AgentRegistry {
    // What is each agent doing?
    fn get_agent_state(agent_id: &AgentId) -> AgentState;
    
    // List all active agents
    fn list_agents() -> Vec<AgentSummary>;
    
    // Who is working on what?
    fn list_work distribution() -> WorkDistribution;
    
    // Detect conflicts
    fn detect_overlap(agent_id: &AgentId) -> Vec<Conflict>;
}
```

### 12.2 Agent State Query

```rust
struct AgentState {
    agent_id: AgentId,
    session: Option<SessionId>,
    bead: Option<BeadId>,
    workspace: Option<WorkspaceId>,
    status: AgentStatus,  // Idle, Working, Blocked, Failed
    heartbeat: DateTime<Utc>,
    progress: Option<Progress>,
}

enum AgentStatus {
    Idle,
    Working { task: String },
    Blocked { reason: String, waiting_on: Option<AgentId> },
    Failed { error: String },
}
```

### 12.3 Cross-Agent Communication

```rust
// Agent can query: "what is agent X doing?"
fn get_agent_observation(observer: &AgentId, target: &AgentId) -> Observation;

// Agent can page another agent
fn page_agent(pager: &AgentId, target: &AgentId, message: &str) -> Result<()>;

// Agent can subscribe to workspace events
fn subscribe_to_workspace(agent: &AgentId, workspace: &WorkspaceId) -> mpsc::Receiver<WorkspaceEvent>;
```

### 12.4 JSON Output for Agent Observability

```bash
# List all agents and their states
scp agent list --json

# What is specific agent doing?
scp agent status bd-123 --json

# Who is working on what (work distribution)
scp agent work-distribution --json

# Page an agent
scp agent page bd-123 "Hey, you're stepping on my files"

# Watch workspace for changes
scp agent watch workspace-abc --json
```

### 12.5 Heartbeat System

```rust
struct Heartbeat {
    agent_id: AgentId,
    last_seen: DateTime<Utc>,
    current_task: Option<String>,
    workspace: Option<WorkspaceId>,
    cpu_percent: f32,
    memory_mb: u32,
}

// Agent sends heartbeat every 30s
// If no heartbeat for 5 minutes, agent is "stale"
// Doctor can clean up stale agents automatically
```

---

## 13. Non-Functional Requirements

| Risk | Status | Resolution |
|------|--------|------------|
| JJ vs Git worktree model difference | OPEN | Abstract at VCS trait level |
| 600+ agent coordination overhead | OPEN | Stateless server, local persistence |
| GitHub API rate limiting | OPEN | Aggressive caching, queuing |
| Stack rebase performance | OPEN | Parallel rebase, DAG optimization |

---

## 11. Quality Score

| Dimension | Score | Notes |
|-----------|-------|-------|
| Completeness | 95% | All major components defined |
| Consistency | 90% | State machines aligned |
| Testability | 95% | Each behavior has acceptance criteria |
| Clarity | 95% | Clear separation of concerns |
| Security | 90% | Inversion analysis complete |

**Overall: 93%** - Ready for planner

---

## 12. Acceptance Criteria

- [ ] SCP compiles with zero warnings
- [ ] All commands work: spawn, switch, sync, done, abort
- [ ] Queue processes entries in priority order
- [ ] Stack restack maintains parent-child relationships
- [ ] GitHub integration syncs PRs correctly
- [ ] TUI displays stack tree and allows navigation
- [ ] Snapshot/undo works for restack operations
- [ ] 100 concurrent agents can operate without corruption
- [ ] Recovery from JJ operation log works
- [ ] JSON output for all commands
- [ ] Single SQLite database with async tokio
- [ ] Doctor command checks and fixes integrity issues
- [ ] Agent list shows all agents and their states
- [ ] Agent status shows what any agent is working on
- [ ] Agent can page another agent with message
- [ ] Heartbeat system detects stale agents
- [ ] Work distribution shows who is working on what
