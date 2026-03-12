# Black Hat Defects Report - Bead scp-5ne

## Status: REJECTED

## Critical Defects

### 1. CONTRACT VIOLATION - Missing null byte check in validate_agent_id

**Location**: `crates/core/src/validation/domain.rs:161-168`

**Issue**: Contract P3 states "Input string must not contain null bytes for all validation functions" but `validate_agent_id` does NOT check for `\0`.

**Contract Violation Example**:
```rust
// Contract says (line 51-52):
// VIOLATES P3: validate_agent_id("foo\0bar") -- returns Err(ValidationError::ShellMetacharacter)

// Implementation returns Ok(()) - WRONG!
validate_agent_id("foo\0bar") // returns Ok(())
```

**Fix Required**:
```rust
pub fn validate_agent_id(id: &str) -> Result<(), ValidationError> {
    if id.is_empty() {
        return Err(ValidationError::EmptyInput);
    }
    if id.contains('\0') {  // <-- ADD THIS CHECK
        return Err(ValidationError::ShellMetacharacter);
    }
    if contains_shell_metachar(id) {
        return Err(ValidationError::ShellMetacharacter);
    }
    Ok(())
}
```

---

### 2. CONTRACT VIOLATION - Missing null byte check in validate_task_id

**Location**: `crates/core/src/validation/domain.rs:186-193`

**Issue**: Contract P3 states "Input string must not contain null bytes for all validation functions" but `validate_task_id` does NOT check for `\0`.

**Contract Violation Example**:
```rust
// Contract says:
// VIOLATES P3: validate_task_id("task\0id") -- returns Err(ValidationError::ShellMetacharacter)

// Implementation returns Ok(()) - WRONG!
validate_task_id("task\0id") // returns Ok(())
```

**Fix Required**:
```rust
pub fn validate_task_id(id: &str) -> Result<(), ValidationError> {
    if id.is_empty() {
        return Err(ValidationError::EmptyInput);
    }
    if id.contains('\0') {  // <-- ADD THIS CHECK
        return Err(ValidationError::ShellMetacharacter);
    }
    if contains_shell_metachar(id) {
        return Err(ValidationError::ShellMetacharacter);
    }
    Ok(())
}
```

---

## Summary

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 1: Contract & Bead Parity | ❌ FAILED | 2 contract violations (P3 not enforced) |
| Phase 2: Farley Engineering Rigor | ✅ PASSED | All functions < 25 lines, < 5 params |
| Phase 3: NASA-Level Functional Rust | ✅ PASSED | Good newtypes, parse don't validate |
| Phase 4: Ruthless Simplicity & DDD | ✅ PASSED | No Option state machines, no boolean flags |
| Phase 5: Bitter Truth | ✅ PASSED | Code is boring and legible |

**Required Action**: Fix the two null byte checks in `validate_agent_id` and `validate_task_id` to match contract P3.
