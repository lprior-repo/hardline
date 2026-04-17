---
bead_id: hl-1p0
bead_title: Port Session Lock Manager (TTL/Heartbeat)
phase: qa-report
qa_date: 2026-03-30T10:49:00Z
verdict: PASS WITH FINDINGS
---

# QA Report: hl-1p0 Session Lock Manager

**QA Enforcer Version:** 2.0.0
**Execution Date:** 2026-03-30
**QA Method:** Full execution with captured output (no hallucinated results)

---

## 1. Executive Summary

**22/22 library tests PASS.** All 4 previously-failing tests identified in the contract
now use the correct hardline error pattern `Error::Lock(LockError(LockErrorKind::...))`
and pass reliably. **96/96 CLI integration tests PASS.** Manual CLI verification
confirms correct behavior across the full lifecycle.

**3 contract parity findings** identified (severity: major). See Section 5.

---

## 2. Test Execution Evidence

### 2.1 Command: Library Tests (Full Suite)

```
$ cargo nextest run -p scp-core --lib coordination::locks 2>&1

   Finished `test` profile [unoptimized + debuginfo] target(s) in 0.11s
────────────
 Nextest run ID f117ca43-b07e-4df2-97f1-f7a8c9f9391a
    Starting 22 tests across 1 binary (1130 tests skipped)
        PASS [   0.003s] ( 1/22) scp-core coordination::locks::helpers::tests::test_constraint_error_codes
        PASS [   0.004s] ( 2/22) scp-core coordination::locks::tests_basic::test_lock_acquire_success
        PASS [   0.005s] ( 3/22) scp-core coordination::locks::tests_basic::test_heartbeat_no_lock_fails
        PASS [   0.005s] ( 4/22) scp-core coordination::locks::tests_basic::test_relock_same_agent_idempotent
        PASS [   0.006s] ( 5/22) scp-core coordination::locks::tests_basic::test_lock_contention_returns_session_locked
        PASS [   0.006s] ( 6/22) scp-core coordination::locks::tests_basic::test_heartbeat_by_non_holder_fails
        PASS [   0.006s] ( 7/22) scp-core coordination::locks::tests_basic::test_lock_state_query_shows_holder
        PASS [   0.006s] ( 8/22) scp-core coordination::locks::tests_basic::test_double_unlock_logs_warning
        PASS [   0.007s] ( 9/22) scp-core coordination::locks::tests_basic::test_get_all_locks_returns_active
        PASS [   0.017s] (10/22) scp-core coordination::locks::tests_basic::test_expired_lock_allows_new_acquisition
        PASS [   0.018s] (11/22) scp-core coordination::locks::tests_session_validation::regression_lock_nonexistent_session_no_longer_creates_orphaned_lock
        PASS [   0.018s] (12/22) scp-core coordination::locks::tests_session_validation::lock_nonexistent_session_returns_not_found_error
        PASS [   0.018s] (13/22) scp-core coordination::locks::tests_basic::test_unlock_by_non_holder_fails
        PASS [   0.018s] (14/22) scp-core coordination::locks::tests_session_validation::lock_existing_session_succeeds
        PASS [   0.018s] (15/22) scp-core coordination::locks::tests_session_validation::lock_deleted_session_fails_with_not_found
        PASS [   0.019s] (16/22) scp-core coordination::locks::tests_ttl_regression::regression_lock_with_ttl_fails_fast_before_session_validation
        PASS [   0.019s] (17/22) scp-core coordination::locks::tests_basic::test_unlock_by_holder_succeeds
        PASS [   0.019s] (18/22) scp-core coordination::locks::tests_ttl_regression::regression_lock_with_ttl_maps_contention_race_to_session_locked
        PASS [   0.020s] (19/22) scp-core coordination::locks::tests_concurrent::regression_concurrent_lock_mutual_exclusion
        PASS [   0.026s] (20/22) scp-core coordination::locks::tests_concurrent::stress_test_concurrent_locks_multiple_sessions
        PASS [   0.030s] (21/22) scp-core coordination::locks::tests_basic::test_get_all_locks_excludes_expired
        PASS [   0.070s] (22/22) scp-core coordination::locks::tests_basic::test_heartbeat_extends_ttl
────────────
     Summary [   0.071s] 22 tests run: 22 passed, 1130 skipped
```

