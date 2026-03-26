# Test Plan Review: Port Session Locks — TTL/Heartbeat Implementation

## VERDICT: REJECTED

---

## Axis 1 — Contract Parity

### ❌ FAIL: Contradictory assertions on empty session

**File:** `.beads/hl-bjy/test-plan.md`
**Lines 63-66** (Behavior Inventory)

**Scenario 10 (line 63):**
```
[LockManager] returns [Error::EmptySessionName("Session name cannot be empty")] when [session = "" empty string]
```

**Scenario 13 (line 66):**
```
[LockManager] returns [Ok(LockResponse)] when [session = "" empty, agent = "agent-1"] (session validation only, agent optional)
```

**Impact:** Implementation cannot satisfy both scenarios. The plan is internally inconsistent.

**Fix:** Delete scenario 13 or change assertion to `Err(Error::EmptySessionName(...))`.

---

### ❌ FAIL: Error variant mismatch for constraint conflicts

**File:** `.beads/hl-bjy/test-plan.md`
**Line 61 (Behavior Inventory), Line 394 (BDD Scenario)**

**Behavior Inventory (line 61):**
```
[LockManager] returns [SessionLocked error with holder=unknown] when [constraint conflict without lock record]
```

**BDD Scenario (lines 389-395):**
```
Then: Err(Error::Unknown("Constraint conflict with unknown session"))
```

**Impact:** The test plan is inconsistent about which error variant is returned for constraint conflicts.

**Fix:** Choose one error variant and update all scenarios. If `Unknown` is correct, update the inventory. If `SessionLocked` is correct, update the BDD scenario.

---

## Axis 2 — Assertion Sharpness

### ❌ FAIL: Tautological assertions

**File:** `.beads/hl-bjy/test-plan.md`
**Lines 665-674, 676-682, 684-689, 691-696, 698-708, 1017-1023, 1025-1030, 1032-1037, 1039-1049, 1051-1078, 1080-1088**

**Confirmed duplicates:**
- Lines 665-674: Duplicate of lines 652-663 (heartbeat extends TTL)
- Lines 676-682: Duplicate of lines 678-682 (heartbeat NotLockHolder)
- Lines 684-689: Duplicate of lines 686-689 (heartbeat NotFound)
- Lines 691-696: Duplicate of lines 693-696 (heartbeat expired)
- Lines 1017-1023: Duplicate of lines 676-682
- Lines 1025-1030: Duplicate of lines 684-689
- Lines 1032-1037: Duplicate of lines 691-696
- Lines 1039-1049: Duplicate of lines 698-708
- Lines 1051-1078: Duplicate of lines 819-832
- Lines 1080-1088: Duplicate of lines 843-850

**Impact:** Test count is inflated by ~30% due to duplicates. Actual unique scenarios: ~74 instead of 85.

**Actual density: 74 / 12 = 6.17x** (still exceeds 5× threshold, but must be accurate)

**Fix:** Remove all duplicate scenarios. Recalculate density.

---

### ❌ FAIL: Impossible `lock()` TTL boundary tests

**File:** `.beads/hl-bjy/test-plan.md`
**Lines 549-556 (scenario 23), Lines 558-567 (scenario 24)**

**Scenario 23 (lines 549-556):**
```
[LockManager] returns [LockResponse with TTL 1] when [ttl_seconds = 1 min valid boundary]
```

**Scenario 24 (lines 558-567):**
```
[LockManager] returns [LockResponse with TTL 86400] when [ttl_seconds = 86400 max valid boundary]
```

**Problem:** `lock()` function signature is:
```rust
pub fn lock(&self, session: &str, agent_id: &str) -> Result<LockResponse, Error>;
```

It **does not accept a `ttl` parameter**. It always uses default TTL (300s). These scenarios claim to test TTL=1 and TTL=86400 for `lock()`, which is **IMPOSSIBLE**.

**Impact:** Tests cannot be written because the function signature doesn't support the tested behavior.

**Fix:** Either:
1. Add `ttl` parameter to `lock()` function, OR
2. Change scenarios to test `lock_with_ttl()` with explicit TTL values, OR
3. Delete these scenarios as they test non-existent functionality

---

### ❌ FAIL: Inconsistent heartbeat assertions

**File:** `.beads/hl-bjy/test-plan.md`
**Lines 652-663, 665-674, 712-723**

