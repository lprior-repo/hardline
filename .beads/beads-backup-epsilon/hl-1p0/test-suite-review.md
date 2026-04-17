---
bead_id: hl-1p0
reviewer: test-reviewer (Mode 2 — Suite Inquisition)
date: 2026-03-30
tier_reached: 0 (STOPPED — LETHAL count exceeded threshold)
---

## VERDICT: REJECTED

Tier 0 alone produced 28+ LETHAL findings. The suite was not worth the compute for Tiers 1-3.
Per protocol: ≥3 LETHAL findings = send it back without finishing the audit.

---

### Tier 0 — Static Analysis

**[FAIL]** Banned pattern scan — 9 LETHAL hits for `assert!(result.is_err())` / `assert!(result.is_ok())` in core lock tests:

| File | Line | Pattern |
|------|------|---------|
| `tests_basic.rs` | 57 | `assert!(result.is_err())` — test_lock_contention_returns_session_locked |
| `tests_basic.rs` | 88 | `assert!(result.is_err())` — test_unlock_by_non_holder_fails |
| `tests_basic.rs` | 162 | `assert!(result.is_err())` — test_heartbeat_by_non_holder_fails |
| `tests_basic.rs` | 171 | `assert!(result.is_err())` — test_heartbeat_no_lock_fails |
| `tests_session_validation.rs` | 38 | `assert!(result.is_err(), "Should fail...")` — lock_nonexistent_session_returns_not_found_error |
| `tests_session_validation.rs` | 89 | `assert!(result.is_ok(), "Lock should succeed...")` — lock_existing_session_succeeds |
| `tests_session_validation.rs` | 137 | `assert!(result.is_err())` — lock_deleted_session_fails_with_not_found |
| `tests_session_validation.rs` | 165 | `assert!(result.is_err(), "Lock must fail...")` — regression_lock_nonexistent_session_no_longer_creates_orphaned_lock |
| `tests_ttl_regression.rs` | 127 | `assert!(result.is_err())` — regression_lock_with_ttl_fails_fast_before_session_validation |

**[FAIL]** Banned pattern scan — 12 LETHAL hits for `assert!(res.is_ok())` / `assert!(res.is_err())` in CLI lock tests:

| File | Line | Pattern |
|------|------|---------|
| `lock_tests.rs` | 19 | `assert!(res1.is_ok(), ...)` |
| `lock_tests.rs` | 22 | `assert!(res2.is_ok(), ...)` |
| `lock_tests.rs` | 32 | `assert!(res.is_err(), ...)` |
| `lock_tests.rs` | 42 | `assert!(res.is_ok(), ...)` |
| `lock_tests.rs` | 53 | `assert!(res.is_ok(), ...)` |
| `lock_tests.rs` | 64 | `assert!(res.is_ok(), ...)` |
| `lock_tests.rs` | 73 | `assert!(res.is_ok(), ...)` |
| `lock_tests.rs` | 96 | `assert!(res.is_err(), ...)` |
| `lock_tests.rs` | 118 | `assert!(res.is_err(), ...)` |
| `lock_tests.rs` | 128 | `assert!(res.is_ok(), ...)` |
| `lock_tests.rs` | 151 | `assert!(res.is_ok(), ...)` |
| `lock_tests.rs` | 179 | `assert!(res.is_ok(), ...)` |

Every single one of these assertions proves nothing about what error variant was returned, what value was produced, or whether the code path that was hit is the correct one. A function returning `Ok(Default::default())` or `Err(Error::Internal(...))` passes every one of these tests. The tests are hollow.

**[FAIL]** Silent error suppression — 11 instances of `let _ = ` in test code:

| File | Line | Context |
|------|------|---------|
| `tests_basic.rs` | 54 | `let _ = mgr.lock("session-1", "agent-a").await?` |
| `tests_basic.rs` | 73 | `let _ = mgr.lock(...)` |
| `tests_basic.rs` | 85 | `let _ = mgr.lock(...)` |
| `tests_basic.rs` | 104 | `let _ = mgr.lock(...)` |
| `tests_basic.rs` | 117 | `let _ = mgr.lock(...)` |
| `tests_basic.rs` | 118 | `let _ = mgr.lock(...)` |
| `tests_basic.rs` | 133 | `let _ = mgr.lock(...)` |
| `tests_basic.rs` | 160 | `let _ = mgr.lock(...)` |
| `tests_basic.rs` | 179 | `let _ = mgr.lock(...)` |
| `tests_basic.rs` | 189 | `let _ = mgr.lock(...)` |
| `tests_basic.rs` | 229 | `let _ = mgr.lock(...)` |