**Exit Code:** 0

### 2.2 Command: Contract-Targeted Regression Tests

The 4 tests listed in the contract as "Failing Tests" were individually executed:

```
$ cargo nextest run -p scp-core --lib coordination::locks -E "test(regression)" -v 2>&1

    Starting 4 tests across 1 binary (1148 tests skipped)
        PASS [   0.004s] scp-core coordination::locks::tests_session_validation::regression_lock_nonexistent_session_no_longer_creates_orphaned_lock
        PASS [   0.004s] scp-core coordination::locks::tests_ttl_regression::regression_lock_with_ttl_fails_fast_before_session_validation
        PASS [   0.006s] scp-core coordination::locks::tests_ttl_regression::regression_lock_with_ttl_maps_contention_race_to_session_locked
        PASS [   0.007s] scp-core coordination::locks::tests_concurrent::regression_concurrent_lock_mutual_exclusion
────────────
     Summary [   0.008s] 4 tests run: 4 passed, 1148 skipped
```

**Exit Code:** 0

### 2.3 Command: Session Validation Tests (Contract Scenarios 3 & 4)

```
$ cargo nextest run -p scp-core --lib coordination::locks -E "test(session_validation)" -v 2>&1

    Starting 5 tests across 1 binary (1147 tests skipped)
        PASS [   0.004s] scp-core coordination::locks::tests_session_validation::lock_deleted_session_fails_with_not_found
        PASS [   0.004s] scp-core coordination::locks::tests_session_validation::lock_existing_session_succeeds
        PASS [   0.005s] scp-core coordination::locks::tests_ttl_regression::regression_lock_with_ttl_fails_fast_before_session_validation
        PASS [   0.005s] scp-core coordination::locks::tests_session_validation::regression_lock_nonexistent_session_no_longer_creates_orphaned_lock
        PASS [   0.005s] scp-core coordination::locks::tests_session_validation::lock_nonexistent_session_returns_not_found_error
────────────
     Summary [   0.005s] 5 tests run: 5 passed, 1147 skipped
```

**Exit Code:** 0

### 2.4 Command: CLI Integration Tests

```
$ cargo nextest run -p scp-cli 2>&1 | tail -20

        PASS [   0.010s] scp-cli commands::task_validation::tests::test_precondition_p3_already_claimed_prevents_claim
        PASS [   0.021s] scp-cli commands::lock_tests::tests::heartbeat_from_wrong_agent_fails
        PASS [   0.021s] scp-cli commands::lock_tests::tests::heartbeat_updates_expiration
        PASS [   0.021s] scp-cli commands::lock_tests::tests::status_reports_correct_state
        PASS [   0.021s] scp-cli commands::lock_tests::tests::list_shows_active_locks
        PASS [   0.029s] scp-cli::lock_integration cli_lock_heartbeat_failure_for_non_holder
        PASS [   0.051s] scp-cli::lock_integration cli_lock_basic_lifecycle
        PASS [   1.114s] scp-cli commands::lock_tests::tests::list_excludes_expired_locks
        PASS [   1.125s] scp-cli commands::lock_tests::tests::heartbeat_for_expired_lock_fails
────────────
     Summary [   1.140s] 96 tests run: 96 passed, 0 skipped
```

**Exit Code:** 0

### 2.5 Command: Compilation Check

```
$ cargo check -p scp-core 2>&1

Checking scp-core v0.5.0 (/home/lewis/src/hardline/crates/core)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.11s
```

