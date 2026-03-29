# Test Plan Review: TOCTOU Race Condition in Directory Creation

```
bead_id: hl-6h1
bead_title: High: TOCTOU race condition in directory creation
phase: state-1.7-retry1
updated_at: 2026-03-29T20:10:00Z
reviewer: test-reviewer (Mode 1 — Plan Inquisition, RE-REVIEW)
```

---

## Previous Review Summary

REJECTED with 6 MAJOR and 5 MINOR findings. This is a re-review round.

---

## Previous Finding Resolution Audit

### MAJOR Findings

| Finding | Status | Verdict |
|---------|--------|---------|
| **M1**: Contract-plan-source error mismatch (spawn_blocking join) | **FIXED** | Source code (`jj_lock.rs:185`) confirmed: `Error::io_error(...)` → `Error::Io(IoErrorKind::IoError(...))`. Test plan B7 matches source. Plan correctly identifies contract.md line 162 as needing correction. Resolution is sound — ground truth is source code. |
| **M2**: Invariant I3 (Atomic Visibility) missing | **FIXED** | B19 added with concrete cross-process filesystem probe pattern using `Command::new` child process. Includes implementation strategy with 5 concrete steps plus alternative single-process fallback. |
| **M3**: B13 no implementation strategy | **FIXED** | Replaced with tracing span capture approach: two `tracing::info!` events from `"test_ordering"` target, captured by test subscriber, ordered assertion `["lock_acquired", "data_dir_created"]`. Concrete, implementable. |
| **M4**: B17 violates Holzmann Rule 2 | **FIXED** | Replaced 24-task iterator map with exactly 3 explicit `tokio::spawn` calls. Added R2 exception justification acknowledging concurrency testing fundamentally requires multiple tasks but the count is a bounded constant. |
| **M5**: 6 banned patterns in existing tests | **FIXED** | Added line-by-line replacement table for 7 patterns (lines 52, 81, 84, 90, 116, 145, 175 in jj_lock_tests.rs and line 186 in jj_operations.rs). Each has a specific replacement assertion. |
| **M6**: 3 mutations survive | **FIXED** | Added B20 (truncate guard), B21 (read flag guard), B22 (backoff sleep guard). Each has concrete implementation strategy with measurable assertions. |

### MINOR Findings

| Finding | Status | Verdict |
|---------|--------|---------|
| **m1**: Summary statistics fabricated | **FIXED** | Corrected to "4 unit / 20 integration / 0 e2e / 4 static" with justification. |
| **m2**: B15/B16 `Err(_)` wildcards | **FIXED** | B15 asserts `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { .. } }))`. B16 asserts `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))`. |
| **m3**: B8 CI feasibility | **FIXED** | Marked `#[ignore]` with documentation for manual execution. Non-strict variant remains CI-feasible. |
| **m4**: `.isolate`-as-file boundary | **FIXED** | Added B23 with concrete scenario. |
| **m5**: Nonexistent repo_root boundary | **FIXED** | Added B24 with concrete scenario including OS error kind assertion. |

**All 11 previous findings resolved.** Proceeding to full 6-axis re-audit.

---

## VERDICT: APPROVED

---

### Axis 1 — Contract Parity

**Public Functions in scope (contract.md + source):**

| Function | Source Location | BDD Scenarios | Covered? |
|----------|----------------|---------------|----------|
| `acquire_cross_process_lock` | jj_lock.rs:131 | B1–B8, B15, B16, B17, B18, B19, B20, B21, B24 (15) | YES |
| `ensure_data_directory` | contract.md:210 (proposed, not yet in source) | B9–B12, B23 (5) | YES |
| `create_workspace_synced` | jj_operations.rs:95 | B13, B14 (2) | YES |
| `acquire_file_lock_with_timeout` | jj_lock.rs:88 | B22 (1) | YES |

[PASS] All 4 public functions have ≥1 BDD scenario. Note: `acquire_file_lock_with_timeout` is `pub fn` (jj_lock.rs:88) — correctly covered by B22.

**Error Variants (contract.md E1–E3 + source verification):**

