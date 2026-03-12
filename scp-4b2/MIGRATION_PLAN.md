# SCP Migration Plan

## Executive Summary

This document provides a detailed step-by-step migration plan for the SCP (Source Control Plane) project following the architecture-spec.md. The migration is divided into 5 phases over 5 weeks.

**Current State:**
- SCP workspace has 10 crates: core, vcs, cli, orchestrator, twins, scenarios, beads, workspace, queue
- External projects to migrate: `/home/lewis/src/isolate` (isolate-core, twins, orchestrator, scenarios), `/home/lewis/src/stak` (stak-core, stak), `/home/lewis/src/stax` (TUI, GitHub integration, snapshots)

**Target State:**
- Consolidated workspace with DDD-structured crates: session, cli, queue, stack, tui, snapshot
- Single binary with all commands
- Zero warnings, zero unwrap/panic, functional Rust

---

## Phase 1: Stabilize SCP (Week 1)

### 1.1 Fix Broken Crates

**Files to modify:**
- `crates/scenarios/src/lib.rs` - Fix compilation errors
- `crates/orchestrator/src/lib.rs` - Fix compilation errors  
- `crates/twins/src/lib.rs` - Fix compilation errors

**Actions:**
```bash
# Test each crate individually
cd crates/scenarios && cargo check
cd crates/orchestrator && cargo check
cd crates/twins && cargo check
```

### 1.2 Ensure Zero Warnings

**Files to modify:**
- Add `#[deny(warnings)]` to all `lib.rs` files
- Fix any clippy warnings

**Action:** Add to each crate's `lib.rs`:
```rust
#![deny(warnings)]
```

### 1.3 Apply DDD Structure to Existing Code

**Existing crates to restructure:**
- `crates/core/` - Already has domain/, add application/ and infrastructure/
- `crates/vcs/` - Add DDD layers
- `crates/queue/` - Add DDD layers
- `crates/workspace/` - Add DDD layers
- `crates/beads/` - Add DDD layers

**DDD Layer Structure:**
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

### 1.4 Phase 1 Cargo.toml Changes

**File: `/home/lewis/src/scp/Cargo.toml`**

Add testing infrastructure dependencies:
```toml
# Testing Infrastructure (add to workspace.dependencies)
pretty_assertions = "1.4"
cargo-deny = "0.14"
cargo-mutants = "0.6"
insta = { version = "1.40", features = ["yaml"] }
# kani - model checking (requires nightly, add separately if needed)
```

---

## Phase 2: Consolidate Crates (Week 2)

### 2.1 Move isolate-core → crates/session

**Source:** `/home/lewis/src/isolate/crates/isolate-core/`
**Target:** `crates/session/`

**Files to create/modify:**
1. Create `crates/session/Cargo.toml`:
```toml
[package]
name = "scp-session"
version.workspace = true
edition.workspace = true

[dependencies]
scp-core = { path = "../core" }
# Add isolate-core dependencies

[dev-dependencies]
tempfile.workspace = true
```

2. Create `crates/session/src/lib.rs` - Main entry point
3. Create `crates/session/src/domain/` - Session aggregate, state machine
4. Create `crates/session/src/application/` - Use cases
5. Create `crates/session/src/infrastructure/` - SQLite persistence
6. Move relevant code from `crates/core/src/domain/session*.rs`

**Workspace update: `/home/lewis/src/scp/Cargo.toml`**
```toml
members = [
    "crates/*", 
    "crates/session",  # ADD
    "crates/orchestrator"
]
```

### 2.2 Move isolate → crates/cli (integrate commands)

**Source:** `/home/lewis/src/isolate/crates/isolate/`
**Target:** Integrate into `crates/cli/`

**Files to modify:**
- `crates/cli/src/commands/session.rs` - Add isolate commands
- `crates/cli/src/main.rs` - Register new commands

### 2.3 Move stak-core → crates/queue

**Source:** `/home/lewis/src/stak/crates/stak-core/`
**Target:** `crates/queue/` (merge with existing queue crate)

**Files to modify:**
1. `crates/queue/src/domain/` - Add QueueEntry aggregate
2. `crates/queue/src/application/` - Add queue use cases
3. `crates/queue/src/infrastructure/` - Add SQLite persistence
4. Copy stak-core queue logic

### 2.4 Move stak → crates/cli (integrate commands)

**Source:** `/home/lewis/src/stak/crates/stak/`
**Target:** Integrate into `crates/cli/`