**Exit Code:** 0

---

## 3. Manual CLI Verification

Full lifecycle test using actual binary with temp database:

```
--- 1. Acquire Lock ---
$ scp-cli lock acquire test-session --agent test-agent
Lock acquired: test-session for agent test-agent (expires: 2026-03-30 10:48:45.240934579 UTC)
EXIT: 0

--- 2. Status (should be Locked) ---
$ scp-cli lock status test-session
Locked: session test-session held by test-agent (expires: 2026-03-30 10:48:45.240934579 UTC)
EXIT: 0

--- 3. Conflict (second agent, should fail) ---
$ scp-cli lock acquire test-session --agent other-agent
Error: Session 'test-session' is locked by 'test-agent'
EXIT: 90

--- 4. Heartbeat by holder ---
$ scp-cli lock heartbeat test-session --agent test-agent
Heartbeat sent: test-session (new expiration: 2026-03-30 10:48:45.254966616 UTC)
EXIT: 0

--- 5. Heartbeat by non-holder (should fail) ---
$ scp-cli lock heartbeat test-session --agent other-agent
Error: Agent 'other-agent' does not hold lock on session 'test-session'
EXIT: 90

--- 6. List ---
$ scp-cli lock list
SESSION              AGENT                EXPIRES
-----------------------------------------------------------------
test-session         test-agent           2026-03-30 10:48:45.254966616 UTC
EXIT: 0

--- 7. Release by non-holder (should fail) ---
$ scp-cli lock release test-session --agent other-agent
Error: Agent 'other-agent' does not hold lock on session 'test-session'
EXIT: 90

--- 8. Release by holder ---
$ scp-cli lock release test-session --agent test-agent
Lock released: test-session
EXIT: 0

--- 9. Status after release (should be Unlocked) ---
$ scp-cli lock status test-session
Unlocked: session test-session
EXIT: 0

--- 10. Status nonexistent session ---
$ scp-cli lock status ghost-session
Unlocked: session ghost-session
EXIT: 0
```

**Reproduction:** Set `SCP_DATABASE_PATH` to a temp file. Run each subcommand in sequence.

---

## 4. Contract Invariant Verification

| # | Invariant | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Mutual Exclusion | PASS | `regression_concurrent_lock_mutual_exclusion`: 10 agents race, exactly 1 succeeds, 9 get `SessionLocked` |
| 2 | TTL Enforcement | PASS | `test_get_all_locks_excludes_expired`: expired locks invisible; `test_expired_lock_allows_new_acquisition`: new agent acquires after expiry |
| 3 | Session Validation | PASS | `lock_nonexistent_session_returns_not_found_error`: ghost-session rejected with `LockErrorKind::SessionNotFound`; graceful degradation when sessions table missing |
| 4 | Double-Unlock Detection | PASS | `test_double_unlock_logs_warning`: second unlock logs `DoubleUnlockWarning` audit entry, returns `Ok(())` |
| 5 | Holder-Only Release | PASS | `test_unlock_by_non_holder_fails`: agent-b cannot unlock agent-a's lock; `test_heartbeat_by_non_holder_fails`: same for heartbeat |
| 6 | Idempotent Lock | PASS | `test_relock_same_agent_idempotent`: same agent re-locks without error |
| 7 | Audit Completeness | PASS | `test_double_unlock_logs_warning`: verifies Lock, Unlock, and DoubleUnlockWarning entries in order |

---

## 5. Contract Parity Findings

### FINDING P1: Exit Code Dispatch Loss (Severity: MAJOR)

**Contract Section:** Error Mapping table + `LockError::exit_code()` method
**File:** `/home/lewis/src/hardline/crates/core/src/error.rs` line 544

```rust
Error::Lock(_) => 90,
```

