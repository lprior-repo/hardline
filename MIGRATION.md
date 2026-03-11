# SCP Migration Plan

**Version:** 1.0  
**Date:** 2026-03-11  
**Status:** Ready for Execution  

---

## Executive Summary

This document provides a comprehensive migration plan for the SCP (Source Control Plane) project, consolidating external projects and implementing the architecture defined in `architecture-spec.md`.

### Current State

| Component | Location | Status |
|-----------|----------|--------|
| SCP Core | `/home/lewis/src/scp/crates/` | 10 crates: core, vcs, cli, orchestrator, twins, scenarios, beads, workspace, queue |
| External: isolate | `/home/lewis/src/isolate/` | 5 crates: isolate-core, twins, scenarios, orchestrator, isolate |
| External: stak | `/home/lewis/src/stak/` | 2 crates: stak-core, stak |
| External: stax | `/home/lewis/src/stax/` | TUI, GitHub integration, snapshots |

### Target State

- Consolidated workspace with DDD-structured crates
- Single binary with all commands
- Zero warnings, zero unwrap/panic, functional Rust
- Comprehensive testing infrastructure

### Migration Principles

1. **Functional Rust**: Data → Calc → Actions, no `mut`, zero `unwrap`/`panic`
2. **Scott Wlaschin DDD**: Domain layer (pure) → Application layer → Infrastructure
3. **Railway-Oriented Programming**: All functions return `Result<T, Error>`
4. **Bitter Truth**: Output-focused, disposable code

---

## Phase 1: Stabilize SCP (Week 1)

**Objective:** Fix compilation errors, ensure zero warnings, establish DDD structure

### 1.1 Fix Broken Crates

| Crate | Issue | Action |
|-------|-------|--------|
| `scenarios` | Compilation errors | Fix `lib.rs` |
| `orchestrator` | Compilation errors | Fix `lib.rs` |
| `twins` | Compilation errors | Fix `lib.rs` |

```bash
# Verify each crate compiles
cd /home/lewis/src/scp
cargo check -p scp-scenarios
cargo check -p scp-orchestrator
cargo check -p scp-twins
```

### 1.2 Enforce Zero Warnings

Add to each crate's `lib.rs`:
```rust
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]
```

### 1.3 Apply DDD Structure

Restructure existing crates following:
```
crates/<name>/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── domain/           # Pure types, state machines, no I/O
│   │   ├── entities/
│   │   ├── value_objects/
│   │   ├── events/
│   │   └── state/
│   ├── application/     # Use cases, orchestrates domain
│   ├── infrastructure/  # DB, VCS, network I/O
│   └── api/            # HTTP/CLI endpoints
```

### 1.4 Add Testing Infrastructure Dependencies

Update `/home/lewis/src/scp/Cargo.toml`:

```toml
[workspace.dependencies]
# === EXISTING (keep) ===
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
futures = "0.3"
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
toml = "0.8"
reqwest = { version = "0.12", features = ["json"] }
tracing = "0.1"
tracing-subscriber = "0.3"
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio", "sqlite", "macros"] }
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
im = { version = "15.1", features = ["serde"] }
rpds = "1.2"
itertools = "0.13"
either = "1.13"
tap = "1.0"
strum = { version = "0.26", features = ["derive"] }
directories = "6"
fs2 = "0.4"
notify = "6"
notify-debouncer-mini = "0.4"
walkdir = "2"
regex = "1.11"
kdl = "4.7"
which = "6.0"
petgraph = "0.6"
hex = "0.4"
faster-hex = "0.10"
jj-lib = "0.38"
git2 = "0.19"

# === TESTING INFRASTRUCTURE (add) ===
tokio-test = "0.4"
proptest = "1.5"
criterion = "0.5"
pretty_assertions = "1.4"
tempfile = "3.14"
serial_test = "3.0"
doc-comment = "0.3"
cargo-deny = "0.16"
cargo-mutants = "0.8"
insta = { version = "1.40", features = ["yaml"] }

# === OPTIONAL: ADVANCED TESTING ===
# kani = "0.42"         # Model checking (requires nightly)
# dylint-linting = "0.2" # Custom lints
```

### 1.5 Add Strict Lints

