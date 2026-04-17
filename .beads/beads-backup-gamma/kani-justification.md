---
name: kani-justification
description: Justification for partial Kani verification on hl-9nb, hl-c18, hl-d3r
type: reference
---

# Kani Justification for Partial Verification

**Date**: 2026-03-30
**Scope**: hl-9nb, hl-c18, hl-d3r

## What Was Verified

### Session crate (hl-c18): 1 harness VERIFIED
- `prove_migration_version_positive_accepted` — **2966 checks, 0 failures, VERIFIED**
- Proves: `MigrationVersion::new()` with positive i64 always succeeds and returns correct value
- Verification time: 0.62s

### Harnesses Written (not yet run to completion)
- `crates/session/src/infrastructure/migration_kani.rs` — 5 harnesses for MigrationVersion and validate_migration_name
- `crates/cli/src/commands/ai_kani.rs` — Kani harnesses for format_session_count, determine_ready_state
- `crates/cli/src/commands/task_kani.rs` — Kani harnesses for validate_task_command, truncate_description

## Why Full Verification Did Not Complete

1. **Unicode table explosion**: `validate_migration_name` calls `.chars().all()` which triggers Kani's symbolic execution through Rust's Unicode tables (~1500+ entries). This causes massive unwinding (iteration 3578+ without convergence).

2. **Resource limits**: Full `cargo kani` run on session crate hit OOM/timeout (killed with SIGTERM 144 after 10+ minutes per harness).

3. **Async dependency chains**: The migration functions are `async` and use `sqlx::SqlitePool`, which Kani cannot model without mocking the entire async runtime.

## What Existing Tests Cover Instead

- **253 nextest tests** all passing across ai/task/session crates
- **32 Red Queen adversarial tests** including boundary and edge cases
- **162 session tests** including v2 migration up/down/idempotency/rollback
- **proptest** infrastructure exists in Cargo.toml for fuzz-style testing

## Recommended Fix for Full Kani

1. Replace `.chars().all()` with `is_ascii_alphanumeric()` (reduces Unicode table explosion)
2. Extract pure validation logic into non-async functions that Kani can verify without sqlx
3. Add `kani::assume()` to constrain input ranges for string-based proofs
4. Run individual harnesses with `--harness` flag rather than full crate verification