These are in setup context (pre-seeding a lock before the real test action). The `?` operator propagates the error, so the `let _ =` is a name discard rather than an error discard. Severity reduced from LETHAL to MINOR per Holzmann Rule 5 — but 11 identical instances of silent discard in setup is a pattern that should use a named helper or `assert!(setup_result.is_ok())`.

**[FAIL]** Sleep in tests — 5 hits:

| File | Line | Duration |
|------|------|----------|
| `tests_basic.rs` | 106 | `tokio::time::sleep(Duration::from_millis(10))` |
| `tests_basic.rs` | 135 | `tokio::time::sleep(Duration::from_millis(10))` |
| `tests_basic.rs` | 149 | `tokio::time::sleep(Duration::from_millis(50))` |
| `lock_tests.rs` | 160 | `std::thread::sleep(Duration::from_millis(1100))` |
| `lock_tests.rs` | 176 | `std::thread::sleep(Duration::from_millis(1100))` |

Sleep-based tests are inherently non-deterministic. On a loaded CI machine, 10ms may not be enough for the TTL=0 lock to expire. The 1100ms sleeps in CLI tests are a reliability hazard. Severity: **MAJOR** per Holzmann Rule 2/Rule 5 — test relies on wall-clock timing rather than deterministic state.

**[FAIL]** Test naming violations — 17 hits for `fn test_` prefix in core tests:

| File | Lines |
|------|-------|
| `tests_basic.rs` | 10, 35, 52, 71, 83, 102, 115, 131, 144, 158, 168, 177, 187, 223 |

The naming convention is `test_<behavior>` rather than the BDD `<behavior>` style used in the concurrent and session_validation test files. Severity: **MINOR** (inconsistent but not misleading).

**[FAIL]** Holzmann Rule 2 — Loop in test body:

| File | Line | Code |
|------|------|------|
| `tests_concurrent.rs` | 122 | `for i in 0..10 {` — loop in `stress_test_concurrent_locks_multiple_sessions` |

This loop inserts 10 session rows as setup. It is bounded and in setup, not in the assertion path. Severity: **MINOR** — bounded setup loop, not assertion logic.

**[PASS]** Mock interrogation — no mocks found.

**[PASS]** Integration test purity — `lock_integration.rs` uses `assert_cmd` and `tempfile` only. No `use crate::` paths.

**[FAIL]** Error variant completeness — 7 of 12 `LockErrorKind` variants have ZERO test assertions:

| Variant | Has Exact-Match Test? |
|---------|----------------------|
| `SessionNotFound` | YES — `tests_session_validation.rs:41,138` |
| `SessionLocked` | YES — `tests_concurrent.rs:73`, `tests_ttl_regression.rs:69` |
| `NotLockHolder` | **NO** — no test asserts `LockErrorKind::NotLockHolder` |
| `NotFound` | **NO** — no test asserts `LockErrorKind::NotFound` |
| `DatabaseError` | PARTIAL — `tests_ttl_regression.rs:73` checks `DatabaseError` count is 0, but no test asserts it IS a DatabaseError |
| `ParseError` | **NO** — zero coverage |
| `Unknown` | **NO** — zero coverage |
| `TtlOutOfRange` | **NO** — zero coverage |
| `EmptySessionName` | **NO** — zero coverage |
| `EmptyAgentId` | **NO** — zero coverage |
| `TtlOverflow` | **NO** — zero coverage |
| `SessionNameTooLong` | **NO** — zero coverage |

7 variants with zero test coverage = **7 LETHAL** findings.

The CLI `lock_tests.rs` tests `acquire_with_empty_session_fails`, `acquire_with_empty_agent_fails`, `acquire_with_invalid_ttl_fails`, `acquire_with_too_long_session_fails` — but all use `assert!(res.is_err())`. They prove nothing about WHICH error variant is returned.

**[FAIL]** Density audit:

- Public functions in lock module: 15 (new, with_ttl, pool, init, lock, lock_with_ttl, unlock, heartbeat, get_all_locks, get_lock_audit_log, get_lock_state, verify_session_exists, code, suggestion, exit_code, is_constraint_conflict_error — plus validate_session_name/validate_agent_id/validate_ttl as pub(super))
- Strictly public (`pub fn` / `pub async fn`): 16
- Test count: 21 (core) + 15 (CLI lock_tests) + 4 (lock_integration) = 40 total
- Ratio: 40 / 16 = 2.5x — **BELOW 5x target** = **LETHAL**

