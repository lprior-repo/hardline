# Black Hat Defects v2: scp-pmw

## Summary
**Status**: REJECTED  
**Review Date**: 2026-03-11  
**Files**: `crates/cli/src/commands/workspace.rs`

---

## Critical Defects

### 🔴 DEFECT 1: Validation Regex Incomplete
**Phase**: 1 (Contract Parity)  
**Severity**: HIGH  
**Location**: `validate_workspace_name`, lines 46-58  

**Contract Requirement**: 
```
P1: Workspace name must be valid identifier (non-empty, alphanumeric + dash/underscore)
Regex: ^[a-zA-Z][a-zA-Z0-9_-]*$
```

**Current Implementation**:
```rust
// Only checks first char is alphabetic - DOES NOT validate
// that remaining chars are alphanumeric/dash/underscore!
if !name
    .chars()
    .next()
    .map(|c| c.is_alphabetic())
    .unwrap_or(false)
{
    return Some(Error::InvalidIdentifier(...));
}
// MISSING: Validation that rest of string is [a-zA-Z0-9_-]*
```

**Violation**: 
- VIOLATES P1: `spawn("abc@#$%", false)` should fail but passes validation
- VIOLATES P1: `spawn("valid-name!", false)` should fail but passes validation

**Fix Required**:
```rust
fn validate_workspace_name(name: &str) -> Option<Error> {
    if name.is_empty() {
        return Some(Error::InvalidIdentifier(
            "workspace name cannot be empty".to_string(),
        ));
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();
    
    // Must start with a letter
    if !first.is_alphabetic() {
        return Some(Error::InvalidIdentifier(format!(
            "workspace name must start with a letter, got '{}'",
            name
        )));
    }

    // Remaining chars must be alphanumeric, dash, or underscore
    if !chars.all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Some(Error::InvalidIdentifier(format!(
            "workspace name must be alphanumeric, dash, or underscore only, got '{}'",
            name
        )));
    }

    None
}
```

---

### 🟠 DEFECT 2: Function Length Violations
**Phase**: 2 (Farley Engineering Rigor)  
**Severity**: MEDIUM  
**Location**: Multiple functions  

**Contract Requirement**: Functions MUST be <25 lines

**Violations**:

| Function | Current Lines | Limit | Excess |
|----------|---------------|-------|--------|
| `spawn` | 31 | 25 | +6 |
| `done` | 36 | 25 | +11 |
| `abort` | 41 | 25 | +16 |
| `switch` | 31 | 25 | +6 |

**Fix Required**: Break into smaller helper functions. Example for `spawn`:
```rust
// Extract I/O to separate functions
fn spawn_create_workspace(backend: &dyn VcsBackend, name: &str) -> Result<()> {
    backend.create_workspace(name)?;
    Output::success(&format!("Created workspace '{}'", name));
    Ok(())
}

fn spawn_sync_with_main(backend: &dyn VcsBackend) -> Result<()> {
    backend.switch_workspace(name)?;
    backend.rebase("main")?;
    Output::success("Synced with main");
    Ok(())
}
```

---

### 🟠 DEFECT 3: Missing Test Module
**Phase**: 5 (The Bitter Truth)  
**Severity**: HIGH  
**Location**: Entire file  

**Contract Requirement**: Tests must verify violation examples from contract

**Current State**: No `#[cfg(test)]` module exists in workspace.rs

**Missing Tests** (from contract VIOLATION EXAMPLES):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_empty_name_returns_error() {
        let result = spawn("", false);
        assert!(matches!(result, Err(Error::InvalidIdentifier(_))));
    }

    #[test]
    fn test_spawn_invalid_name_returns_error() {
        let result = spawn("123invalid", false);
        assert!(matches!(result, Err(Error::InvalidIdentifier(_))));
    }

    #[test]
    fn test_spawn_valid_name_with_special_char_fails() {
        // This will FAIL with current implementation - demonstrating the bug
        let result = spawn("valid-name!", false);
        assert!(matches!(result, Err(Error::InvalidIdentifier(_))));
    }

    #[test]
    fn test_abort_main_returns_error() {
        let result = abort(Some("main"));
        assert!(matches!(result, Err(Error::InvalidOperation(_))));
    }

    #[test]
    fn test_validate_workspace_name_valid_patterns() {
        assert!(validate_workspace_name("abc").is_none());
        assert!(validate_workspace_name("abc-def").is_none());
        assert!(validate_workspace_name("abc_def").is_none());
        assert!(validate_workspace_name("abc123").is_none());
    }

    #[test]
    fn test_validate_workspace_name_invalid_patterns() {
        assert!(validate_workspace_name("").is_some());
        assert!(validate_workspace_name("123abc").is_some());
        assert!(validate_workspace_name("abc@def").is_some());
        assert!(validate_workspace_name("abc!").is_some());
    }
}
```

---

## Enforcement Summary

| Phase | Status | Issues |
|-------|--------|--------|
| Phase 1: Contract & Bead Parity | ❌ FAIL | Validation regex incomplete |
| Phase 2: Farley Rigor | ❌ FAIL | 4 functions > 25 lines |
| Phase 3: NASA-Level Functional Rust | ✅ PASS | - |
| Phase 4: Ruthless Simplicity | ✅ PASS | - |
| Phase 5: Bitter Truth | ❌ FAIL | No tests |

**Total Critical Defects**: 3  
**Decision**: REJECTED - Fix all defects before re-review.

---

## Recommended Fix Order

1. **FIX 1 first**: Validation regex bug (blocks contract compliance)
2. **FIX 2**: Break long functions (maintainsability)
3. **FIX 3**: Add tests (verifies FIX 1 works)

EOF
