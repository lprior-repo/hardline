# Black Hat Defects - Bead scp-5ne

## Phase 1: Contract Parity - FAIL

### CRITICAL: Contract Signature Mismatch
**Location**: `crates/core/src/validation/domain.rs` - All functions

**Contract specifies**:
```rust
pub fn validate_session_name(name: &str) -> Result<(), ValidationError>
```

**Implementation returns**:
```rust
pub fn validate_session_name(s: &str) -> Result<(), Error>
```

The contract explicitly requires:
- `ValidationError` enum with `ValidationError::EmptyInput` variant
- `ValidationError` enum with `ValidationError::ShellMetacharacter` variant

But the implementation uses `crate::error::Error::ValidationFieldError` - completely different type.

**Violation Examples from contract do NOT match implementation**:
| Contract Example | Expected Return | Actual Return |
|-----------------|------------------|---------------|
| `validate_session_name("")` | `Err(ValidationError::EmptyInput)` | `Err(Error::ValidationFieldError { message: "session name cannot be empty", ... })` |
| `validate_session_name("foo&bar")` | `Err(ValidationError::ShellMetacharacter)` | `Err(Error::ValidationFieldError { message: "...must not contain shell metacharacters", ... })` |

---

## Phase 2: Farley Engineering Rigor - FAIL

### Function Length Violations (Hard Constraint: <25 lines)
| Function | Lines | Limit | Status |
|----------|-------|-------|--------|
| `validate_session_name` (lines 45-94) | 50 | 25 | **FAIL** |
| `validate_agent_id` (lines 96-132) | 37 | 25 | **FAIL** |
| `validate_workspace_name` (lines 134-165) | 32 | 25 | **FAIL** |
| `validate_task_id` (lines 167-197) | 31 | 25 | **FAIL** |
| `validate_absolute_path` (lines 223-268) | 46 | 25 | **FAIL** |

### Redundant Validation
**Location**: Lines 276-295 (`validate_workspace_name_safe`)

This function calls `validate_workspace_name(s)?` (line 277) which already validates shell metacharacters, then re-validates a subset (lines 279-292). This is redundant bloat.

---

## Phase 3: NASA-Level Functional Rust (Big 6) - FAIL

### No Newtypes (Primitive Obsession)
**Contract requires domain terms**: session names, agent IDs, workspace names, task IDs

**Implementation uses**: Raw `&str` everywhere

**Required but missing**:
```rust
pub struct SessionName(String);
pub struct AgentId(String);
pub struct WorkspaceName(String);
pub struct TaskId(String);
pub struct AbsolutePath(String);
```

### Error Type Mismatch
**Contract specifies**:
```rust
pub enum ValidationError {
    EmptyInput,
    ShellMetacharacter,
}
```

**Implementation uses**: `Error::ValidationFieldError { message, field, value }`

The contract's error taxonomy is NOT implemented.

---

## Phase 4: Ruthless Simplicity & DDD - FAIL

### Primitive Obsession Violation
- All functions accept `&str` instead of newtype wrappers
- No type-level enforcement of valid/invalid states
- Can't make illegal states unrepresentable

### Code Duplication
- `contains_shell_metachar()` helper (lines 30-32) is good
- But validation logic in each function repeats similar patterns (empty check, shell check)
- Should be composed from smaller validators

---

## Phase 5: The Bitter Truth - FAIL

### YAGNI Violation
**File**: `crates/core/src/validation/validators.rs` - 811 lines

This file provides generic validator combinators (`ValidationRule`, `ComposedValidator`, `MappedValidator`, etc.) but:
1. The actual domain validation in `domain.rs` does NOT use these combinators
2. It's unused infrastructure that adds complexity without benefit
3. The contract-only feature has 811 lines of "framework" that doesn't match the contract

### Velocity & Legibility
- Functions are too long to be "painfully obvious"
- Error types don't mental translation match contract, requiring
- The `validators.rs` file appears to be "clever" framework-building rather than boring, direct code

---

## Summary

| Phase | Status | Critical Defects |
|-------|--------|------------------|
| 1. Contract Parity | **REJECT** | Error type mismatch (ValidationError vs Error) |
| 2. Farley Rigor | **REJECT** | 5 functions >25 lines, redundant validation |
| 3. Big 6 | **REJECT** | No newtypes, error type mismatch |
| 4. DDD | **REJECT** | Primitive obsession, no domain types |
| 5. Bitter Truth | **REJECT** | 811-line unused validators.rs, YAGNI |

---

## Required Fixes

1. **Define contract-specified error type**:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub enum ValidationError {
       EmptyInput,
       ShellMetacharacter,
   }
   ```

2. **Create newtypes** for domain identifiers

3. **Refactor functions** to <25 lines each by composing smaller validators

4. **Delete or use** `validators.rs` - don't leave unused framework code

5. **Match contract signatures exactly**:
   ```rust
   pub fn validate_session_name(name: &str) -> Result<(), ValidationError>
   ```

---

**VERDICT**: REJECT - Code fails Phase 1 contract parity fundamentally. Error types don't match contract specification.