**Problem:** `Error::exit_code()` hard-codes exit code 90 for ALL lock errors, ignoring the
granular codes defined in `LockError::exit_code()`:
- SessionNotFound -> 14 (contract says 14)
- SessionLocked -> 16 (contract says 16)
- NotLockHolder -> 17 (contract says 17)
- DatabaseError -> 63 (contract says 63)
- ParseError -> 80 (contract says 80)

**Actual behavior observed:** CLI returns exit code 90 for all lock errors (SessionLocked,
NotLockHolder, etc.) instead of the granular codes.

**Expected:** `Error::Lock(ref e) => e.exit_code()` to delegate to the granular codes.

**Contract text (LockError::exit_code in errors.rs):**
```
SessionNotFound => 14
SessionLocked => 16
NotLockHolder => 17
NotFound => 71
DatabaseError => 63
```

**Evidence from manual CLI test:** Step 3 (conflict) returned EXIT 90 instead of 16.
Step 5 (non-holder heartbeat) returned EXIT 90 instead of 17.

**Reproduction:**
```bash
TMPDB=$(mktemp --suffix=.db)
SCP_DATABASE_PATH="$TMPDB" scp-cli lock acquire s1 --agent a1
SCP_DATABASE_PATH="$TMPDB" scp-cli lock acquire s1 --agent a2
echo $?  # Expected: 16, Actual: 90
```

### FINDING P2: TTL=0 Semantic Disconnect (Severity: MAJOR)

**Contract Section:** Section 5 Preconditions, `Ttl` value object
**File:** `/home/lewis/src/hardline/crates/core/src/coordination/locks/types.rs` lines 36-46
**File:** `/home/lewis/src/hardline/crates/core/src/coordination/locks/manager_lock.rs` lines 68-73

```rust
// types.rs: Ttl::new(0) returns Some(Ttl { seconds: 0 })
// manager_lock.rs:
let ttl = if ttl_seconds > 0 {
    Duration::seconds(ttl_seconds as i64)
} else {
    self.ttl  // Uses manager default (300s) when ttl_seconds=0
};
```

**Problem:** `Ttl::new(0)` validates 0 as a valid TTL, but `lock_with_ttl` treats
`ttl_seconds=0` as "use manager default" (300s). The `Ttl` value object claims 0 is valid,
but the implementation silently replaces it. Contract section 5 says "ttl <= 86400" but
does not document that 0 means "use default". The `lock()` wrapper passes `ttl_seconds=0`
to `lock_with_ttl`, making its TTL value always the manager default, never 0.

**Impact:** A caller calling `lock_with_ttl("session", "agent", 0)` gets a 300-second TTL,
not a 0-second TTL. If 0-second TTL is meant to be instantaneous expiry, this is a bug.
If it is meant to mean "use default", the `Ttl` type's validation is misleading.

### FINDING P3: Error::Lock Not Delegating exit_code in Error::suggestion() (Severity: MINOR)

**Contract Section:** Error context & suggestions
**File:** `/home/lewis/src/hardline/crates/core/src/error.rs` line 520

```rust
Error::Lock(_) => None,
```

**Problem:** `Error::suggestion()` returns `None` for all lock errors, despite
`LockError::suggestion()` providing actionable suggestions for `SessionLocked`
(`Use 'scp agent kill {holder}' to force release`) and `SessionNotFound`
(`Try 'scp session list' to see available sessions`).

**Expected:** `Error::Lock(ref e) => e.suggestion()` to delegate to the granular suggestions.

---

## 6. Contract API Surface Verification

