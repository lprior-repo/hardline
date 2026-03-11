# Hardline Merge Plan

## Overview
Merge isolate repository (~140k LOC) into hardline (~68k LOC) following functional Rust, Scott Wlaschin DDD, and Bitter Truth principles.

## Phase 1: Session Crate Enhancement

### 1.1 Add Missing Domain Types
- [ ] **AgentId** - Agent identifier (1-128 chars, validated)
- [ ] **WorkspaceName** - Workspace name (1-255 chars)
- [ ] **TaskId/BeadId** - Task with "bd-" prefix + hex validation
- [ ] **AbsolutePath** - Validated filesystem path
- [ ] **Title, Description** - Bead content types
- [ ] **Labels, DependsOn, BlockedBy** - Collections
- [ ] **Priority** - P0-P4 levels
- [ ] **IssueType** - Bug, Feature, Task, Epic, Chore, MergeRequest

### 1.2 Add Missing State Machines
- [ ] **WorkspaceState** - Full lifecycle (Created→Working→Ready→Merged/Conflict/Abandoned)
- [ ] **AgentState** - Idle↔Active, any→Offline, any→Error

### 1.3 Add Missing Validation
- [ ] `validate_session_name` - Trim, non-empty, max 63 chars
- [ ] `validate_agent_id` - Alphanumeric + hyphen/underscore/dot/colon
- [ ] `validate_workspace_name` - No path separators
- [ ] `validate_task_id` - Must start with "bd-"
- [ ] `validate_absolute_path` - Platform-aware absolute path
- [ ] Shell metachar filtering (`$`, `` ` ``, `;`, `|`, `&`)

### 1.4 Add Missing Aggregates
- [ ] **Workspace aggregate** - Full lifecycle management
- [ ] **Bead aggregate** - State transitions (Open→InProgress→Blocked→Closed)

## Phase 2: CLI Commands

### 2.1 Core Workflow Commands
- [ ] **spawn** - Create isolated workspace, run agent
- [ ] **done** - Complete work, merge to main
- [ ] **abort** - Abandon without merging
- [ ] **wait** - Blocking primitives (session-exists, healthy, etc.)
- [ ] **batch** - Atomic batch with checkpoint rollback

### 2.2 Session Management
- [ ] **switch** - Switch between workspaces
- [ ] **context/whereami** - Location detection

### 2.3 Task Management
- [ ] **task list** - List tasks/beads
- [ ] **task show** - Show task details
- [ ] **task claim** - Claim task (with TTL lock)
- [ ] **task yield** - Release claim
- [ ] **task start** - Start work
- [ ] **task done** - Complete task

### 2.4 Backup/Recovery
- [ ] **checkpoint** - JJ checkpoint management
- [ ] **backup** - Backup with retention
- [ ] **recover** - Recovery operations

## Phase 3: Orchestrator Enhancement

### 3.1 Missing Features
- [ ] **Parallel phase execution** - Run multiple phases in parallel
- [ ] **Phase dependencies** - Explicit dependency graph
- [ ] **Rollback/cleanup** - Cleanup on failure
- [ ] **Checkpointing** - Granular within-phase checkpointing
- [ ] **Timeout management** - Phase-level timeouts
- [ ] **Retry policies** - Exponential backoff, circuit breakers

### 3.2 Integration
- [ ] Integrate queue workflow with orchestrator
- [ ] Integrate bead dependencies with orchestration
- [ ] Coordinate twins state with validation phases

## Phase 4: Already Synced (Verify)
- [ ] **coordination** - locks.rs, conflict_resolutions.rs (verify identical)
- [ ] **output/JSONL** - output_jsonl module (verify equivalent)
- [ ] **twins** - Server (verify identical)

## Standards

All code must follow:
- **Functional Rust**: Data → Calc → Actions, zero unwrap/panic
- **Scott Wlaschin DDD**: domain/application/infrastructure
- **Railway-Oriented**: All functions return Result<T, Error>
- **Bitter Truth**: Velocity-first
- **File limits**: 300 lines/file, 40 lines/function
- **Moon CI/CD**: Remote cache enabled

## Commands to Run After Merge

```bash
# Verify compilation
moon run :ci

# Check line limits
moon run :check-lines

# Run all tests
moon run :test
```
