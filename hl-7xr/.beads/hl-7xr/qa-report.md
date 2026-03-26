# QA Report: PostgreSQL Integration Tests - Worktree Crate

**Date:** Wed Mar 25 2026  
**Target:** `crates/worktree/tests/postgres_repository_integration.rs`  
**QA Enforcer:** Automated QA Execution

---

## Execution Evidence

### Test Execution

```bash
cd /home/lewis/src/hardline && cargo test --package worktree --test postgres_repository_integration 2>&1
```

**Exit Code:** 0 (test command completed, but tests FAILED)

**Summary:**
- Total tests: 102
- Passed: 35
- Failed: 67

### PostgreSQL Status

```bash
$ pg_isready -h localhost -p 5432
localhost:5432 - accepting connections
```

PostgreSQL IS running and accepting connections.

```bash
$ psql "postgres://postgres:postgres@localhost:5432/postgres" -c "SELECT COUNT(*) FROM worktrees;" 2>&1
```

**Before tests (stale data):**
```
 count 
-------
   225
(1 row)
```

**After reset:**
```bash
$ psql "postgres://postgres:postgres@localhost:5432/postgres" -c "DROP DATABASE IF EXISTS worktree_test;" && psql "postgres://postgres:postgres@localhost:5432/postgres" -c "CREATE DATABASE worktree_test;" 2>&1
```

---

## Phase 1 — Discovery

### Test File Analysis

The test file `crates/worktree/tests/postgres_repository_integration.rs` contains:
- 102 integration tests for `PostgresWorktreeRepository`
- Uses SQLx for PostgreSQL connections
- Test database: `postgres://postgres:postgres@localhost:5432/worktree_test`
- Tests cover: CRUD operations, state transitions, concurrent access, metadata, edge cases

### Test Structure

```rust
const POSTGRES_TEST_DB: &str = "postgres://postgres:postgres@localhost:5432/worktree_test";
```

---

## Phase 2 — Happy Path

### Initial Test Run (Stale Database)

```bash
cargo test --package worktree --test postgres_repository_integration 2>&1 | head -100
```

**Result:** 81 passed, 21 failed

**Issues found:**
- Database had 225 stale rows from previous test runs
- Tests expecting empty database were failing
- State tests failing due to leftover data affecting assertions

### Database Reset and Second Run

```bash
psql "postgres://postgres:postgres@localhost:5432/postgres" -c "DROP DATABASE IF EXISTS worktree_test;" && psql "postgres://postgres:postgres@localhost:5432/postgres" -c "CREATE DATABASE worktree_test;"
```

**Result:** 50 passed, 52 failed

**Remaining issues:**
- Tests now failing with schema constraint violations
- Error: `duplicate key value violates unique constraint "pg_type_typname_nsp_index"`
- Error: `duplicate key value violates unique constraint "pg_class_relname_nsp_index"`

### Third Run (Fresh Database, Parallel Tests)

```bash
cargo test --package worktree --test postgres_repository_integration 2>&1
```

**Result:** 35 passed, 67 failed

---

## Phase 3 — Hostile Interrogation

### Test 1: Invalid Path Format

**Command (implicit in test):**
```rust
Worktree::new(
    WorktreeName::new("valid-name").unwrap(),
    AbsolutePath::new("invalid-relative-path").unwrap(), // This should fail
    AbsolutePath::new("/home/user/proj").unwrap(),
    WorktreeTypeEnum::Development,
    None,
)
```

**Actual Output:**
```
thread 'postgres_repository_integration::error_invalid_path_format' panicked at 
crates/worktree/tests/postgres_repository_integration.rs:859:56:
called `Result::unwrap()` on an `Err` value: InvalidPath("Path is not absolute: invalid-relative-path")
```

**Expected:** Test should verify error handling for invalid paths
**Actual:** The error IS being caught correctly (InvalidPath), but the test is incorrectly using `.unwrap()` instead of checking the error

**Verdict:** ❌ Test design error - test itself is incorrectly written

### Test 2: Database Constraint Violations (Most Common Failure)

**Error Pattern:**
```
called `Result::unwrap()` on an `Err` value: InvalidPath("Failed to create worktrees table: error returned from database: duplicate key value violates unique constraint \"pg_type_typname_nsp_index\"")
```

**Analysis:** This is a PostgreSQL internal system table constraint violation, NOT a test data issue. This suggests the test repository initialization code is attempting to create tables/indexes that conflict with PostgreSQL's internal schema.

**Root Cause:** The `PostgresWorktreeRepository::new()` method is calling migration scripts that are trying to create indexes with names that conflict with PostgreSQL's internal `pg_class` and `pg_type` system tables.

### Test 3: Concurrent State Updates

**Command (implicit):**
```rust
let mut worktree = create_test_worktree("concurrent-state", "/tmp/wt", "/home/user/proj", WorktreeTypeEnum::Development, None);
worktree.initialize().unwrap();
repo.save(&mut worktree).await.unwrap();

for i in 0..5 {
    let mut wt = repo.find_by_id(worktree.id()).await.unwrap().unwrap();
    if i % 2 == 0 {
        wt.suspend().unwrap();
    } else {
        wt.resume().unwrap();
    }
    repo.save(&mut wt).await.unwrap();
}
```

**Expected:** Final state should be `Active` (5 iterations: suspend, resume, suspend, resume, suspend = ends in Suspended, but test expects Active)

**Actual Output:**
```
assertion `left == right` failed
  left: Suspended
 right: Active
```