**[PASS]** Insta present — flagged for Tier 1 gate (Tier 1 not reached).

---

### Tier 1 — Execution: NOT RUN

Stopped at Tier 0. Suite has 28+ LETHAL findings.

### Tier 2 — Coverage: NOT RUN

### Tier 3 — Mutation: NOT RUN

---

### LETHAL FINDINGS (28)

1. **tests_basic.rs:57** — `assert!(result.is_err())` in `test_lock_contention_returns_session_locked`. This test name claims it verifies `SessionLocked` but the assertion only checks `is_err()`. Any error passes. A function returning `Err(Error::Internal("oops"))` passes this test.

2. **tests_basic.rs:88** — `assert!(result.is_err())` in `test_unlock_by_non_holder_fails`. Claims to verify `NotLockHolder` but asserts only `is_err()`.

3. **tests_basic.rs:162** — `assert!(result.is_err())` in `test_heartbeat_by_non_holder_fails`. Same hollow pattern.

4. **tests_basic.rs:171** — `assert!(result.is_err())` in `test_heartbeat_no_lock_fails`. Same hollow pattern.

5. **tests_session_validation.rs:38** — `assert!(result.is_err(), "Should fail for non-existent session")` in `lock_nonexistent_session_returns_not_found_error`. The name says "not_found_error" but the assertion doesn't check the error variant. This test passes even if the error is `DatabaseError`.

6. **tests_session_validation.rs:89** — `assert!(result.is_ok(), "Lock should succeed for existing session")` in `lock_existing_session_succeeds`. Proves nothing about the returned `LockResponse` values beyond what the subsequent assertions check (which are fine). But the `is_ok()` assertion is redundant and banned.

7. **tests_session_validation.rs:137** — `assert!(result.is_err())` in `lock_deleted_session_fails_with_not_found`. Hollow.

8. **tests_session_validation.rs:165** — `assert!(result.is_err(), "Lock must fail for non-existent session")` in `regression_lock_nonexistent_session_no_longer_creates_orphaned_lock`. Hollow.

9. **tests_ttl_regression.rs:127** — `assert!(result.is_err())` in `regression_lock_with_ttl_fails_fast_before_session_validation`. Hollow.

10. **lock_tests.rs:19,22,32,42,53,64,73,96,118,128,151,179** — 12 instances of `assert!(res.is_ok())` / `assert!(res.is_err())` in CLI lock tests. Every single CLI test that checks for errors uses the hollow `is_err()` pattern. None verify the error variant.

11. **LockErrorKind::NotLockHolder** — No test in the entire suite asserts this exact variant. The implementation produces it in `manager_unlock.rs:38` and `manager_unlock.rs:90`. `tests_basic.rs:88` claims to test "non_holder" but only checks `is_err()`.

12. **LockErrorKind::NotFound** — No test asserts this exact variant. Produced by `manager_unlock.rs:94` when heartbeat is called on a lock that doesn't exist.

13. **LockErrorKind::ParseError** — Zero test coverage. Produced by timestamp parsing failures in `manager_lock.rs:43`, `manager_query.rs:28,32,63`.

14. **LockErrorKind::Unknown** — Zero test coverage. Produced by `manager_lock.rs:79` (timestamp nanos overflow) and `manager_query.rs:73` (unknown operation string).

15. **LockErrorKind::TtlOutOfRange** — Zero test coverage. Produced by `manager.rs:108` when `ttl_seconds > 86400`. CLI test `acquire_with_invalid_ttl_fails` checks `is_err()` only.

16. **LockErrorKind::EmptySessionName** — Zero test coverage. Produced by `manager.rs:82`. CLI test checks `is_err()` only.

17. **LockErrorKind::EmptyAgentId** — Zero test coverage. Produced by `manager.rs:97`. CLI test checks `is_err()` only.

18. **LockErrorKind::TtlOverflow** — Zero test coverage. Produced by `manager.rs:105` when `ttl_seconds == u64::MAX`.

19. **LockErrorKind::SessionNameTooLong** — Zero test coverage. Produced by `manager.rs:86` when `session.len() > 255`. CLI test checks `is_err()` only.