| Error Variant | Test Plan Scenario | Matches Source? |
|---------------|--------------------|-----------------|
| `Error::Jj(JjError { inner: JjErrorKind::LockTimeout { operation, timeout_ms, retries } })` | B5, B17 | YES — jj_lock.rs:108-113 produces this via `.into()` |
| `Error::Io(IoError { inner: IoErrorKind::IoError("Failed to open workspace lock file: ...") })` | B6, B16, B24 | YES — jj_lock.rs:146 |
| `Error::Io(IoError { inner: IoErrorKind::IoError("Failed to join lock task: ...") })` | B7 | YES — jj_lock.rs:185 |
| `Error::State(StateError { inner: StateErrorKind::ValidationError("LOCK_PORTABILITY_UNSUPPORTED: ...") })` | B8 | YES — jj_lock.rs:176-178 via `Error::validation_error` |
| `Error::Io(IoError { inner: IoErrorKind::IoError("Failed to create data directory: ...") })` | B11, B23 | YES — contract.md:267 |
| `Error::Config(ConfigError { inner: ConfigErrorKind::Invalid("workspace name cannot be empty") })` | B14 | YES — jj_operations.rs:101-103 |

[PASS] All error variants have scenarios asserting exact variants. No `is_err()` assertions in plan.

**Note on contract.md line 162:** Plan correctly identifies this as needing correction. The plan's error variant assertions match the source code, which is the authoritative reference. This is acceptable — the contract doc will be updated as part of the fix.

**Invariants (contract.md I1–I5):**

| Invariant | BDD Scenario | Covered? |
|-----------|--------------|----------|
| I1 Lock-Before-Create | B15 | YES |
| I2 No Phantom Directory | B16 | YES |
| I3 Atomic Visibility | B19 | YES |
| I4 Idempotent | B18 | YES |
| I5 Single-Holder | B17 | YES |

[PASS] All 5 invariants have dedicated BDD scenarios.

---

### Axis 2 — Assertion Sharpness

**Full scan of all 24 "Then:" clauses:**

| Scenario | Assertion Type | Sharp? |
|----------|---------------|--------|
| B1 | `Ok(File)` + `file exists` + `second try_lock fails` | YES — concrete value + side-effect checks |
| B2 | `path.exists() == true` + `old path.exists() == false` + `.isolate.exists() == false` | YES — concrete booleans |
| B3 | `.isolate.exists() == false` | YES — concrete |
| B4 | `try_lock_exclusive()` returns `Ok(())` after drop | YES — concrete |
| B5 | `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { operation, timeout_ms, retries } }))` with field equality checks | YES — exact variant + field values |
| B6 | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` + `msg.contains("Failed to open workspace lock file")` | YES — exact variant + substring |
| B7 | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` + `msg.contains("Failed to join lock task")` | YES — exact variant + substring |
| B8 | `Err(Error::State(StateError { inner: StateErrorKind::ValidationError(msg) }))` + `msg.contains("LOCK_PORTABILITY_UNSUPPORTED")` | YES — exact variant + substring |
| B9 | `Ok(())` + `.is_dir() == true` | YES — concrete |
| B10 | `Ok(())` + existence unchanged | YES — concrete |
| B11 | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` + `msg.contains("Failed to create data directory")` | YES — exact variant + substring |
| B12 | `Ok(())` + lock file `.exists() == false` | YES — concrete |
| B13 | Event list `["lock_acquired", "data_dir_created"]` | YES — exact ordered list |
| B14 | `Err(Error::Config(ConfigError { inner: ConfigErrorKind::Invalid(msg) }))` + `msg == "workspace name cannot be empty"` | YES — exact variant + exact equality |
| B15 | `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { .. } }))` + `.isolate.exists() == false` | YES — exact variant + concrete state |
| B16 | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` where `msg.contains("Failed to open workspace lock file")` + no partial dirs | YES — exact variant + substring + filesystem state |
| B17 | `max(in_critical_section) == 1` + every task is `Ok(File)` or `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { .. } }))` | YES — exact value + exact variant set |
| B18 | All three acquisitions return `Ok(File)` + one lock file exists | YES — concrete count + state |
| B19 | `probe_child` exits code 0 | YES — concrete exit code |
| B20 | `read_to_string == "LOCK-STATE-MARKER"` + file size 18 bytes | YES — exact value + exact size |
| B21 | `file.try_clone()` returns `Ok(cloned)` + `read_to_string` returns `Ok(_)` | YES — concrete success |
| B22 | elapsed time `>= 60ms` | YES — concrete bound |
| B23 | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` + `msg.contains("Failed to create data directory")` | YES — exact variant + substring |
| B24 | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` + `msg.contains("Failed to open workspace lock file")` + OS error kind `NotFound` | YES — exact variant + substring + OS kind |

