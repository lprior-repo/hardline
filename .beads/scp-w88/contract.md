# Contract Specification

## Context
- **Feature**: Fix scenarios crate - add missing type annotations / configuration
- **Domain terms**:
  - `scenarios` crate - behavioral scenario vault with information barrier
  - Workspace inheritance - Cargo.toml pattern for sharing config across crates
  - Self-sufficient crate - can compile without workspace context
- **Assumptions**:
  - The scenarios crate code (lib.rs, runner.rs, scenario.rs, sanitizer.rs) is syntactically correct
  - The issue is with crate configuration preventing standalone compilation
  - The crate should compile when Cargo.toml has hardcoded values
- **Open questions**:
  - Was the crate intentionally excluded from workspace? If so, why?
  - What version should be hardcoded?

## Preconditions
- [P1] Original Cargo.toml uses workspace inheritance and fails to parse when built standalone
- [P2] All required dependencies are declared (may need workspace = true removal)
- [P3] Source files (lib.rs, runner.rs, scenario.rs, sanitizer.rs) are syntactically valid Rust

## Postconditions
- [Q1] Code compiles after changes: `cargo check` in crate directory returns exit code 0
- [Q2] No new warnings introduced: compilation produces no warnings (lib.rs has #![deny(warnings)])
- [Q3] Cargo.toml is self-sufficient (no workspace.workspace = true patterns)
- [Q4] Crate can be built in isolation without workspace context

## Invariants
- [I1] No new warnings introduced - compilation must be clean
- [I2] All existing tests continue to pass after changes
- [I3] The information barrier (sanitizer) functionality remains unchanged
- [I4] Public API signatures remain unchanged

## Error Taxonomy
Since this is a compilation fix (not runtime), there are no runtime error variants. Build errors encountered are:

- **BuildError::WorkspaceInheritFailed** - When Cargo.toml tries to inherit workspace values that don't exist
  - Occurs when: Building scenarios crate standalone or when workspace is not properly configured
  - Manifest error: "error inheriting `edition` from workspace root manifest's `workspace.package.edition`"

- **BuildError::MissingDependency** - When a required dependency is not declared
  - Occurs when: Dependencies used in source are not listed in Cargo.toml
  - Manifest error: "failed to resolve: could not find package..."

- **BuildError::ManifestParseError** - When Cargo.toml has invalid syntax
  - Occurs when: File has syntax errors or missing required fields
  - Manifest error: "failed to parse manifest"

## Contract Signatures
This is a compilation fix - no runtime functions. The relevant "signatures" are build configurations:

```toml
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

| Issue | Enforcement Level | Solution |
|---|---|---|
| Workspace inheritance | Build-time (cargo) | Hardcode values in Cargo.toml |
| Missing dependencies | Build-time (cargo check) | Add explicit dependencies |
| Type inference in source | Compile-time (rustc) | Add explicit annotations if needed |
| Warnings as errors | Compile-time (#![deny]) | Fix or allow specific lints |

## Violation Examples (REQUIRED -- one per precondition and postcondition)

### Precondition Violations
- VIOLATES P1: Running `cargo check` in crates/scenarios/ returns:
  ```
  error: failed to parse manifest at `.../Cargo.toml`
  Caused by: error inheriting `edition` from workspace root manifest's `workspace.package.edition`
  Caused by: failed to find a workspace root
  ```
  Should produce: Build failure (non-zero exit code)

- VIOLATES P2: If dependencies are missing, running `cargo check` returns:
  ```
  error: failed to resolve: could not find package...
  ```
  Should produce: Build failure

### Postcondition Violations
- VIOLATES Q1: After attempted fix, running `cargo check` returns non-zero exit code
- VIOLATES Q2: After fix, running `cargo check` produces warnings (lib.rs has `#![deny(warnings)]`)
- VIOLATES Q3: Cargo.toml still uses workspace inheritance patterns (e.g., `version.workspace = true`)
- VIOLATES Q4: Crate fails when workspace directory is unavailable

## Ownership Contracts (Rust-specific)
N/A - this is a crate configuration fix, not runtime code changes.

## Non-goals
- [ ] Add new features to scenarios crate
- [ ] Fix runtime bugs in scenarios code
- [ ] Modify workspace configuration for other crates
- [ ] Add integration with other system components