**Files to modify:**
- `crates/cli/src/commands/queue.rs` - Add queue commands
- `crates/cli/src/main.rs` - Register queue commands

### 2.5 Phase 2 Complete Workspace Structure

```
crates/
├── cli/           # Main CLI (integrated from isolate, stak)
├── core/          # Core domain
├── vcs/           # VCS abstraction
├── queue/         # Merge queue (merged from stak-core)
├── session/       # Session management (from isolate-core) [NEW]
├── workspace/     # Workspace management
├── beads/         # Bead tracking
├── orchestrator/  # Multi-step workflows
├── twins/         # Agent coordination
└── scenarios/     # Test scenarios
```

---

## Phase 3: Add Stax Features (Week 3)

### 3.1 Create crates/stack with GitHub Integration

**New crate:** `crates/stack/`

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
octocrab = "0.40"
gix = "0.78"  # Replace git2

[dev-dependencies]
insta.workspace = true
proptest.workspace = true
```

2. `crates/stack/src/lib.rs`:
```rust
pub mod domain;
pub mod github;
pub mod engine;
```

3. `crates/stack/src/domain/mod.rs` - StackBranch, Stack aggregate
4. `crates/stack/src/domain/stack.rs` - Stack operations
5. `crates/stack/src/github/mod.rs` - GitHub API client
6. `crates/stack/src/github/pr.rs` - PR operations
7. `crates/stack/src/engine/mod.rs` - Stack rebase/restack logic

### 3.2 Migrate git2 → gix

**Files to modify:**
- `Cargo.toml` workspace dependencies:
```toml
# REMOVE:
# git2 = "0.19"

# ADD:
gix = "0.78"
```

- Update all crates using git2:
  - `crates/vcs/src/vcs/git.rs` - Rewrite using gix
  - `crates/stack/src/engine/` - Use gix

### 3.3 Create crates/tui

**New crate:** `crates/tui/`

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
ratatui = "0.28"
crossterm = "0.28"

[dev-dependencies]
insta.workspace = true
```

2. `crates/tui/src/lib.rs`:
```rust
pub mod views;
pub mod input;
pub mod app;
```

3. `crates/tui/src/views/mod.rs` - Stack tree, diff, details
4. `crates/tui/src/input/mod.rs` - Key bindings
5. `crates/tui/src/app/mod.rs` - Main app state

### 3.4 Create crates/snapshot

**New crate:** `crates/snapshot/`

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

[dev-dependencies]
tempfile.workspace = true
```

2. `crates/snapshot/src/lib.rs`:
```rust
pub mod domain;
pub mod storage;
```

3. `crates/snapshot/src/domain/mod.rs` - Snapshot state machine
4. `crates/snapshot/src/storage/mod.rs` - Git ref-based backup

### 3.5 Phase 3 Complete Workspace Structure

```
crates/
├── cli/           # Main CLI
├── core/          # Core domain
├── vcs/           # VCS abstraction (gix migrated)
├── queue/         # Merge queue
├── session/       # Session management
├── workspace/     # Workspace management
├── beads/         # Bead tracking
├── stack/         # Stacked PRs [NEW]
├── tui/           # Terminal UI [NEW]
├── snapshot/      # Undo/redo [NEW]
├── orchestrator/  # Multi-step workflows
├── twins/         # Agent coordination
└── scenarios/     # Test scenarios
```

---

## Phase 4: Integration (Week 4)

### 4.1 Unify Error Handling

**Create unified error types:**

**File: `crates/core/src/error/unified.rs`**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScpError {
    // 1xxx - Workspace
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),
    #[error("Workspace locked: {0}")]
    WorkspaceLocked(String),
    
    // 2xxx - Session
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Session already active: {0}")]
    SessionAlreadyActive(String),
    
    // 3xxx - Bead
    #[error("Bead not found: {0}")]
    BeadNotFound(String),
    #[error("Bead already claimed: {0}")]
    BeadAlreadyClaimed(String),
    
    // 4xxx - Queue
    #[error("Queue entry not found: {0}")]
    QueueNotFound(String),
    #[error("Queue priority conflict")]
    QueuePriorityConflict,
    
    // 5xxx - VCS
    #[error("VCS error: {0}")]
    VcsError(String),
    #[error("VCS conflict: {0}")]
    VcsConflict(String),
    
    // 6xxx - Stack
    #[error("Stack not found: {0}")]
    StackNotFound(String),
    #[error("Stack orphaned: {0}")]
    StackOrphaned(String),
    
    // 7xxx - GitHub
    #[error("GitHub auth failed")]
    GitHubAuthFailed,
    #[error("GitHub rate limited")]
    GitHubRateLimited,
    
    // 8xxx - Snapshot
    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),
    
    // 9xxx - Internal
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl ScpError {
    pub fn code(&self) -> u16 {
        match self {
            Self::WorkspaceNotFound(_) => 1001,
            Self::WorkspaceLocked(_) => 1002,
            Self::SessionNotFound(_) => 2001,
            Self::SessionAlreadyActive(_) => 2002,
            Self::BeadNotFound(_) => 3001,
            Self::BeadAlreadyClaimed(_) => 3002,
            Self::QueueNotFound(_) => 4001,
            Self::QueuePriorityConflict => 4002,
            Self::VcsError(_) => 5001,
            Self::VcsConflict(_) => 5002,
            Self::StackNotFound(_) => 6001,
            Self::StackOrphaned(_) => 6002,
            Self::GitHubAuthFailed => 7001,
            Self::GitHubRateLimited => 7002,
            Self::SnapshotNotFound(_) => 8001,
            Self::InternalError(_) => 9001,
        }
    }
    
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::GitHubRateLimited | Self::VcsError(_))
    }
}
```

