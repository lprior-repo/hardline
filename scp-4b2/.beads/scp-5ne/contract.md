# Contract Specification

## Context
- Feature: Add validation module with pure functions
- Domain terms: session names, agent IDs, workspace names, task IDs, absolute paths, shell metacharacters
- Assumptions: These validation functions will be used in a CLI context where shell metacharacters pose a security risk
- Open questions: None

## Preconditions
- [P1] Input string must be non-empty for all validation functions
- [P2] validate_absolute_path requires input to be a valid path format (not empty, no null bytes)
- [P3] Input string must not contain null bytes for all validation functions

## Postconditions
- [Q1] validate_session_name returns Ok(()) for valid session names, Err(ValidationError::EmptyInput) for empty, Err(ValidationError::ShellMetacharacter) for shell metacharacters
- [Q2] validate_agent_id returns Ok(()) for valid agent IDs, Err(ValidationError::EmptyInput) for empty, Err(ValidationError::ShellMetacharacter) for shell metacharacters
- [Q3] validate_workspace_name returns Ok(()) for valid workspace names, Err(ValidationError::EmptyInput) for empty, Err(ValidationError::ShellMetacharacter) for shell metacharacters
- [Q4] validate_task_id returns Ok(()) for valid task IDs, Err(ValidationError::EmptyInput) for empty, Err(ValidationError::ShellMetacharacter) for shell metacharacters
- [Q5] validate_absolute_path returns Ok(()) for valid absolute paths, Err(ValidationError::EmptyInput) for empty, Err(ValidationError::ShellMetacharacter) for shell metacharacters

## Invariants
- [I1] All validation functions are pure - no side effects, same input always produces same output
- [I2] Functions return Result<(), ValidationError> - never panic
- [I3] All functions filter shell metacharacters: `;&$#()*?|><[]{}'"`\n,` and backticks

## Error Taxonomy
- ValidationError::EmptyInput - when input is empty string
- ValidationError::ShellMetacharacter - when input contains shell metacharacters (applied to all validators)

## Contract Signatures
```rust
pub fn validate_session_name(name: &str) -> Result<(), ValidationError>
pub fn validate_agent_id(id: &str) -> Result<(), ValidationError>
pub fn validate_workspace_name(name: &str) -> Result<(), ValidationError>
pub fn validate_task_id(id: &str) -> Result<(), ValidationError>
pub fn validate_absolute_path(path: &str) -> Result<(), ValidationError>
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| string non-empty | Runtime-checked constructor | `validate_input().is_empty()` check returns Err |
| shell metachar filter | Runtime (strongest) | Iterate and check each char against set |

## Violation Examples (REQUIRED)
- VIOLATES P1: `validate_session_name("")` -- returns `Err(ValidationError::EmptyInput)`
- VIOLATES P1: `validate_agent_id("")` -- returns `Err(ValidationError::EmptyInput)`
- VIOLATES P1: `validate_workspace_name("")` -- returns `Err(ValidationError::EmptyInput)`
- VIOLATES P1: `validate_task_id("")` -- returns `Err(ValidationError::EmptyInput)`
- VIOLATES P1: `validate_absolute_path("")` -- returns `Err(ValidationError::EmptyInput)`
- VIOLATES P3: `validate_session_name("foo\0bar")` -- returns `Err(ValidationError::ShellMetacharacter)` (null byte treated as invalid)
- VIOLATES P3: `validate_absolute_path("/path\0/invalid")` -- returns `Err(ValidationError::ShellMetacharacter)`
- VIOLATES Q1: `validate_session_name("foo&bar")` -- returns `Err(ValidationError::ShellMetacharacter)`
- VIOLATES Q2: `validate_agent_id("agent$test")` -- returns `Err(ValidationError::ShellMetacharacter)`
- VIOLATES Q3: `validate_workspace_name("work|space")` -- returns `Err(ValidationError::ShellMetacharacter)`
- VIOLATES Q4: `validate_task_id("task;cmd")` -- returns `Err(ValidationError::ShellMetacharacter)`
- VIOLATES Q5: `validate_absolute_path("/path/with`backtick`")` -- returns `Err(ValidationError::ShellMetacharacter)`

## Ownership Contracts (Rust-specific)
- All functions take `&str` - borrowed input, no ownership transferred
- No mutation - all functions are pure
- No clone required - input remains owned by caller

## Non-goals
- [ ] Path existence checking (only format validation)
- [ ] Network or file I/O operations
- [ ] Unicode normalization
