# Black Hat Defects: Bead scp-pmw

## Summary
**STATUS: REJECTED** - Critical contract violations found.

---

## Critical Defects (Must Fix)

### 1. P4 Violation: Missing Dirty Working Copy Check in `done()`
**Location**: `crates/cli/src/commands/workspace.rs` lines 151-184

**Issue**: The `done()` function does NOT check for uncommitted changes before performing rebase and push operations. This violates precondition P4: "Working copy must be clean before switch/done/abort operations".

**Impact**: If user has uncommitted changes, the VCS will fail with confusing error messages instead of providing a clean validation error.

**Fix Required**: Add dirty check at start of `done()`:
```rust
let status = backend.status()?;
if status != VcsStatus::Clean {
    return Err(Error::WorkingCopyDirty);
}
```

---

### 2. P4 Violation: Missing Dirty Working Copy Check in `abort()`
**Location**: `crates/cli/src/commands/workspace.rs` lines 187-214

**Issue**: Same as above - `abort()` does not check for dirty working copy before deleting workspace.

**Impact**: May leave repository in inconsistent state.

**Fix Required**: Add dirty check at start of `abort()`:
```rust
let status = backend.status()?;
if status != VcsStatus::Clean {
    return Err(Error::WorkingCopyDirty);
}
```

---

### 3. P3 Improper Handling: "current" Workspace Name
**Location**: `crates/cli/src/commands/workspace.rs` lines 151-166

**Issue**: 
```rust
let workspace_name = name.unwrap_or("current");
```
Uses literal string "current" which is NOT a valid jj workspace name. When `name` is None, the existence check at lines 162-166 will always fail (or use incorrect logic).

**Impact**: `done(None)` will not work correctly.

**Fix Required**: Query the current workspace name from backend when name is None.

---

## High Priority Defects

### 4. Function Line Count Violations
**Location**: Multiple functions

| Function | Current Lines | Limit | Violation |
|----------|---------------|-------|------------|
| `spawn()` | 57 | 25 | Yes |
| `done()` | 34 | 25 | Yes |
| `abort()` | 28 | 25 | Yes |
| `next()` | 42 | 25 | Yes |
| `prev()` | 46 | 25 | Yes |
| `switch()` | 31 | 25 | Yes |

**Fix Required**: Break down into smaller functions following Single Responsibility Principle.

---

### 5. Parse Not Validate - I/O Before Validation
**Location**: `crates/cli/src/commands/workspace.rs` lines 35-38

**Issue**: 
```rust
let cwd = std::env::current_dir().map_err(Error::Io)?;
let backend = vcs::create_backend(&cwd)?;
```
I/O operations happen BEFORE workspace name validation (lines 14-31). Expensive I/O should happen after cheap validation.

---

## Medium Priority Defects

### 6. Boolean Parameter - Anti-Pattern
**Location**: `crates/cli/src/commands/workspace.rs` line 12

**Issue**: `sync: bool` is not self-documenting.

**Fix Required**: Use enum `SyncOption { NoSync, WithSync }`.

---

### 7. Mixed Output Styles
**Location**: Throughout file

**Issue**: Uses both `Output::info/success()` and raw `println!()` inconsistently.

**Examples**:
- Lines 33, 48, 53: `Output::info/success()`
- Lines 197, 212, 224: `println!()`

**Fix Required**: Pick one output abstraction and use consistently.

---

### 8. Code Duplication
**Location**: Lines 434-438 and 478-482

**Issue**: Identical sorting logic duplicated in `next()` and `prev()`.

**Fix Required**: Extract to shared helper function.

---

## Test Coverage Gap

### 9. Missing Unit Tests
**Location**: No tests exist for workspace commands

**Issue**: `martin-fowler-tests.md` specifies 20+ test cases including:
- test_spawn_fails_with_empty_name
- test_spawn_fails_with_invalid_identifier  
- test_spawn_fails_when_workspace_exists
- test_done_fails_when_workspace_not_found
- test_abort_fails_when_workspace_not_found
- test_abort_fails_when_trying_to_abort_main
- test_switch_fails_with_dirty_working_copy

**None of these tests are implemented.**

---

## Contract Postcondition Issues

### 10. Q3 Postcondition Ambiguity
**Location**: Lines 175-180

**Issue**: Contract states "After done: Working copy is rebased on main and pushed to remote" but code does NOT switch to main after completing. User remains in the completed workspace.

**Clarification Needed**: Is this intended behavior or should we switch to main?

---

## Violation Summary

| Phase | Violations Count |
|-------|-----------------|
| Phase 1 (Contract) | 3 Critical |
| Phase 2 (Farley) | 6 functions over limit |
| Phase 3 (Big 6) | 3 issues |
| Phase 4 (DDD) | 4 issues |
| Phase 5 (Bitter Truth) | 2 YAGNI issues + test gap |

**Total Critical Issues**: 3
**Total Issues**: 10