**Update all crates to use unified errors:**
```bash
# Add to each crate's lib.rs
pub use scp_core::error::{ScpError, Result};
```

### 4.2 Ensure Single Binary

**File: `crates/cli/Cargo.toml`**
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

### 4.3 Enforce File/Function Line Limits

**File: `scripts/check-line-limits.sh`**
```bash
#!/bin/bash
set -e

echo "Checking for files over 300 lines..."
find crates -name "*.rs" -exec wc -l {} \; | \
    awk '$1 > 300 { print $2 }' | \
    while read file; do
        echo "ERROR: $file has $(wc -l < $file) lines (max 300)"
    done

echo "Checking for functions over 40 lines..."
# Use a more sophisticated tool or grep pattern
```

**Make executable:**
```bash
chmod +x scripts/check-line-limits.sh
```

### 4.4 Phase 4 Cargo.toml - Final Testing Infrastructure

**File: `/home/lewis/src/scp/Cargo.toml`** - Final version:
```toml
[workspace]
members = [
    "crates/cli",
    "crates/core", 
    "crates/vcs",
    "crates/queue",
    "crates/session",
    "crates/workspace",
    "crates/beads",
    "crates/stack",
    "crates/tui",
    "crates/snapshot",
    "crates/orchestrator",
    "crates/twins",
    "crates/scenarios",
]
resolver = "2"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
futures = "0.3"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
toml = "0.8"

# HTTP client
reqwest = { version = "0.12", features = ["json"] }

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Database
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio", "sqlite", "macros"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# CLI
clap = { version = "4", features = ["derive"] }

# Collections & functional
im = { version = "15.1", features = ["serde"] }
rpds = "1.2"
itertools = "0.13"
either = "1.13"
tap = "1.0"

# String utilities
strum = { version = "0.26", features = ["derive"] }

# File system
directories = "6"
notify = "6"
notify-debouncer-mini = "0.4"
walkdir = "2"

# Parsing
regex = "1.11"
kdl = "4.7"

# Process
which = "6.0"

# Graph algorithms
petgraph = "0.6"

# Hex encoding/decoding
hex = "0.4"
faster-hex = "0.10"

# JJ (Jujutsu)
jj-lib = "0.38"

# Git (gix - pure Rust, replace git2)
gix = "0.78"

# GitHub
octocrab = "0.40"

# TUI
ratatui = "0.28"
crossterm = "0.28"

# Dev dependencies
tokio-test = "0.4"
proptest = "1.0"
tempfile = "3.0"
serial_test = "3.0"
criterion = "0.5"
doc-comment = "0.3"
pretty_assertions = "1.4"
cargo-deny = "0.14"
cargo-mutants = "0.6"
insta = { version = "1.40", features = ["yaml"] }

[workspace.package]
version = "0.5.0"
edition = "2021"
rust-version = "1.80"
authors = ["Source Control Plane Contributors"]
license = "MIT"
repository = "https://github.com/source-control-plane/scp"

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

# Specific allows
missing_errors_doc = "allow"
missing_panics_doc = "allow"
module_name_repetitions = "allow"
must_use_candidate = "allow"
redundant_pub_crate = "allow"
type_complexity = "allow"
return_self_not_must_use = "allow"
trivially_copy_pass_by_ref = "allow"
map_unwrap_or = "allow"

[profile.dev]
opt-level = 0
debug = true

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.bench]
inherits = "release"
debug = true
strip = false
```