[PASS] Zero `is_ok()` assertions. Zero `is_err()` assertions. Zero vague `> 0` or `Some(_)` patterns. Every "Then:" specifies a concrete value or exact variant with field checks.

---

### Axis 3 — Trophy Allocation

**Density audit:**

- Public functions in scope: 4 (`acquire_cross_process_lock`, `ensure_data_directory`, `create_workspace_synced`, `acquire_file_lock_with_timeout`)
- Total planned test scenarios: 24 BDD + 3 proptest + 2 Kani + 4 static + 3 constants validation = **36**
- Ratio: 36 / 4 = **9.0x** (target ≥ 5x)

[PASS] Density: 36 tests / 4 functions = 9.0x ≥ 5x

**Trophy layer verification:**

| Layer | Count | Scenarios |
|-------|-------|-----------|
| Unit | 4 | B7, B14, lock constants, backoff arithmetic |
| Integration | 20 | B1–B6, B8–B13, B15–B24 |
| E2E | 0 | No CLI surface at this layer |
| Static | 4 | 5 clippy/code checks |
| Proptest | 3 | P1, P2, P3 |
| Kani | 2 | K1, K2 |

Plan summary (test-plan.md:27): "4 unit / 20 integration / 0 e2e / 4 static". **Matches actual allocation.** Previous m1 resolved.

[PASS] Summary statistics consistent with plan content.

**Pure function / proptest coverage:**

- Backoff arithmetic (pure) → P1, P2, K1 ✓
- Lock path construction (pure) → P3, K2 ✓
- No parsers/deserializers → no fuzz target needed ✓

[PASS] Pure functions have proptest + Kani coverage.
[PASS] No fuzz surface (justified: filesystem I/O only, no untrusted string parsing).

---

### Axis 4 — Boundary Completeness

**`acquire_cross_process_lock`:**

| Boundary | Covered? | By |
|----------|----------|----|
| Happy path (writable, uncontested) | YES | B1 |
| Contended (another holder) | YES | B5 |
| Permission denied (read-only dir) | YES | B6 |
| Nonexistent repo_root | YES | B24 |
| spawn_blocking failure | YES | B7 |
| Strict locks + unsupported FS | YES | B8 |
| Non-strict + unsupported FS | YES | B8 |
| Lock release on drop | YES | B4 |
| Idempotent cycle | YES | B18 |
| Lock file path at repo root | YES | B2 |
| No `.isolate` side effect | YES | B3 |
| Content preserved across cycle | YES | B20 |
| File opened readable | YES | B21 |

**Missing:** `repo_root` is a file, not directory; `PATH_MAX` exceeded. Both are edge cases of the OS `open()` call — low-risk since `OpenOptions::open()` will return `IoError` caught by B6/B24. Not worth dedicated scenarios.

**`ensure_data_directory`:**

