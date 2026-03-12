# Black Hat Defects Report: scp-4t5

**Bead**: scp-4t5
**Title**: cli: Add task management commands
**Review Date**: 2026-03-11
**Verdict**: REJECTED

---

## Critical Defects (Must Fix)

### DEFECT-001: No Test Implementation
**Phase**: 2 (Farley Engineering Rigor)
**Severity**: CRITICAL

**Location**: Entire codebase - no test file for task commands

**Issue**: 
- Martin-Fowler test plan exists at `.beads/scp-4t5/martin-fowler-tests.md`
- No corresponding test implementation found in `crates/cli/src/commands/` or anywhere in codebase
- Tests are a hard requirement per Farley TDD methodology

**Required Fix**: Implement tests in `crates/cli/src/commands/task.rs` with `#[cfg(test)]` module or separate test file

---

### DEFECT-002: Hardcoded User Context
**Phase**: 5 (The Bitter Truth)
**Severity**: CRITICAL

**Locations**:
- Line 189: `"current-user"` in `task_claim()`
- Line 194: `"current-user"` in `task_claim()`
- Line 205: `"current-user"` in `task_yield()`
- Line 208: `"current-user"` in `task_yield()`
- Line 221: `"current-user"` in `task_start()`
- Line 224: `"current-user"` in `task_start()`
- Line 237: `"current-user"` in `task_done()`
- Line 240: `"current-user"` in `task_done()`
- Line 241: `"current-user"` in `task_done()`

**Issue**: 
- Contract specifies `holder: &str` parameter representing current user identity
- All operations use hardcoded `"current-user"` string
- Makes it impossible to test preconditions P3 (task claimed by other user) and P4 (task claimed by current user) properly
- Violates the "parse at boundary" principle - user should come from CLI context

**Required Fix**: Accept user as parameter from CLI context (e.g., from environment or command-line flag)

---

### DEFECT-003: TaskId Validation at Wrong Layer
**Phase**: 3 (NASA-Level Functional Rust)
**Severity**: HIGH

**Location**: 
- `crates/cli/src/commands/task_types.rs` line 23: `TaskId::new()`
- `crates/cli/src/commands/task_validation.rs` line 17: `validate_task_id()`

**Issue**:
- Contract specifies: `BeadId::new() -> Result<BeadId>` (compile-time or runtime validation at construction)
- Implementation: `TaskId::new(id: impl Into<String>) -> Self` accepts ANY string
- Validation happens separately in `validate_task_id()` - violates "Parse, Don't Validate" principle
- Makes illegal states representable (can create invalid TaskId)

**Required Fix**: 
```rust
impl TaskId {
    pub fn new(id: impl Into<String>) -> Result<Self, Error::InvalidTaskId> {
        let id = id.into();
        if id.is_empty() {
            return Err(Error::InvalidTaskId("Task ID cannot be empty".to_string()));
        }
        if !TASK_ID_PATTERN.is_match(&id) {
            return Err(Error::InvalidTaskId(format!(
                "Task ID must be alphanumeric with - or _, got: {}",
                id
            )));
        }
        Ok(Self(id))
    }
}
```

---

## Medium Defects (Should Fix)

### DEFECT-004: Potential YAGNI Violation - Demo Tasks
**Phase**: 5 (The Bitter Truth)
**Severity**: MEDIUM

**Location**: 
- Line 88-107: `init_demo_tasks()`
- Line 158-163: Lazy initialization in `list()`

**Issue**:
- Contract specifies: "Task operations use in-memory bead repository"
- No mention of demo data or auto-initialization
- `init_demo_tasks()` populates sample tasks on first list call
- Appears to be feature creep / YAGNI violation

**Required Fix**: Remove demo task initialization OR document as intentional UX feature

---

### DEFECT-005: Poison Error Swallowed
**Phase**: 4 (Ruthless Simplicity)
**Severity**: LOW

**Location**: `crates/cli/src/commands/task.rs` line 39

**Issue**:
```rust
.unwrap_or_else(|_| Vec::new())
```
- Swallows RwLock poison error silently
- In production, this could hide serious concurrency bugs

**Required Fix**: Either propagate the error or use `into_inner()` with proper error handling

---

## Approved Aspects

### What's Good:
- ✅ Phase 1: Contract parity (preconditions, postconditions, invariants all enforced)
- ✅ Phase 2: Function length < 25 lines, parameters < 5
- ✅ Phase 2: Clean separation of pure functions (task_validation.rs) from I/O (task.rs)
- ✅ Phase 3: TaskState enum makes illegal states unrepresentable
- ✅ Phase 3: Newtypes for TaskId, Title, Priority, Assignee
- ✅ Phase 4: No panic!, expect(), or unwrap()
- ✅ Phase 4: Explicit state transitions via transition_to_* functions
- ✅ Phase 5: Code is legible, no clever tricks

---

## Fix Priority Order

1. **DEFECT-002** (hardcoded user) - Breaks testing, most critical
2. **DEFECT-001** (no tests) - Hard requirement per Phase 2
3. **DEFECT-003** (TaskId validation) - Architectural issue
4. **DEFECT-004** (demo tasks) - Cleanup
5. **DEFECT-005** (poison error) - Minor cleanup