20. **Density ratio 2.5x** — 40 tests / 16 public functions = 2.5x, below the 5x minimum. The suite lacks boundary tests, validation error tests with exact variant matching, and proptest invariants for the `Ttl` value object.

21. **helpers.rs:29** — `assert!(true)` placeholder test. This test literally proves nothing. It was a TODO that was shipped.

22. **tests_ttl_regression.rs:127-131** — `regression_lock_with_ttl_fails_fast_before_session_validation` does `assert!(result.is_err())` then `result.unwrap_err().to_string()` and checks string contents. This is fragile string-matching instead of variant matching. If the error message format changes, the test breaks silently or passes incorrectly.

---

### MAJOR FINDINGS (2)

1. **Sleep-based non-determinism** — 5 tests use `sleep()` to test TTL expiration (`tests_basic.rs:106,135,149`, `lock_tests.rs:160,176`). On slow CI runners, these tests are flaky. The proper approach is to inject a clock or manipulate the database `expires_at` directly.

2. **`let _ =` pattern in setup** — 11 instances in `tests_basic.rs`. While the `?` operator propagates errors, the `let _ =` name discard is misleading. A reader might think the result is being silently discarded.

---

### MINOR FINDINGS (3)

1. **Test naming inconsistency** — `tests_basic.rs` uses `test_<behavior>` naming while `tests_concurrent.rs`, `tests_session_validation.rs`, and `tests_ttl_regression.rs` use descriptive BDD naming without the `test_` prefix.

2. **Bounded loop in setup** — `tests_concurrent.rs:122` has a `for i in 0..10` loop in setup. Bounded, not in assertion path, but still technically a loop in a test body per Holzmann Rule 2.

3. **String-based error assertion** — `tests_basic.rs:62-65` and `tests_basic.rs:93-96` check error messages via `.to_string()` and `msg.contains(...)`. This is fragile — any message wording change breaks the test without any actual behavior change.

---

### MANDATE

Before resubmission, ALL of the following must exist:

#### 1. Replace every `is_err()` / `is_ok()` assertion with exact variant matching

Every test that currently asserts `is_err()` must instead use:
```rust
assert!(matches!(result, Err(Error::Lock(lk)) if matches!(lk.kind(), LockErrorKind::SpecificVariant { .. })));
```

Affected tests (21 instances):
- `tests_basic.rs`: 4 tests (lines 57, 88, 162, 171)
- `tests_session_validation.rs`: 4 tests (lines 38, 89, 137, 165)
- `tests_ttl_regression.rs`: 1 test (line 127)
- `lock_tests.rs`: 12 tests (lines 19, 22, 32, 42, 53, 64, 73, 96, 118, 128, 151, 179)

#### 2. Add exact-variant tests for 7 uncovered `LockErrorKind` variants

Required new tests:
- `lock_unlock_by_non_holder_returns_not_lock_holder` — must assert `LockErrorKind::NotLockHolder { session, agent_id }` with exact field values
- `heartbeat_on_missing_lock_returns_not_found` — must assert `LockErrorKind::NotFound`
- `lock_with_corrupted_timestamp_returns_parse_error` — must assert `LockErrorKind::ParseError`
- `lock_with_overflow_timestamp_returns_unknown` — must assert `LockErrorKind::Unknown`
- `lock_with_ttl_over_86400_returns_ttl_out_of_range` — must assert `LockErrorKind::TtlOutOfRange`
- `lock_with_empty_session_returns_empty_session_name` — must assert `LockErrorKind::EmptySessionName`
- `lock_with_empty_agent_returns_empty_agent_id` — must assert `LockErrorKind::EmptyAgentId`
- `lock_with_ttl_max_u64_returns_ttl_overflow` — must assert `LockErrorKind::TtlOverflow`
- `lock_with_256_char_session_returns_name_too_long` — must assert `LockErrorKind::SessionNameTooLong`

#### 3. Delete the placeholder test in `helpers.rs`

`helpers.rs:26-30` contains `assert!(true)`. Delete it or replace with a real test of `is_constraint_conflict_error`.

#### 4. Fix string-based error assertions

`tests_basic.rs:62-65`, `tests_basic.rs:93-96`, `tests_ttl_regression.rs:128-132` must use variant matching, not `msg.contains(...)`.

#### 5. Resubmission requires full re-run from Tier 0

Per protocol: fixing one thing breaks another. Full re-run. Always.