Update workspace lints in `Cargo.toml`:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "allow"
dead_code = "deny"
unused_must_use = "deny"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
correctness = { level = "deny", priority = -1 }
suspicious = { level = "deny", priority = -1 }
```

### Phase 1 Verification

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

---

## Phase 2: Consolidate Crates (Week 2)

**Objective:** Migrate isolate and stak projects into SCP workspace

### 2.1 Migrate isolate-core → crates/session

| Aspect | Details |
|--------|---------|
| Source | `/home/lewis/src/isolate/crates/isolate-core/` |
| Target | `/home/lewis/src/scp/crates/session/` |

**Files to create:**

1. `crates/session/Cargo.toml`:
```toml
[package]
name = "scp-session"
version.workspace = true
edition.workspace = true

[dependencies]
scp-core = { path = "../core" }
scp-workspace = { path = "../workspace" }
scp-beads = { path = "../beads" }
tokio.workspace = true
async-trait.workspace = true
thiserror.workspace = true
chrono.workspace = true
serde.workspace = true

[dev-dependencies]
tokio-test.workspace = true
proptest.workspace = true
tempfile.workspace = true
pretty_assertions.workspace = true
```

2. `crates/session/src/lib.rs`:
```rust
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod application;
pub mod infrastructure;
pub mod error;

pub use domain::entities::session::{Session, SessionId, SessionState};
pub use application::session_service::SessionService;
pub use error::{SessionError, Result};
```

3. `crates/session/src/domain/mod.rs`:
```rust
pub mod entities;
pub mod value_objects;
pub mod events;
pub mod state;

pub use entities::session::{Session, SessionId, SessionState};
pub use events::session_event::SessionEvent;
pub use state::session_state_machine::SessionStateMachine;
```

4. `crates/session/src/domain/entities/session.rs`:
```rust
use chrono::{DateTime, Utc};
use crate::domain::value_objects::{SessionName, WorkspaceId, BeadId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Created,
    Active,
    Syncing,
    Synced,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub name: SessionName,
    pub workspace: Option<WorkspaceId>,
    pub bead: Option<BeadId>,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn create(name: SessionName) -> Result<Self, SessionError> {
        Ok(Self {
            id: SessionId::generate(),
            name,
            workspace: None,
            bead: None,
            state: SessionState::Created,
            created_at: Utc::now(),
        })
    }

    pub fn transition(&self, event: SessionEvent) -> Result<Self, SessionError> {
        let new_state = SessionStateMachine::transition(&self.state, &event)?;
        Ok(Self {
            id: self.id.clone(),
            name: self.name.clone(),
            workspace: self.workspace.clone(),
            bead: self.bead.clone(),
            state: new_state,
            created_at: self.created_at,
        })
    }
}
```

5. `crates/session/src/error.rs`:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    
    #[error("Session already active: {0}")]
    AlreadyActive(String),
    
    #[error("Session expired: {0}")]
    Expired(String),
    
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidTransition { from: String, to: String },
    
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),
    
    #[error("Bead not found: {0}")]
    BeadNotFound(String),
    
    #[error("Bead already claimed: {0}")]
    BeadAlreadyClaimed(String),
}

pub type Result<T> = std::result::Result<T, SessionError>;
```

### 2.2 Migrate stak-core → crates/queue (merge)

| Aspect | Details |
|--------|---------|
| Source | `/home/lewis/src/stak/crates/stak-core/` |
| Target | `/home/lewis/src/scp/crates/queue/` |

**Files to enhance:**

1. `crates/queue/src/domain/entities/queue_entry.rs`:
```rust
use chrono::{DateTime, Utc};
use crate::domain::value_objects::{QueueEntryId, SessionId, Priority};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueStatus {
    Pending,
    Claimed,
    Rebasing,
    Testing,
    ReadyToMerge,
    Merging,
    Merged,
    FailedRetryable,
    FailedTerminal,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: QueueEntryId,
    pub session: SessionId,
    pub priority: Priority,
    pub enqueued_at: DateTime<Utc>,
    pub status: QueueStatus,
}

impl QueueEntry {
    pub fn enqueue(session: SessionId, priority: Priority) -> Self {
        Self {
            id: QueueEntryId::generate(),
            session,
            priority,
            enqueued_at: Utc::now(),
            status: QueueStatus::Pending,
        }
    }

    pub fn transition(&self, new_status: QueueStatus) -> Result<Self, QueueError> {
        let valid = match (&self.status, &new_status) {
            (Pending, Claimed) => true,
            (Claimed, Rebasing) => true,
            (Rebasing, Testing) => true,
            (Testing, ReadyToMerge) => true,
            (ReadyToMerge, Merging) => true,
            (Merging, Merged) => true,
            (Testing, FailedRetryable) => true,
            (Testing, FailedTerminal) => true,
            (_, Cancelled) => true,
            _ => false,
        };
        
        if valid {
            Ok(Self {
                id: self.id.clone(),
                session: self.session.clone(),
                priority: self.priority,
                enqueued_at: self.enqueued_at,
                status: new_status,
            })
        } else {
            Err(QueueError::InvalidTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", new_status),
            })
        }
    }
}
```