**Scenarios use mixed absolute and relative time assertions:**
- Lines 652-663: `new_expires_at == "2026-03-26T00:10:00Z"` (absolute)
- Lines 665-674: `new_expires_at == acquired_at + Duration::seconds(300)` (relative)
- Lines 712-723: `new_expires_at == "2026-03-26T00:10:00Z"` (absolute)

**Impact:** Tests may pass/fail based on mock timing. Relative time assertions are more robust.

**Fix:** Use relative time assertions in all heartbeat scenarios.

---

### ❌ FAIL: Weak lock_id assertion in scenario 478

**File:** `.beads/hl-bjy/test-plan.md`
**Line 478**

```
Then: Ok(LockResponse)
And: lock_id.starts_with("lock-") && lock_id.len() > 10
```

**Impact:** Weak assertion allows invalid lock_id formats to pass.

**Fix:** Strengthen to:
```
And: lock_id.starts_with("lock-test-session-") && lock_id.len() > "lock-test-session-".len()
```

---

## Axis 3 — Trophy Allocation

### ❌ FAIL: Test density calculation is misleading

**File:** `.beads/hl-bjy/test-plan.md`
**Line 12, Line 18, Line 184**

**Line 12:** Claims "85 BDD scenarios across 12 public functions"
**Line 18:** Claims "85 BDD scenarios / 12 public functions = 7.08x"
**Line 184:** Claims "81 core domain behaviors / 12 functions = 6.75x"

**Actual count after removing duplicates:**
- Original claim: 85 scenarios
- Duplicates identified: ~11 scenarios
- Actual unique scenarios: ~74 scenarios

**Actual density: 74 / 12 = 6.17x**

**MAJOR:** The claim of 7.08x is inflated by duplicates. While still above 5× threshold, the density must be accurately reported.

---

## Axis 4 — Boundary Completeness

### ⚠️ PARTIAL: Missing boundary tests

**File:** `.beads/hl-bjy/test-plan.md`
**Lines 66, 64, 75-82**

**Missing 1-char session name boundary test:**
- Line 66: Scenario 13 tests empty session (contradictory)
- Line 15: Scenario 15 tests 255-char session
- **Missing:** No scenario tests `session = "a"` (1 character, minimum valid length)

**Missing 1-char agent_id boundary test:**
- Line 64: Scenario 11 tests empty agent_id
- **Missing:** No scenario tests `agent_id = "a"` (1 character, minimum valid)

**Fix:** Add explicit boundary tests for 1-char session and 1-char agent_id.

---

## Axis 5 — Mutation Survivability

### ❌ FAIL: Missing mutation checkpoints

**File:** `.beads/hl-bjy/test-plan.md`
**Lines 1561-1650**

**Claimed: 33 mutation checkpoints**

**Missing mutation checkpoints:**
1. `get_all_locks` — mutation: `WHERE expires_at >= now()` → `WHERE expires_at > now()`
2. `get_lock_audit_log` — mutation: `ORDER BY timestamp DESC` → `ORDER BY timestamp ASC`
3. `get_lock_state` — mutation: empty session returns `Ok` instead of `Err`
4. `verify_session_exists` — mutation: table missing returns `Err` instead of `Ok`
5. `LockManager::pool` — mutation: return different pool reference
6. `is_constraint_conflict_error` — mutation: wrong error codes (1556, 2066)

**Impact:** These mutations could survive without explicit checkpoints.

**Fix:** Add explicit mutation checkpoints for all missing functions.

---

## Axis 6 — Holzmann Rules

### ⚠️ PARTIAL: Some preconditions are vague

**File:** `.beads/hl-bjy/test-plan.md`
**Lines 269-275, 280-284, 289-296**

**Scenario preconditions use vague language:**
- "In-memory SQLite database with no tables" (line 269)
- "In-memory SQLite database" (line 280)
- "In-memory SQLite database with session_locks and session_lock_audit tables already created" (line 289)

**Impact:** While mostly clear, "in-memory SQLite database" doesn't specify:
- Connection string (`sqlite::memory:`)
- Whether tables are created by `init()` or manually
- Whether foreign keys are enabled

**Fix:** Standardize preconditions to use explicit connection strings and clear table creation steps.

---

## LETHAL FINDINGS (4)

