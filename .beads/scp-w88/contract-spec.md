# Contract Specification

## Context
- **Feature**: Fix scenarios crate - add missing type annotations / configuration
- **Domain terms**: 
  - `scenarios` crate - behavioral scenario vault with information barrier
  - Type annotations - Rust compiler-inferred or explicit type information
  - Workspace inheritance - Cargo.toml pattern for sharing config across crates
- **Assumptions**:
  - The scenarios crate should compile as a standalone crate
  - The crate code (lib.rs, runner.rs, scenario.rs, sanitizer.rs) is syntactically correct
  - The issue is with crate configuration preventing compilation
- **Open questions**:
  - Was the crate intentionally excluded from workspace? If so, why?
  - Are there other crates depending on scenarios that would break if it's fixed?

## Preconditions
- [P1] Code compiles before changes: The scenarios crate fails to compile due to missing workspace inheritance configuration
- [P2] All required dependencies are available in crates/scenarios/Cargo.toml
- [P3] The crate files (lib.rs, runner.rs, scenario.rs, sanitizer.rs) are syntactically valid Rust

## Postconditions
- [Q1] Code compiles after changes: `cargo check -p scenarios` (or direct check) returns exit code 0
- [Q2] No new warnings introduced: compilation produces no warnings
- [Q3] All public types have explicit type annotations where needed for API clarity
- [Q4] Cargo.toml is self-sufficient (does not require workspace inheritance for basic build)

## Invariants
- [I1] No new warnings introduced - compilation must be clean (deny warnings enabled in lib.rs)
- [I2] All existing tests continue to pass after changes
- [I3] The information barrier (sanitizer) functionality remains unchanged
- [I4] All error types remain semantically equivalent

## Error Taxonomy
Since this is a compilation fix (not runtime), there are no runtime error variants to document. The errors encountered are:

- **BuildError::WorkspaceInheritFailed** - When Cargo.toml tries to inherit workspace values that don't exist
  - Occurs when: Building scenarios crate standalone or when workspace is not properly configured
  - Resolution: Provide local [package] section with required values

- **BuildError::MissingDependency** - When a required dependency is not declared
  - Occurs when: Dependencies used in source are not listed in Cargo.toml
  - Resolution: Add missing dependencies to Cargo.toml

- **BuildError::TypeInferenceFailed** - When compiler cannot infer a type (rare in annotated code)
  - Occurs when: Complex closures or return types lack explicit annotations
  - Resolution: Add explicit type annotations

## Contract Signatures
This is a compilation fix - no runtime functions to specify. The relevant "signatures" are build configurations:

```
# Cargo.toml must be self-sufficient:
[package]
name = "scenarios"
version = "0.5.0"  # Hardcoded (not workspace inherited)
edition = "2021"   # Hardcoded
rust-version = "1.80"

[dependencies]
# All explicit, no workspace inheritance
```

## Type Encoding
Since this is a build-time fix, type encoding is not applicable in the traditional sense. Instead:

| Issue | Enforcement Level | Solution |
|---|---|---|
| Workspace inheritance | Build-time | Hardcode values in Cargo.toml |
| Missing dependencies | Build-time (cargo check) | Add to Cargo.toml |
| Missing type annotations | Compile-time | Add explicit types where needed |
| Warnings as errors | Compile-time | Fix or annotate with allow(*) |

## Violation Examples (REQUIRED -- one per precondition and postcondition)

### Precondition Violations
- VIOLATES P1: Running `cargo check` in crates/scenarios/ returns:
  ```
  error: failed to parse manifest at `.../Cargo.toml`
  Caused by: error inheriting `edition` from workspace root manifest's `workspace.package.edition`
  ```
  Should produce: Build failure (non-zero exit code)

- VIOLATES P2: If dependencies are missing, running `cargo check` returns:
  ```
  error: failed to resolve: could not find package...
  ```
  Should produce: Build failure

### Postcondition Violations
- VIOLATES Q1: After fix, running `cargo check` returns non-zero exit code
- VIOLATES Q2: After fix, running `cargo check` produces warnings (lib.rs has `#![deny(warnings)]`)
- VIOLATES Q3: API types lack documentation or unclear signatures (subjective, check manually)
- VIOLATES Q4: Cargo.toml still uses workspace inheritance (e.g., `version.workspace = true`)

## Ownership Contracts (Rust-specific)
N/A - this is a crate configuration fix, not runtime code changes.

## Non-goals
- [ ] Add new features to scenarios crate
- [ ] Fix runtime bugs in scenarios code
- [ ] Modify workspace configuration for other crates
- [ ] Add integration with other system components