---

## Phase 5: Polish (Week 5)

### 5.1 Comprehensive Tests

**Testing Trophy Coverage:**

1. **Unit Tests** - Every domain function
2. **Integration Tests** - Cross-crate interactions
3. **Property-Based Tests** - Using proptest
4. **Snapshot Tests** - Using insta for API contracts
5. **Mutation Testing** - Using cargo-mutants

**Test naming convention (BDD):**
```rust
#[test]
fn session_when_created_then_has_created_state() { }
#[test]
fn session_given_active_when_claim_bead_then_transitions_to_working() { }
```

**Property-based tests:**
```rust
proptest! {
    #[test]
    fn session_id_always_valid(s in "[a-z0-9-]{8,20}") {
        let id = SessionId::parse(s);
        prop_assert!(id.is_ok());
    }
}
```

### 5.2 Documentation

**Files to generate:**
- `docs/api.md` - API documentation
- `docs/commands.md` - CLI commands
- `docs/architecture.md` - Architecture overview
- `docs/contributing.md` - Contribution guide

**Run:**
```bash
cargo doc --no-deps --document-private-items
```

### 5.3 Cargo-deny Configuration

**File: `deny.toml`**
```toml
[licenses]
unlicensed = "deny"
allow = ["MIT", "Apache-2.0", "BSD-3-Clause"]

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

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### 5.4 Cargo-mutants Configuration

**File: `.cargo/mutants.toml`**
```toml
examine_globs = ["crates/**/domain/**/*.rs"]
exclude_re = ["impl Debug", "impl Display", "impl Clone"]
failures-are-ok = true
```

---

## Summary: Complete File Changes

### New Crates to Create (Phase 3)
| Crate | Path | Key Files |
|-------|------|-----------|
| session | `crates/session/` | `Cargo.toml`, `src/lib.rs`, `src/domain/mod.rs`, `src/application/mod.rs`, `src/infrastructure/mod.rs` |
| stack | `crates/stack/` | `Cargo.toml`, `src/lib.rs`, `src/domain/mod.rs`, `src/github/mod.rs`, `src/engine/mod.rs` |
| tui | `crates/tui/` | `Cargo.toml`, `src/lib.rs`, `src/views/mod.rs`, `src/input/mod.rs`, `src/app/mod.rs` |
| snapshot | `crates/snapshot/` | `Cargo.toml`, `src/lib.rs`, `src/domain/mod.rs`, `src/storage/mod.rs` |

### Files to Modify (All Phases)
| File | Phase | Change |
|------|-------|--------|
| `Cargo.toml` | 1,2,3,4 | Add testing deps, new crates, gix |
| `crates/cli/Cargo.toml` | 2,4 | Add dependencies, single binary |
| `crates/cli/src/main.rs` | 2,4 | Register all commands |
| `crates/core/src/error/unified.rs` | 4 | Unified error types |
| `crates/vcs/src/vcs/git.rs` | 3 | Migrate git2 → gix |
| `crates/queue/src/` | 2 | Merge stak-core logic |
| `scripts/check-line-limits.sh` | 4 | Line limit enforcement |

### Dependencies to Add (Cargo.toml)
```toml
# Testing
proptest = "1.0"
criterion = "0.5"
pretty_assertions = "1.4"
tempfile = "3.0"
cargo-deny = "0.14"
cargo-mutants = "0.6"
insta = "1.40"

# New libraries
gix = "0.78"           # Replace git2
octocrab = "0.40"      # GitHub API
ratatui = "0.28"       # TUI
crossterm = "0.28"     # Terminal
```

### Commands to Run Verification
```bash
# Phase 1
cargo check --workspace
cargo clippy --workspace -- -D warnings

# Phase 2
cargo check --workspace

# Phase 3
cargo check --workspace

# Phase 4
./scripts/check-line-limits.sh
cargo deny check

# Phase 5
cargo nextest run --workspace
cargo doc --no-deps --document-private-items
```

---

## Migration Timeline

| Week | Phase | Deliverables |
|------|-------|--------------|
| 1 | Stabilize SCP | Fixed crates, zero warnings, DDD structure |
| 2 | Consolidate Crates | session crate, merged queue, CLI commands |
| 3 | Add Stax Features | stack, tui, snapshot crates, gix migration |
| 4 | Integration | Unified errors, single binary, line limits |
| 5 | Polish | Tests, docs, deny config, release v1.0 |