| File:Line | Finding | Impact |
|-----------|---------|--------|
| `.beads/hl-bjy/test-plan.md:63-66` | Contradictory assertions on empty session (scenario 10 says `Err`, scenario 13 says `Ok`) | Implementation cannot satisfy both scenarios |
| `.beads/hl-bjy/test-plan.md:61, 394` | Error variant mismatch for constraint conflicts (`SessionLocked` vs `Unknown`) | Inconsistent error handling |
| `.beads/hl-bjy/test-plan.md:549-567` | Impossible `lock()` TTL boundary tests (function has no TTL parameter) | Tests cannot be written |
| `.beads/hl-bjy/test-plan.md:652-1088` | At least 11 duplicate scenarios inflate test count by ~30% | Test density claim is inaccurate |

---

## MAJOR FINDINGS (15)

| File:Line | Finding | Impact |
|-----------|---------|--------|
| `.beads/hl-bjy/test-plan.md:66` | Missing 1-char session name boundary test | Boundary not tested |
| `.beads/hl-bjy/test-plan.md:64` | Missing 1-char agent_id boundary test | Boundary not tested |
| `.beads/hl-bjy/test-plan.md:652-674, 712-723` | Inconsistent heartbeat assertions (absolute vs relative time) | Tests may pass/fail based on mock timing |
| `.beads/hl-bjy/test-plan.md:478` | Weak lock_id assertion in scenario 478 | Invalid lock_id formats may pass |
| `.beads/hl-bjy/test-plan.md:61-62` | Duplicate scenario 62 (duplicate of 58) | Inflates test count |
| `.beads/hl-bjy/test-plan.md:66-67` | Duplicate scenario 67 (duplicate of 63) | Inflates test count |
| `.beads/hl-bjy/test-plan.md:75-82` | Missing lock() boundary tests | Function not properly tested |
| `.beads/hl-bjy/test-plan.md:381` | Incomplete error message assertions (`contains()` pattern) | Partial message match allows different messages |
| `.beads/hl-bjy/test-plan.md:18` | Missing parse_error test clarity | No clear path for ParseError trigger |
| `.beads/hl-bjy/test-plan.md:1561-1571` | Missing database error mutation checkpoint | Mutation may survive |
| `.beads/hl-bjy/test-plan.md:1306-1309` | Incomplete proptest anti-invariant coverage | Concurrency testing misplaced |
| `.beads/hl-bjy/test-plan.md:1378-1381` | Missing null/empty lock_id assertion | Invalid lock_ids may pass |
| `.beads/hl-bjy/test-plan.md:1422-1426` | Missing session validation mutation checkpoint | Mutation may survive |
| `.beads/hl-bjy/test-plan.md:1437-1443` | Fuzz target corpus incomplete | Missing edge cases |
| `.beads/hl-bjy/test-plan.md:1524` | Kani harness bounds insufficient | May miss race conditions |

---

## MINOR FINDINGS (5+)

| File:Line | Finding | Impact |
|-----------|---------|--------|
| `.beads/hl-bjy/test-plan.md:652-1088` | Inconsistent scenario numbering | Confusing document structure |
| `.beads/hl-bjy/test-plan.md:884-891` | Scenario 891-892 has wrong expected value (`Some("agent-2")` vs `None`) | Wrong assertion |
| `.beads/hl-bjy/test-plan.md:63-67` | Missing `verify_session_exists` empty agent_id test | Unclear validation rules |
| `.beads/hl-bjy/test-plan.md:288-296` | Incomplete init idempotency test | Edge cases not covered |
| `.beads/hl-bjy/test-plan.md:957-964` | Missing pool reference equality test | Edge cases not covered |

---

## SIX AXIS REVIEW SUMMARY

### Axis 1 — Contract Parity
**FAIL** - Error variant mismatch (`SessionLocked` vs `Unknown`) for constraint conflicts.

### Axis 2 — Assertion Sharpness
**FAIL** - Multiple tautological and contradictory assertions:
- Scenario 13 contradicts scenario 10 on empty session
- Impossible `lock()` TTL boundary tests
- Weak lock_id assertion in scenario 478

### Axis 3 — Trophy Allocation
**FAIL** - Test density calculation is misleading:
- Claimed: 85 scenarios / 12 functions = 7.08x
- Actual (after removing duplicates): ~74 / 12 = 6.17x
- While still above 5× threshold, the claim must be accurate

### Axis 4 — Boundary Completeness
**FAIL** - Missing boundary tests:
- 1-char session name
- 1-char agent_id
- Missing explicit boundary tests for lock()