### 2.3 Integrate CLI Commands

**Files to modify:**

- `crates/cli/src/commands/session.rs` - Add session commands
- `crates/cli/src/commands/queue.rs` - Add queue commands  
- `crates/cli/src/main.rs` - Register new commands

```rust
// crates/cli/src/commands/session.rs
use clap::{Subcommand, Args};

#[derive(Subcommand)]
pub enum SessionCommands {
    Create(CreateSessionArgs),
    List,
    Status(SessionStatusArgs),
    Complete(SessionCompleteArgs),
    Abort(SessionAbortArgs),
}

#[derive(Args)]
pub struct CreateSessionArgs {
    pub name: String,
    pub workspace: Option<String>,
}

#[derive(Args)]
pub struct SessionStatusArgs {
    pub session_id: String,
}

#[derive(Args)]
pub struct SessionCompleteArgs {
    pub session_id: String,
}

#[derive(Args)]
pub struct SessionAbortArgs {
    pub session_id: String,
    pub reason: Option<String>,
}
```

### 2.4 Update Workspace Members

Update `/home/lewis/src/scp/Cargo.toml`:

```toml
[workspace]
members = [
    "crates/cli",
    "crates/core",
    "crates/vcs",
    "crates/queue",
    "crates/session",    # NEW
    "crates/workspace",
    "crates/beads",
    "crates/orchestrator",
    "crates/twins",
    "crates/scenarios",
]
```

### Phase 2 Verification

```bash
cargo check --workspace
cargo build --workspace
```

---

## Phase 3: Add Stax Features (Week 3)

**Objective:** Create new crates for stack, TUI, and snapshot functionality

### 3.1 Create crates/stack

| Aspect | Details |
|--------|---------|
| Target | `/home/lewis/src/scp/crates/stack/` |
| Purpose | Stacked PRs management with GitHub integration |

**Files to create:**

1. `crates/stack/Cargo.toml`:
```toml
[package]
name = "scp-stack"
version.workspace = true
edition.workspace = true

[dependencies]
scp-core = { path = "../core" }
scp-vcs = { path = "../vcs" }
scp-session = { path = "../session" }
gix = "0.78"           # Pure Rust Git (replace git2)
octocrab = "0.44"
tokio.workspace = true
thiserror.workspace = true
serde.workspace = true
petgraph.workspace = true

[dev-dependencies]
insta.workspace = true
proptest.workspace = true
tempfile.workspace = true
```

2. `crates/stack/src/lib.rs`:
```rust
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod github;
pub mod engine;
pub mod error;

pub use domain::entities::{Stack, StackBranch, BranchName};
pub use error::{StackError, Result};
```

3. `crates/stack/src/domain/entities/stack.rs`:
```rust
use serde::{Deserialize, Serialize};
use crate::domain::value_objects::{BranchName, CommitHash};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u32,
    pub url: String,
    pub title: String,
    pub state: PrState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackBranch {
    pub name: BranchName,
    pub parent: Option<BranchName>,
    pub pr_info: Option<PrInfo>,
    pub revision: CommitHash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub branches: Vec<StackBranch>,
    pub main_branch: BranchName,
}

impl Stack {
    pub fn new(main_branch: BranchName) -> Self {
        Self {
            branches: Vec::new(),
            main_branch,
        }
    }

    pub fn add_branch(&mut self, branch: StackBranch) -> Result<(), StackError> {
        if let Some(parent) = &branch.parent {
            if !self.branches.iter().any(|b| &b.name == parent) 
                && parent != &self.main_branch {
                return Err(StackError::OrphanedBranch(branch.name.to_string()));
            }
        }
        self.branches.push(branch);
        Ok(())
    }

    pub fn topological_order(&self) -> Vec<&StackBranch> {
        let mut graph: petgraph::Graph<&StackBranch, ()> = petgraph::Graph::new();
        let mut indices: std::collections::HashMap<&BranchName, _> = std::collections::HashMap::new();
        
        for branch in &self.branches {
            let idx = graph.add_node(branch);
            indices.insert(&branch.name, idx);
        }
        
        for branch in &self.branches {
            if let Some(parent) = &branch.parent {
                if let Some(&child_idx) = indices.get(&branch.name) {
                    if let Some(&parent_idx) = indices.get(parent) {
                        graph.add_edge(parent_idx, child_idx, ());
                    }
                }
            }
        }
        
        petgraph::algo::toposort(&graph, None)
            .unwrap_or_default()
            .iter()
            .map(|&idx| graph.node_weight(idx).unwrap())
            .collect()
    }
}
```