| Boundary | Covered? | By |
|----------|----------|----|
| Happy path (doesn't exist) | YES | B9 |
| Already exists (idempotent) | YES | B10 |
| Permission denied | YES | B11 |
| `.isolate` exists as regular file | YES | B23 |
| No lock file side effect | YES | B12 |

**`create_workspace_synced`:**

| Boundary | Covered? | By |
|----------|----------|----|
| Empty name | YES | B14 |
| Correct call order | YES | B13 |

[PASS] All critical boundaries covered. No function has ≥3 missing boundaries.

---

### Axis 5 — Mutation Survivability

**Full mutation table review (12 identified + new):**

| Mutation | Caught By? | Mechanism |
|----------|-----------|-----------|
| Replace lock file constant | B2 | Path assertion fails |
| Remove `ensure_data_directory()` call | B13 | Tracing captures only `"lock_acquired"`, missing `"data_dir_created"` |
| Swap call order | B13 | Tracing captures `"data_dir_created"` before `"lock_acquired"` |
| Reintroduce `create_dir_all(.isolate)` into `acquire` | B3 | `.isolate` exists after lock call |
| `try_lock_exclusive()` always returns `Ok(())` | B5 | Timeout error not returned |
| Remove `drop` release behavior | B4 | Second lock attempt fails |
| Change error message prefix | B11 | Message assertion fails |
| Remove lock portability probe | B8 | ValidationError not returned |
| Set `lock_supported = true` unconditionally | B8 | ValidationError not returned |
| **`truncate(false)` → `truncate(true)`** | **B20** | File content `"LOCK-STATE-MARKER"` wiped — content assertion fails |
| **Remove `read(true)` from OpenOptions** | **B21** | `read_to_string` fails — file not readable |
| **Delete backoff `std::thread::sleep`** | **B22** | Elapsed time < 60ms — timing assertion fails |

**Additional mutation analysis (beyond original 12):**

| Mutation | Caught By? | Mechanism |
|----------|-----------|-----------|
| `create(true)` → `create(false)` | B1 (first call on empty dir) | File doesn't exist, open fails |
| `write(true)` removed from OpenOptions | File lock requires write — `try_lock_exclusive()` fails | B1 would fail |
| Change `FILE_LOCK_BASE_BACKOFF_MS` to 0 | P1 | `0 * 2^i = 0` for all i — proptest catches if assertion is `> 0` |
| Change `HIGH_CONTENTION_MAX_ATTEMPTS` from 8 to 0 | B5 | `retries` field check catches it |
| Remove `unwrap_or(8)` fallback | No test | MINOR — defensive fallback, not a behavior path |
| Change `"workspace creation cross-process lock"` string | B5 | `operation` field equality check catches it |
| `Error::io_error` changed to `Error::Internal` for spawn_blocking | B7 | Variant match would fail |

**Kill rate estimate:** 12/12 identified mutations caught + 6/7 additional mutations caught = 18/19 = **94.7%**. Exceeds 90% threshold.

[PASS] All previously-surviving mutations now have named catching tests.

---

### Axis 6 — Holzmann Plan Audit

| Rule | Status | Evidence |
|------|--------|----------|
| R1: Linear flow | PASS | All 24 scenarios follow Given→When→Then |
| R2: No loops in tests | PASS | B17 uses exactly 3 explicit `tokio::spawn` calls. R2 exception justified: bounded constant concurrency, not dynamic iteration. |
| R3: Resource ownership | PASS | `tempfile::tempdir()` throughout. RAII cleanup. |
| R4: One function, one job | PASS | Each scenario tests one behavior. B15/B16 are invariant-focused but single-behavior. |
| R5: State assumptions | PASS | All scenarios have explicit `Given:` blocks with concrete preconditions. |
| R6: Never swallow errors | PASS | M5 fix addresses all 7 banned patterns. Stress test (B17) now uses explicit variant match instead of `if guard.is_err() { return; }`. |
| R7: No shared mutable state | PASS | `Arc<AtomicUsize>` is shared but immutable (atomic operations are the concurrency primitive being tested). `LazyLock<Mutex>` in source is a one-time init with no subsequent mutation of the `LazyLock` itself. |
| R8: Surface side effects | PASS | Filesystem operations explicitly named. Child process spawning in B19 explicitly described. |

[PASS] All Holzmann rules satisfied.

---

### Existing Test Update Plan Audit

The "Existing Tests to Update" section (test-plan.md:681–703) now includes:

**Path reference updates (3 tests):**

| Test | Line | Change |
|------|------|--------|
| `given_lock_constants_when_validated_then_reasonable_values` | 27 | `"workspace-create.lock"` → `".scp-workspace-create.lock"` |
| `regression_cross_process_lock_blocks_second_holder` | 103–105 | `.join(".isolate").join(WORKSPACE_CREATION_LOCK_FILE)` → `.join(WORKSPACE_CREATION_LOCK_FILE)` |
| `regression_cross_process_lock_releases_on_drop` | 132–134 | Same as above |

**Banned pattern fixes (7 patterns across 2 files):**

| Line | Test | Pattern | Replacement |
|------|------|---------|-------------|
| 52 | `given_file_lock_on_available_file_when_acquired_then_succeeds` | `assert!(result.is_ok())` | `assert_eq!(result, Ok(()))` |
| 81 | `given_file_already_locked_when_timeout_acquisition_then_returns_error` | `assert!(result.is_err())` | Delete — match block on 83–91 is sufficient |
| 84 | Same test | `Error::LockTimeout { ... }` (wrong variant) | `Error::Jj(crate::error_jj::JjError { inner: crate::error_jj::JjErrorKind::LockTimeout { ... } })` |
| 90 | Same test | `_ => panic!("Expected LockTimeout error")` | `other => panic!("Expected Error::Jj(...), got: {other:?}")` |
| 116 | `regression_cross_process_lock_blocks_second_holder` | `assert!(second_lock_attempt.is_err())` | `let Err(e) = ... else { panic!(...) };` |
| 145 | `regression_cross_process_lock_releases_on_drop` | `assert!(second_lock_attempt.is_ok(), ...)` | `assert_eq!(second_lock_attempt, Ok(()), ...)` |
| 175 | `stress_cross_process_lock_keeps_single_holder` | `if guard.is_err() { return; }` | Explicit variant match with panic on unexpected error |
| 186 | `test_empty_workspace_name_returns_error` (jj_operations.rs) | `assert!(result.is_err())` | Delete — match block on 188–192 is sufficient |

[PASS] All 8 existing test modifications are specified with exact line numbers and exact replacement assertions.

---

## LETHAL FINDINGS

None.

---

## MAJOR FINDINGS (0)

None. All 6 previous MAJOR findings have been resolved with concrete, implementable solutions.

---

## MINOR FINDINGS (2/5 threshold)

### m1: B19 child process test complexity

**test-plan.md:393–416** (B19) specifies a child process probe pattern. The "Alternative (if child binary is too complex)" fallback uses a single-process tokio task polling at 1ms intervals with `tokio::time::sleep`. This fallback introduces `sleep` in the test body (Holzmann-adjacent — not a loop, but time-dependent). The primary approach (child process) is clean. The fallback should be documented as a last resort only.

**Severity:** MINOR. Does not affect plan correctness. Implementation can choose the primary approach.

---

### m2: B5 timeout_ms computation is implementation-dependent

**test-plan.md:174** asserts `timeout_ms` equals `(0..8).map(|i| 25 * 2^i).sum()`. The source code (jj_lock.rs:105-107) computes this with a `map().sum()` inside the error path. If the implementation changes to compute timeout differently (e.g., wall-clock elapsed time), the assertion would break. However, this is actually a feature — the test pins the exact computation, preventing silent changes to timeout semantics.

**Severity:** MINOR. Test is correctly strict. Noted for awareness during implementation.

---

## MANDATE

**None.** The test plan is approved for implementation.

**Implementation order recommendations:**

1. Implement `ensure_data_directory` function first (new code, simplest)
2. Relocate lock file path in `acquire_cross_process_lock`
3. Remove `create_dir_all(.isolate)` from `acquire_cross_process_lock`
4. Add `ensure_data_directory()` call in `create_workspace_synced`
5. Add tracing spans for B13
6. Update existing tests (path changes + banned pattern fixes)
7. Implement new BDD scenarios B1–B24
8. Implement proptest P1–P3 and Kani K1–K2
9. Fix contract.md line 162

**Post-implementation:** Submit for Mode 2 (Suite Inquisition) — full Tier 0–3 pipeline.