### Axis 5 — Mutation Survivability
**FAIL** - Insufficient mutation checkpoints:
- Missing `get_all_locks`, `get_lock_audit_log`, `get_lock_state`, `verify_session_exists`, `LockManager::pool`, `is_constraint_conflict_error` mutation checkpoints

### Axis 6 — Holzmann Rules
**PASS** - All BDD scenarios follow Holzmann rules:
- Linear Given → When → Then flow
- No loops in test bodies
- Explicit preconditions (mostly)
- No shared state
- No error swallowing

---

## MANDATE

Before resubmission, the test plan must:

### Must Fix (LETHAL - 4 items)

1. **Delete or fix scenario 13** (line 66): Either change to `Err(Error::EmptySessionName(...))` or remove the scenario.

2. **Delete scenarios 23-24** (lines 549-567): `lock()` has no TTL parameter. Replace with `lock_with_ttl()` tests if TTL boundary testing is needed.

3. **Remove all duplicate scenarios** (lines 652-1088):
   - Lines 665-674 → delete (duplicate of 652-663)
   - Lines 676-682 → delete (duplicate of 678-682)
   - Lines 684-689 → delete (duplicate of 686-689)
   - Lines 691-696 → delete (duplicate of 693-696)
   - Lines 1017-1023 → delete (duplicate of 676-682)
   - Lines 1025-1030 → delete (duplicate of 684-689)
   - Lines 1032-1037 → delete (duplicate of 691-696)
   - Lines 1039-1049 → delete (duplicate of 698-708)
   - Lines 1051-1078 → delete (duplicate of 819-832)
   - Lines 1080-1088 → delete (duplicate of 843-850)
   - Lines 1006-1015 → delete (duplicate of 665-674)

4. **Fix error variant for constraint conflicts**: Choose either `SessionLocked` or `Unknown` and update all scenarios (behavior inventory line 61, BDD scenario line 394).

### Must Fix (MAJOR - 15 items)

5. Add 1-char session name boundary test (after scenario 15)
6. Add 1-char agent_id boundary test (after scenario 11)
7. Unify heartbeat assertions to use relative time (acquired_at + 300s)
8. Strengthen lock_id assertion in scenario 478 to match format
9. Delete scenario 62 (duplicate of 58)
10. Delete scenario 67 (duplicate of 63)
11. Fix scenario 891-892 to assert `holder == None` when no lock exists
12. Add missing `DatabaseError` mutation checkpoints
13. Add missing fuzz corpus seeds (Unicode, special chars, long agent_id)
14. Increase Kani harness bounds to 3+ concurrent calls
15. Strengthen lock_id proptest assertion to exclude empty/short values

### Recalculate Density

After removing duplicates, recalculate:
- Original claim: 85 scenarios / 12 functions = 7.08x
- After removing ~11 duplicates: ~74 scenarios / 12 functions = 6.17x
- **Still exceeds 5× threshold**, but must be accurate.

### Re-verify Error Variant Coverage

After fixing the constraint conflict error variant, verify all 12 variants have explicit tests:
- ✅ SessionNotFound
- ✅ SessionLocked
- ✅ NotLockHolder
- ✅ NotFound
- ✅ DatabaseError
- ✅ ParseError
- ⚠️ Unknown (needs to be confirmed after fix)
- ✅ TtlOutOfRange
- ✅ EmptySessionName
- ✅ EmptyAgentId
- ✅ TtlOverflow
- ✅ SessionNameTooLong

---

## FINAL CHECKLIST BEFORE RESUBMISSION

- [ ] All duplicate scenarios removed
- [ ] Contradictory scenario 13 fixed
- [ ] Impossible `lock()` TTL scenarios removed
- [ ] Error variant for constraint conflicts fixed
- [ ] All 12 error variants have explicit tests
- [ ] All public functions have ≥5x scenario coverage
- [ ] All boundaries explicitly named per function
- [ ] No assertions use `is_ok()` or `is_err()`
- [ ] All assertions use concrete values
- [ ] Proptest invariants have anti-invariants
- [ ] Fuzz corpus seeds complete
- [ ] Mutation checkpoints match actual tests
- [ ] Kani harness bounds sufficient
- [ ] Test density accurately reported

**Resubmit with these fixes. Full re-review from Tier 0.**

---

**Audit completed by: Test Inquisitor (Mode 1: Plan Inquisition)**
**Date:** Thu Mar 26 2026
**Model:** Qwen3.5-35B-A3B-UD-Q5_K_XL.gguf