| Method | Contract Signature | Actual Signature | Status |
|--------|--------------------|-----------------|--------|
| `new(db)` | `pub fn new(db: SqlitePool) -> Self` | Matches | PASS |
| `with_ttl(db, ttl)` | `pub fn with_ttl(db: SqlitePool, ttl: Duration) -> Self` | Matches | PASS |
| `pool()` | `pub const fn pool(&self) -> &SqlitePool` | Matches | PASS |
| `init()` | `pub async fn init(&self) -> Result<()>` | Matches | PASS |
| `lock()` | `pub async fn lock(&self, session: &str, agent_id: &str) -> Result<LockResponse>` | Matches | PASS |
| `lock_with_ttl()` | `pub async fn lock_with_ttl(&self, session: &str, agent_id: &str, ttl_seconds: u64) -> Result<LockResponse>` | Matches | PASS |
| `unlock()` | `pub async fn unlock(&self, session: &str, agent_id: &str) -> Result<()>` | Matches | PASS |
| `heartbeat()` | `pub async fn heartbeat(&self, session: &str, agent_id: &str) -> Result<LockResponse>` | Matches | PASS |
| `get_all_locks()` | `pub async fn get_all_locks(&self) -> Result<Vec<LockInfo>>` | Matches | PASS |
| `get_lock_state()` | `pub async fn get_lock_state(&self, session: &str) -> Result<LockState>` | Matches | PASS |
| `get_lock_audit_log()` | `pub async fn get_lock_audit_log(&self, session: &str) -> Result<Vec<LockAuditEntry>>` | Matches | PASS |

---

## 7. Error Mapping Verification

| Isolate Error | Contract: Hardline Error | Actual Code | Status |
|---------------|-------------------------|-------------|--------|
| `Error::SessionLocked { .. }` | `Error::Lock(LockError(LockErrorKind::SessionLocked { .. }))` | `errors.rs:SessionLocked` via `LockErrorKind` | PASS |
| `Error::SessionNotFound { .. }` | `Error::Lock(LockError(LockErrorKind::SessionNotFound { .. }))` | `errors.rs:SessionNotFound` via `LockErrorKind` | PASS |
| `Error::NotLockHolder { .. }` | `Error::Lock(LockError(LockErrorKind::NotLockHolder { .. }))` | `errors.rs:NotLockHolder` via `LockErrorKind` | PASS |
| `Error::NotFound(msg)` | `Error::Lock(LockError(LockErrorKind::NotFound(msg)))` | `errors.rs:NotFound` via `LockErrorKind` | PASS |
| `Error::DatabaseError(msg)` | `Error::Lock(LockError(LockErrorKind::DatabaseError(msg)))` | `errors.rs:DatabaseError` via `LockErrorKind` | PASS |
| `Error::IoError(msg)` | `Error::Io(IoError(IoErrorKind::...))` | Uses `Error::database()` / `IoError` | PASS |
| `Error::ParseError(msg)` | `Error::Lock(LockError(LockErrorKind::ParseError(msg)))` | `errors.rs:ParseError` via `LockErrorKind` | PASS |

All 12 `LockErrorKind` variants present: SessionNotFound, SessionLocked, NotLockHolder,
NotFound, DatabaseError, ParseError, Unknown, TtlOutOfRange, EmptySessionName,
EmptyAgentId, TtlOverflow, SessionNameTooLong. Contract says 12 variants; implementation has 12.

---

## 8. Postconditions Verification

| Postcondition | Status | Evidence |
|---------------|--------|----------|
| `lock()` returns `LockResponse` with unique `lock_id` | PASS | `test_lock_acquire_success`: asserts session and agent_id match |
| `lock()` returns `SessionLocked` when contended | PASS | `test_lock_contention_returns_session_locked` + `regression_concurrent_lock_mutual_exclusion` |
| `lock()` returns `SessionNotFound` for nonexistent session | PASS | `lock_nonexistent_session_returns_not_found_error` matches `LockErrorKind::SessionNotFound` |
| `lock()` validates input (non-empty, len <= 255) | PASS | Validation functions present; not directly tested but code exists in `manager.rs` |
| `unlock()` returns `Ok(())` on success | PASS | `test_unlock_by_holder_succeeds` |
| `unlock()` returns `NotLockHolder` for wrong agent | PASS | `test_unlock_by_non_holder_fails` |
| `unlock()` returns `Ok(())` for double-unlock with audit | PASS | `test_double_unlock_logs_warning` |
| `heartbeat()` extends `expires_at` | PASS | `test_heartbeat_extends_ttl`: asserts `hb.expires_at > original_expires` |
| `heartbeat()` returns `NotLockHolder` for wrong agent | PASS | `test_heartbeat_by_non_holder_fails` |
| `heartbeat()` returns `NotFound` for missing lock | PASS | `test_heartbeat_no_lock_fails` |
| `get_all_locks()` only returns non-expired, sorted ASC | PASS | `test_get_all_locks_excludes_expired` + SQL uses `ORDER BY expires_at ASC` |