### 3.2 Create crates/tui

| Aspect | Details |
|--------|---------|
| Target | `/home/lewis/src/scp/crates/tui/` |
| Purpose | Terminal UI for stack visualization |

**Files to create:**

1. `crates/tui/Cargo.toml`:
```toml
[package]
name = "scp-tui"
version.workspace = true
edition.workspace = true

[dependencies]
scp-core = { path = "../core" }
scp-stack = { path = "../stack" }
scp-snapshot = { path = "../snapshot" }
ratatui = "0.28"
crossterm = "0.28"
tokio.workspace = true
anyhow.workspace = true

[dev-dependencies]
insta.workspace = true
```

2. `crates/tui/src/lib.rs`:
```rust
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod views;
pub mod input;
pub mod app;
pub mod error;

pub use app::TuiApp;
pub use error::{TuiError, Result};
```

### 3.3 Create crates/snapshot

| Aspect | Details |
|--------|---------|
| Target | `/home/lewis/src/scp/crates/snapshot/` |
| Purpose | Undo/redo functionality via Git refs |

**Files to create:**

1. `crates/snapshot/Cargo.toml`:
```toml
[package]
name = "scp-snapshot"
version.workspace = true
edition.workspace = true

[dependencies]
scp-core = { path = "../core" }
scp-vcs = { path = "../vcs" }
tokio.workspace = true
thiserror.workspace = true
chrono.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

2. `crates/snapshot/src/lib.rs`:
```rust
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod storage;
pub mod error;

pub use domain::snapshot::{Snapshot, SnapshotId};
pub use error::{SnapshotError, Result};
```

3. `crates/snapshot/src/domain/snapshot.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub branch_name: String,
    pub commit_hash: String,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
}

