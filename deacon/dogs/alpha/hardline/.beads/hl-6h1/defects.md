# Defects Report: TOCTOU Fix Re-Review (hl-6h1)

```
reviewer: black-hat-reviewer
verdict: REJECTED
date: 2026-03-29
review: 2nd pass — post-fix verification
```

## LETHAL FINDINGS — Re-verification

### L1: unwrap_or() in source code — FIXED ✅
**Evidence**: `rg -n 'unwrap_or' jj_lock.rs` → 0 matches. `rg -n 'unwrap_or' jj_operations.rs` → 0 matches.
All 4 instances replaced with `u32::try_from(...).map_err(|_| Error::internal(...))?`.

### L2: Banned assertion is_err() — FIXED ✅
**Evidence**: `rg -n 'is_err()' jj_operations.rs` → 0 matches.
Line 191 now uses `match result { Err(Error::Config(...)) => {}, ... }` — exact variant match.

### L3: Loop in test body — FIXED ✅
**Evidence**: `rg -n 'while ' jj_lock_tests.rs` → only match is in a comment string (line 126).
Poller task (lines 794-822) now uses `tokio::select!` + `tokio::sync::Notify` for bounded event-driven loop. Correct pattern.

### L4: Silent .ok() discards in tests — FIXED ✅
**Evidence**: `rg -n '\.ok()' jj_lock_tests.rs` → only match is in a comment (line 808: `// L4: explicit match instead of .ok()`).
Line 809 now uses `match probe.unlock() { Ok(()) | Err(_) => {} }` — explicit consumption.

### L5: Wrong error variant for spawn_blocking — FIXED ✅
**Evidence**: `rg 'Error::io_error.*join' jj_lock.rs` → 0 matches.
Line 221: `.map_err(|e| Error::internal(format!("Failed to join lock task: {e}")))?` — correct `Error::Internal` variant per contract E1.

### L6: Functions exceeding 25-line limit — NOT FULLY FIXED ❌
**Evidence**: Manual line count of every function in jj_lock.rs:

| Function | Lines | Status |
|---|---|---|
| `calculate_backoff_ms` | 39-44 (6) | ✅ |
| `build_workspace_lock_timeout_error` | 64-71 (8) | ✅ |
| `acquire_lock_with_backoff` | 74-96 (23) | ✅ FIXED (was 50) |
| `build_file_lock_timeout_error` | 99-113 (15) | ✅ |
| **`acquire_file_lock_with_timeout`** | **116-146 (31)** | **❌ STILL 31 LINES** |
| `open_lock_file` | 149-157 (9) | ✅ |
| `verify_lock_support` | 163-179 (17) | ✅ |
| `enforce_strict_locks` | 182-200 (19) | ✅ |
| `acquire_cross_process_lock` | 210-222 (13) | ✅ FIXED (was 50) |
| `ensure_data_directory` | 232-238 (7) | ✅ |

**`acquire_file_lock_with_timeout` is 31 lines. Limit is 25.** The retry loop body (lines 122-139) is 18 lines alone. Extract `try_acquire_with_backoff_iteration` or similar.

**Fix**: Extract the inner `match file.try_lock_exclusive()` block into a helper, or extract the for-loop into a `retry_with_backoff` combinator.

---

## ZERO_UNWRAP_PANIC audit — CLEAN ✅
`rg 'unwrap\(\)|expect\(|panic!\(\)|todo!\(\)|unimplemented!\(\)' jj_lock.rs jj_operations.rs` → 0 matches in source code.

---

## MAJOR FINDINGS — Status unchanged (warnings)

### M1: Raw u64 return type — STILL PRESENT
`calculate_backoff_ms` returns `u64`. Should be `BackoffMs` newtype.

### M2: Stringly-typed description — STILL PRESENT
`description: &str` at line 116. Should be `LockOperation` enum.

### M3: Dead code after exhaustive for loops — STILL PRESENT
- Line 94 in `acquire_lock_with_backoff`: unreachable `Err(...)` after exhaustive `0..5` loop
- Lines 141-145 in `acquire_file_lock_with_timeout`: unreachable after exhaustive `0..8` loop

### M4: test_ prefix naming — STILL PRESENT
`jj_operations.rs:187` `test_empty_workspace_name_returns_error` and `jj_operations.rs:200` `test_workspace_without_parent_returns_error` violate Given-When-Then naming convention.

### M5: Redundant unwrap_err() in test — STILL PRESENT
`jj_lock_tests.rs:262`: `result.unwrap_err().to_string()` — tests are exempt from ZERO_UNWRAP_PANIC but this is still a code smell.

### M6: Blanket #![allow(unused)] — PARTIALLY FIXED
- `jj_lock.rs`: ✅ Removed (was line 9)
- `jj_operations.rs:8`: ❌ Still present

---

## NEW FINDING (this review)

### N1: Test file exceeds 300-line limit
**File**: `jj_lock_tests.rs` — **1338 lines** (4.5x the limit)
**Rule**: `<300 line file limit` from AGENTS.md
**Impact**: MAJOR — 1338 lines is unmaintainable. Split into modules: `jj_lock_tests_unit.rs`, `jj_lock_tests_regression.rs`, `jj_lock_tests_stress.rs`, `jj_lock_tests_proptest.rs`.

---

## VERDICT

```
LETHAL fixed:   5/6 (L1-L5 ✅, L6 ❌)
LETHAL remaining: 1 (L6 — acquire_file_lock_with_timeout is 31 lines)
NEW LETHAL:     0
NEW MAJOR:      1 (N1 — test file 1338 lines)

STATUS: REJECTED
```

**One function still exceeds 25 lines.** The fix is straightforward — extract the retry loop body of `acquire_file_lock_with_timeout` into a helper function. Re-submit after decomposing.