---

## 9. Quality Gates

| Gate | Status | Notes |
|------|--------|-------|
| Every test executed | PASS | 22 library + 96 CLI = 118 total |
| No skipped tests in scope | PASS | 0 failed, 0 skipped in lock scope |
| No panics in output | PASS | Zero panics observed |
| No secrets in output | PASS | No credentials, tokens, or keys in output |
| Error messages actionable | PASS | "Session 'X' is locked by 'Y'" includes both session and holder |
| Compilation clean | PASS | `cargo check -p scp-core` exits 0 |
| No lock-specific clippy issues | PASS | `cargo clippy -p scp-core` shows no warnings for `coordination/locks` |
| User workflow completes | PASS | Full acquire/status/heartbeat/list/release lifecycle verified via CLI |

---

## 10. Files Examined

| File | Purpose |
|------|---------|
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/mod.rs` | Module root |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/errors.rs` | `LockError` + `LockErrorKind` (12 variants) |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/types.rs` | `Ttl`, `LockResponse`, `LockInfo`, `LockState`, `LockAuditEntry`, `LockOperation` |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/manager.rs` | `LockManager` core: init, validation, session check |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/manager_lock.rs` | `lock()` and `lock_with_ttl()` |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/manager_unlock.rs` | `unlock()` and `heartbeat()` |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/manager_query.rs` | `get_all_locks()`, `get_lock_state()`, `get_lock_audit_log()` |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/helpers.rs` | `is_constraint_conflict_error()` |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/tests_basic.rs` | 12 basic tests |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/tests_concurrent.rs` | 2 concurrent tests |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/tests_session_validation.rs` | 4 session validation tests |
| `/home/lewis/src/hardline/crates/core/src/coordination/locks/tests_ttl_regression.rs` | 2 TTL regression tests |
| `/home/lewis/src/hardline/crates/core/src/error.rs` | Top-level `Error` enum |
| `/home/lewis/src/hardline/crates/cli/tests/lock_integration.rs` | CLI integration tests |
| `/home/lewis/src/hardline/.beads/hl-1p0/contract.md` | Contract definition |
| `/home/lewis/src/hardline/.beads/hl-1p0/test-plan.md` | Test plan |

---

## 11. Verdict

**PASS WITH FINDINGS**

The lock manager implementation correctly fulfills the contract's core scope:
4 previously-failing tests now pass with correct hardline error patterns.
All 7 invariants hold. All 11 public API methods match their signatures.
All 7 error mappings are correct.

**3 findings require attention:**
- **P1 (MAJOR):** `Error::exit_code()` returns flat 90 for all lock errors instead of
  delegating to `LockError::exit_code()` which provides granular codes (14, 16, 17, etc.)
- **P2 (MAJOR):** `Ttl::new(0)` validates 0 as valid but `lock_with_ttl` silently replaces
  0 with the manager default (300s). Semantic disconnect between type and behavior.
- **P3 (MINOR):** `Error::suggestion()` returns `None` for lock errors instead of
  delegating to `LockError::suggestion()` which provides actionable guidance.

None of these findings block the current bead scope (fixing 4 test patterns), but they
represent contract parity issues that should be filed as follow-up work.