impl Snapshot {
    pub fn create(branch_name: String, commit_hash: String, description: Option<String>) -> Self {
        Self {
            id: SnapshotId::generate(),
            branch_name,
            commit_hash,
            created_at: Utc::now(),
            description,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SnapshotId(String);

impl SnapshotId {
    pub fn generate() -> Self {
        Self(format!("snap-{}", uuid::Uuid::new_v4()))
    }
}
```

### 3.4 Migrate git2 → gix

**Files to modify:**

- `Cargo.toml` - Remove `git2`, add `gix`
- `crates/vcs/src/vcs/git.rs` - Rewrite using gix

```toml
# Remove
git2 = "0.19"

# Add
gix = "0.78"
```

### Phase 3 Verification

```bash
cargo check --workspace
cargo build --workspace
```

---

## Phase 4: Integration (Week 4)

**Objective:** Unify error handling, enforce architectural constraints

### 4.1 Create Unified Error Types

**File:** `crates/core/src/error/unified.rs`

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScpError {
    // 1xxx - Workspace
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),
    #[error("Workspace locked: {0}")]
    WorkspaceLocked(String),
    #[error("Workspace corrupt: {0}")]
    WorkspaceCorrupt(String),

    // 2xxx - Session
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Session already active: {0}")]
    SessionAlreadyActive(String),
    #[error("Session expired: {0}")]
    SessionExpired(String),

    // 3xxx - Bead
    #[error("Bead not found: {0}")]
    BeadNotFound(String),
    #[error("Bead already claimed: {0}")]
    BeadAlreadyClaimed(String),
    #[error("Bead dependency cycle detected")]
    BeadDependencyCycle,

    // 4xxx - Queue
    #[error("Queue entry not found: {0}")]
    QueueNotFound(String),
    #[error("Queue priority conflict")]
    QueuePriorityConflict,
    #[error("Queue stale entry: {0}")]
    QueueStaleEntry(String),

    // 5xxx - VCS
    #[error("VCS not found: {0}")]
    VcsNotFound(String),
    #[error("VCS conflict: {0}")]
    VcsConflict(String),
    #[error("VCS detached head")]
    VcsDetachedHead,

    // 6xxx - Stack
    #[error("Stack not found: {0}")]
    StackNotFound(String),
    #[error("Stack orphaned branch: {0}")]
    StackOrphaned(String),
    #[error("Stack cyclic dependency")]
    StackCyclicDependency,

    // 7xxx - GitHub
    #[error("GitHub auth failed")]
    GitHubAuthFailed,
    #[error("GitHub PR closed: {0}")]
    GitHubPrClosed(String),
    #[error("GitHub rate limited")]
    GitHubRateLimited,

    // 8xxx - Snapshot
    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),
    #[error("Snapshot corrupt: {0}")]
    SnapshotCorrupt(String),

    // 9xxx - Internal
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Database corrupt: {0}")]
    DatabaseCorrupt(String),
    #[error("Unexpected null value")]
    UnexpectedNull,
}

impl ScpError {
    pub fn code(&self) -> u16 {
        match self {
            Self::WorkspaceNotFound(_) => 1001,
            Self::WorkspaceLocked(_) => 1002,
            Self::WorkspaceCorrupt(_) => 1003,
            Self::SessionNotFound(_) => 2001,
            Self::SessionAlreadyActive(_) => 2002,
            Self::SessionExpired(_) => 2003,
            Self::BeadNotFound(_) => 3001,
            Self::BeadAlreadyClaimed(_) => 3002,
            Self::BeadDependencyCycle => 3003,
            Self::QueueNotFound(_) => 4001,
            Self::QueuePriorityConflict => 4002,
            Self::QueueStaleEntry(_) => 4003,
            Self::VcsNotFound(_) => 5001,
            Self::VcsConflict(_) => 5002,
            Self::VcsDetachedHead => 5003,
            Self::StackNotFound(_) => 6001,
            Self::StackOrphaned(_) => 6002,
            Self::StackCyclicDependency => 6003,
            Self::GitHubAuthFailed => 7001,
            Self::GitHubPrClosed(_) => 7002,
            Self::GitHubRateLimited => 7003,
            Self::SnapshotNotFound(_) => 8001,
            Self::SnapshotCorrupt(_) => 8002,
            Self::InternalError(_) => 9001,
            Self::DatabaseCorrupt(_) => 9002,
            Self::UnexpectedNull => 9003,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::GitHubRateLimited | Self::VcsConflict(_) | Self::QueueStaleEntry(_)
        )
    }
}

pub type Result<T> = std::result::Result<T, ScpError>;
```

### 4.2 Ensure Single Binary

**File:** `crates/cli/Cargo.toml`

```toml
[[bin]]
name = "scp"
path = "src/main.rs"

[dependencies]
scp-core = { path = "../core" }
scp-session = { path = "../session" }
scp-queue = { path = "../queue" }
scp-stack = { path = "../stack" }
scp-tui = { path = "../tui" }
scp-snapshot = { path = "../snapshot" }
scp-vcs = { path = "../vcs" }
scp-workspace = { path = "../workspace" }
scp-beads = { path = "../beads" }
```

### 4.3 Create Line Limit Enforcement Script

**File:** `scripts/check-line-limits.sh`

```bash
#!/bin/bash
set -e

echo "Checking for files over 300 lines..."
FILES=$(find crates -name "*.rs" -exec wc -l {} \; | awk '$1 > 300 { print $2 }')
if [ -n "$FILES" ]; then
    echo "ERROR: The following files exceed 300 lines:"
    echo "$FILES"
    exit 1
fi

echo "Checking for functions over 40 lines..."
# Uses cargo machete or similar tool
cargo machete --strict crates/

echo "✓ Line limits check passed"
```

### 4.4 Create deny.toml Configuration

**File:** `deny.toml`

```toml
[licenses]
unlicensed = "deny"
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "ISC"]

[advisories]
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"

[bans]
wildcards = "deny"
highlight = "all"
workspace-default-features = "deny"
external-default-features = "allow"
allow = [
    { id = "RUSTSEC-0000", reason = "false positive" },
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### 4.5 Create cargo-mutants Configuration

**File:** `.cargo/mutants.toml`

```toml
examine_globs = ["crates/**/domain/**/*.rs"]
exclude_re = ["impl Debug", "impl Display", "impl Clone", "impl Default"]
fail_on_automatic = true
fail_on_unmodified = true
```

### 4.6 Create dylint Custom Lints

**File:** `lints/no_unwrap_in_domain/src/lib.rs`

```rust
#![feature(rustc_private)]
extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_lint;

use rustc_hir::{Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};

declare_lint! {
    pub NO_UNWRAP_IN_DOMAIN,
    Deny,
    "`.unwrap()` and `.expect()` are forbidden in domain layer"
}

declare_lint_pass!(NoUnwrapInDomain => [NO_UNWRAP_IN_DOMAIN]);

impl<'tcx> LateLintPass<'tcx> for NoUnwrapInDomain {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::MethodCall(_, receiver, .. ) = expr.kind {
            let method_name = cx.tcx.item_name(receiver.hir_id);
            if method_name.as_str() == "unwrap" || method_name.as_str() == "expect" {
                cx.span_lint(NO_UNWRAP_IN_DOMAIN, expr.span, "unwrap/expect forbidden in domain");
            }
        }
    }
}
```

### Phase 4 Verification

```bash
./scripts/check-line-limits.sh
cargo deny check
cargo clippy --workspace -- -D warnings
```

---

## Phase 5: Polish (Week 5)

**Objective:** Comprehensive testing, documentation, release

### 5.1 Testing Trophy Implementation

#### Unit Tests

Every domain function must have tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn session_when_created_then_has_created_state() {
        let session = Session::create(SessionName::new("test".into())).unwrap();
        assert_eq!(session.state, SessionState::Created);
    }

    #[test]
    fn session_given_created_when_activate_then_has_active_state() {
        let session = Session::create(SessionName::new("test".into())).unwrap();
        let activated = session.transition(SessionEvent::Activated).unwrap();
        assert_eq!(activated.state, SessionState::Active);
    }
}
```

#### Property-Based Tests (proptest)

```rust
proptest! {
    #[test]
    fn session_id_always_valid(s in "[a-z0-9-]{8,20}") {
        let id = SessionId::parse(s);
        prop_assert!(id.is_ok());
    }

    #[test]
    fn queue_entry_priority_always_valid(p in 0u8..=255) {
        let entry = QueueEntry::enqueue(SessionId::generate(), Priority::new(p));
        prop_assert_eq!(entry.priority.value(), p);
    }
}
```

#### Snapshot Tests (insta)

```rust
#[test]
fn test_session_json_output() {
    let session = Session::create(SessionName::new("test".into())).unwrap();
    let json = serde_json::to_string_pretty(&session).unwrap();
    insta::assert_snapshot!(json);
}
```

#### Benchmark Tests (criterion)

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn bench_stack_topological_order(c: &mut Criterion) {
    let stack = create_test_stack(100);
    
    c.bench_function("topological_sort_100_branches", |b| {
        b.iter(|| stack.topological_order())
    });
}

criterion_group!(benches, bench_stack_topological_order);
criterion_main!(benches);
```

#### Mutation Testing (cargo-mutants)

```bash
# Run mutation testing on domain layer
cargo mutants --in-diff HEAD~1..HEAD -- crates/*/domain/
```

#### Model Checking (kani) - Optional

```rust
#[kani::proof]
fn session_state_machine_valid() {
    let session = Session::create(SessionName::new("test".into()));
    // Verify all state transitions are valid
}
```

### 5.2 Create moon Tasks Configuration

**File:** `.moon/tasks.yml`

```yaml
$schema: "https://moonrepo.dev/schemas/tasks.json"

tasks:
  # Stage 1: Formatting & Linting
  fmt:
    command: "cargo fmt --all --check"
    inputs: ["crates/**/*.rs", "Cargo.toml"]
    options:
      cache: true

  clippy:
    command: "cargo clippy --workspace --all-targets -- -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic"
    inputs: ["crates/**/*.rs", "Cargo.toml"]
    options:
      cache: true

  deny:
    command: "cargo deny check"
    options:
      cache: true

  # Stage 2: Testing
  test:
    command: "cargo nextest run --workspace"
    inputs: ["crates/**/*.rs"]
    options:
      cache: true
      runInCI: true

  test-doc:
    command: "cargo test --doc"
    options:
      cache: true

  # Stage 3: Build
  check:
    command: "cargo check --workspace --all-features"
    options:
      cache: true

  build:
    command: "cargo build --release --workspace"
    outputs: ["/target/release/scp"]
    options:
      cache: true
      runInCI: true

  # Stage 4: Quality
  audit:
    command: "cargo audit"
    options:
      cache: false

  mutants:
    command: "cargo mutants"
    options:
      cache: false

  # Stage 5: Architecture
  check-lines:
    command: "./scripts/check-line-limits.sh"
    options:
      cache: false

  # Composite
  ci:
    command: "cargo fmt --all --check && cargo clippy --workspace -- -D warnings && cargo nextest run --workspace"
    options:
      cache: false
      runInCI: true
```

### 5.3 Generate Documentation

```bash
cargo doc --no-deps --document-private-items
```

### 5.4 Final Verification Commands

```bash
# Full CI pipeline
moon run :ci

# Quick verification
moon run :fmt
moon run :clippy
moon run :test

# Quality gates
cargo deny check
cargo mutants
```

---

## Complete File Manifest

### New Crates to Create

| Crate | Path | Phase | Key Dependencies |
|-------|------|-------|------------------|
| session | `crates/session/` | 2 | core, workspace, beads |
| stack | `crates/stack/` | 3 | core, vcs, session, gix, octocrab |
| tui | `crates/tui/` | 3 | core, stack, snapshot, ratatui |
| snapshot | `crates/snapshot/` | 3 | core, vcs |

### Files to Modify

| File | Phase | Change |
|------|-------|--------|
| `Cargo.toml` | 1-4 | Add testing deps, new crates, gix |
| `crates/cli/Cargo.toml` | 2-4 | Add dependencies, single binary |
| `crates/cli/src/main.rs` | 2-4 | Register all commands |
| `crates/core/src/error/unified.rs` | 4 | Unified error types |
| `crates/vcs/src/vcs/git.rs` | 3 | Migrate git2 → gix |
| `crates/queue/src/` | 2 | Merge stak-core logic |
| `scripts/check-line-limits.sh` | 4 | Line limit enforcement |
| `deny.toml` | 4 | License/banned deps |
| `.cargo/mutants.toml` | 4 | Mutation config |
| `.moon/tasks.yml` | 5 | CI/CD tasks |

### Dependencies Summary

```toml
# Testing (workspace.dependencies)
proptest = "1.5"
criterion = "0.5"
pretty_assertions = "1.4"
tempfile = "3.14"
cargo-deny = "0.16"
cargo-mutants = "0.8"
insta = "1.40"
# kani = "0.42"       # Optional
# dylint-linting = "0.2"  # Optional

# New libraries
gix = "0.78"          # Replace git2
octocrab = "0.44"     # GitHub API
ratatui = "0.28"      # TUI
crossterm = "0.28"    # Terminal
```

---

## Migration Timeline

| Week | Phase | Deliverables |
|------|-------|---------------|
| 1 | Stabilize SCP | Fixed crates, zero warnings, DDD structure, testing deps |
| 2 | Consolidate Crates | session crate, merged queue, CLI commands integrated |
| 3 | Add Stax Features | stack, tui, snapshot crates, gix migration |
| 4 | Integration | Unified errors, single binary, line limits, deny config |
| 5 | Polish | Comprehensive tests, docs, mutation testing, release v1.0 |

---

## Verification Checklist

- [ ] All crates compile with `cargo check --workspace`
- [ ] Zero warnings with `cargo clippy --workspace -- -D warnings`
- [ ] All tests pass with `cargo nextest run --workspace`
- [ ] License audit passes with `cargo deny check`
- [ ] Mutation tests validate test quality with `cargo mutants`
- [ ] File line limits enforced (< 300 lines)
- [ ] Function line limits enforced (< 40 lines)
- [ ] All functions return `Result<T, Error>`
- [ ] Zero `unwrap()`/`panic()` in domain layer
- [ ] Documentation builds with `cargo doc`

---

## References

- Architecture Specification: `architecture-spec.md`
- Session Domain: `/home/lewis/src/isolate/crates/isolate-core/`
- Queue Domain: `/home/lewis/src/stak/crates/stak-core/`
- Stack/TUI/Snapshot: `/home/lewis/src/stax/src/`
- Reference: triagebot, effectum, git-stack (see architecture-spec.md)
