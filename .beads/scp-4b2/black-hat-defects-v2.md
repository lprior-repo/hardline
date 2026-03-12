# Black Hat Defects Report v2: Bead scp-4b2

## Summary
**STATUS: REJECTED** - Critical contract violations remain despite partial fixes.

---

## Phase 1: Contract Parity - CRITICAL FAILURES

### DEFECT-001: Wrong Return Type for wait.rs
- **Severity**: CRITICAL (P0)
- **Location**: `crates/cli/src/commands/wait.rs:51`
- **Issue**: `run()` returns `Result<()>` but contract (line 156) specifies `Result<WaitResult, Error>`
- **Contract Reference**: Contract signature requires `WaitResult` enum with `ConditionMet` and `Timeout` variants

### DEFECT-002: Primitive Obsession - SessionName
- **Severity**: HIGH (P1)
- **Location**: `crates/cli/src/commands/wait.rs:47, 126`
- **Issue**: Uses raw `&str` and `String` instead of domain `SessionName` type
- **Contract Reference**: Contract line 241 specifies "SessionName - Borrowed, read-only"

### DEFECT-003: Primitive Obsession - CheckpointId
- **Severity**: HIGH (P1)
- **Location**: `crates/cli/src/commands/batch.rs:13, 18`
- **Issue**: Uses `String` for checkpoint_id instead of `CheckpointId` newtype
- **Contract Reference**: Contract lines 172-173, 177

### DEFECT-004: BatchResult::RolledBack Missing Error Field
- **Severity**: HIGH (P1)
- **Location**: `crates/cli/src/commands/batch.rs:17-20`
- **Issue**: Contract (line 178) specifies `RolledBack { checkpoint_id, error: Error }` but implementation has `results` field instead
- **Contract Reference**: Contract line 178

---

## Phase 2: Farley Rigor Flaws

### DEFECT-005: run() Function Exceeds 25 Lines
- **Severity**: HIGH (P1)
- **Location**: `crates/cli/src/commands/wait.rs:46-116`
- **Issue**: Function is **71 lines**, far exceeding 25-line limit
- **Required**: Split into: validate_inputs(), create_wait_loop(), check_and_return()

### DEFECT-006: execute_batch() Exceeds 25 Lines  
- **Severity**: HIGH (P1)
- **Location**: `crates/cli/src/commands/batch.rs:112-150`
- **Issue**: Function is **39 lines**, exceeds 25-line limit

---

## Phase 3: Functional Rust Flaws (The Big 6)

### DEFECT-007: Unwrap in Production Code
- **Severity**: HIGH (P1)
- **Location**: `crates/cli/src/commands/batch.rs:106`
- **Issue**: `.unwrap_or(0)` on SystemTime calculation - panic vector
- **Required**: Use proper error handling with map_err

### DEFECT-008: No CheckpointId Newtype
- **Severity**: MEDIUM (P2)
- **Location**: `crates/cli/src/commands/batch.rs`
- **Issue**: Should have `struct CheckpointId(String)` newtype per contract

### DEFECT-009: Poll Interval Validation Silent
- **Severity**: MEDIUM (P2)
- **Location**: `crates/cli/src/commands/wait.rs:70`
- **Issue**: Uses `.clamp()` silently instead of returning error or using named constants with explicit validation
- **Contract Reference**: Contract line 91-92 specifies invariant with MIN/MAX constants

---

## Phase 4: Simplicity & DDD Failures

### DEFECT-010: I/O Inside Pure Function
- **Severity**: HIGH (P1)
- **Location**: `crates/cli/src/commands/wait.rs:119-139`
- **Issue**: `check_condition()` does file I/O (`current_dir()`, `list_workspaces()`) mixed with logic - violates Functional Core / Imperative Shell
- **Required**: Move I/O to shell, pass validated data to pure function

### DEFECT-011: Missing WaitResult Return Type
- **Severity**: HIGH (P1)
- **Location**: `crates/cli/src/commands/wait.rs`
- **Issue**: Returns `Ok(())` on success instead of `Ok(WaitResult::ConditionMet { session, state })`
- **Contract Reference**: Contract lines 143-148

### DEFECT-012: SessionNotFound Handling Inconsistent
- **Severity**: MEDIUM (P2)
- **Location**: `crates/cli/src/commands/wait.rs:103-110`
- **Issue**: For session-exists mode, swallows error and continues. Contract (line 69-70) says should return SessionNotFound error

---

## Phase 5: The Bitter Truth

### DEFECT-013: Stub Rollback Implementation
- **Severity**: HIGH (P1)
- **Location**: `crates/cli/src/commands/batch.rs:178-183`
- **Issue**: `perform_rollback()` just prints message and returns Ok - no actual state restoration
- **Impact**: Rollback feature is not implemented, just stub code

### DEFECT-014: Status Check Uses Substring
- **Severity**: LOW (P3)
- **Location**: `crates/cli/src/commands/wait.rs:135`
- **Issue**: `ws.branch.contains(expected)` - substring matching is imprecise for status checking

### DEFECT-015: Dry-Run Skips Checkpoint Creation
- **Severity**: MEDIUM (P2)
- **Location**: `crates/cli/src/commands/batch.rs:59-61`
- **Issue**: Returns early without creating checkpoint, but contract (line 185-186) implies checkpoint should be created for validation

---

## Required Fixes

1. **wait.rs**:
   - Create `WaitResult` enum with `ConditionMet` and `Timeout` variants
   - Return `Result<WaitResult, Error>` instead of `Result<()>`
   - Create `SessionName` newtype wrapper
   - Split `run()` into smaller functions (<25 lines each)
   - Move I/O to imperative shell, keep pure logic separate

2. **batch.rs**:
   - Fix `BatchResult::RolledBack` to have `error: Error` field
   - Change `checkpoint_id: Option<String>` to `CheckpointId` newtype
   - Implement actual rollback logic in `perform_rollback()`
   - Remove `.unwrap_or(0)` - use proper error handling
   - Split `execute_batch()` into smaller functions

---

**VERDICT**: REJECTED - Contract parity FAILED on fundamental type signatures. wait.rs returns wrong type, batch.rs has malformed BatchResult. Farley constraints violated (71-line function). Stub rollback not implemented.