**Analysis:** Test expects Active but got Suspended. With 5 iterations starting from Active:
- i=0: suspend → Suspended
- i=1: resume → Active
- i=2: suspend → Suspended
- i=3: resume → Active
- i=4: suspend → Suspended

**Verdict:** Test expectation is WRONG. Final state SHOULD be Suspended, not Active.

---

## Findings

### CRITICAL (block merge)

#### CRITICAL-001: Test Design - error_invalid_path_format

**File:** `crates/worktree/tests/postgres_repository_integration.rs:859`

**Command:** Test execution revealed:
```rust
let result = Worktree::new(..., AbsolutePath::new("invalid-relative-path").unwrap(), ...);
assert!(result.is_err());  // This line is never reached
```

**Evidence:**
```
thread 'postgres_repository_integration::error_invalid_path_format' panicked at 
crates/worktree/tests/postgres_repository_integration.rs:859:56:
called `Result::unwrap()` on an `Err` value: InvalidPath("Path is not absolute: invalid-relative-path")
```

**Root Cause:** The test uses `.unwrap()` on `AbsolutePath::new()` which panics when given invalid input, instead of testing the error path.

**Reproduction:**
1. Run: `cargo test --package worktree --test postgres_repository_integration error_invalid_path_format`
2. Observe panic at line 859

**Expected:** Test should be:
```rust
let result = AbsolutePath::new("invalid-relative-path");
assert!(result.is_err());
```

**Action:** Fix test to not unwrap on expected error path.

---

#### CRITICAL-002: Test Design - concurrent_state_updates

**File:** `crates/worktree/tests/postgres_repository_integration.rs:1012`

**Evidence:**
```
assertion `left == right` failed
  left: Suspended
 right: Active
```

**Root Cause:** Test expectation is mathematically incorrect. 5 state transitions from Active with pattern (suspend, resume, suspend, resume, suspend) ends in Suspended.

**Reproduction:**
1. Run: `cargo test --package worktree --test postgres_repository_integration concurrent_state_updates`
2. Observe final state assertion failure

**Expected:** Either:
- Change test to expect `Suspended`, OR
- Change test to use 6 iterations (even number) to end in Active

**Action:** Fix test expectation.

---

### MAJOR (fix before merge)

#### MAJOR-001: PostgreSQL Schema Migration Conflicts

**File:** `crates/worktree/src/infrastructure/sqlx/postgres.rs` (inferred)

**Evidence:**
```
called `Result::unwrap()` on an `Err` value: InvalidPath("Failed to create worktrees table: error returned from database: duplicate key value violates unique constraint \"pg_type_typname_nsp_index\"")
```

**Reproduction:**
```bash
# Fresh database
psql "postgres://postgres:postgres@localhost:5432/postgres" -c "DROP DATABASE IF EXISTS worktree_test;" && psql "postgres://postgres:postgres@localhost:5432/postgres" -c "CREATE DATABASE worktree_test;"
cargo test --package worktree --test postgres_repository_integration
```

**Expected:** Tests should create fresh schema without conflicts
**Actual:** Repository initialization attempts to create indexes that conflict with PostgreSQL system tables

**Root Cause:** The SQL migration is creating indexes named `idx_worktrees_*` but the error references `pg_class_relname_nsp_index`, suggesting a naming conflict with PostgreSQL's internal schema management.

**Action:** Review repository initialization SQL, ensure proper schema isolation.

---

#### MAJOR-002: Database Isolation Between Tests

**File:** `crates/worktree/tests/postgres_repository_integration.rs`

**Evidence:**
- First test run: 225 rows in database
- Tests failing with `assertion 'left == right' failed` due to leftover data
- `list_all_returns_empty_when_no_worktrees` failing

**Reproduction:**
```bash
# Run tests multiple times without reset
cargo test --package worktree --test postgres_repository_integration
cargo test --package worktree --test postgres_repository_integration  # 2nd run fails
```

**Expected:** Each test should have isolated database state
**Actual:** Tests share database state, causing cascading failures

**Action:** Add proper test teardown/cleanup, or use transaction-based isolation.

---

### MINOR (fix if time)

#### MINOR-001: Test Naming - find_by_name_returns_none_when_not_found

**File:** `crates/worktree/tests/postgres_repository_integration.rs:404`

**Evidence:** Test appears twice in test output (once with unicode, once without).

**Action:** Consolidate duplicate tests or rename for clarity.

---

## Summary of Root Causes

1. **Test Design Errors** (CRITICAL): Tests using `.unwrap()` on expected error paths
2. **PostgreSQL Schema Conflicts** (MAJOR): Migration SQL conflicts with system tables
3. **Database Isolation** (MAJOR): Tests don't properly isolate state
4. **Mathematical Errors** (CRITICAL): Test expectations don't match actual behavior

---

## Recommendations

### Immediate Actions

1. **Fix test design errors:**
   - Replace `.unwrap()` with proper error checking in `error_invalid_path_format`
   - Fix `concurrent_state_updates` expectation to `Suspended`

2. **Add database cleanup:**
   - Each test should clean up after itself
   - Use transactions where possible

3. **Review SQL migrations:**
   - Check index naming conventions
   - Ensure proper schema isolation

### Long-term Actions

1. **Implement test database lifecycle:**
   - Create fresh database per test run
   - Or use in-memory PostgreSQL for testing

2. **Add test fixtures:**
   - Pre-populated test data
   - Cleanup hooks

---

## VERDICT: FAIL

**PostgreSQL integration tests are currently FAILING due to:**
- 67 out of 102 tests failing
- Critical test design errors
- Database isolation issues
- Schema migration conflicts

**Action Required:** Fix the identified issues before merge.
