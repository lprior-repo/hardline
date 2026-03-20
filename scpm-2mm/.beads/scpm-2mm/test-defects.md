# Test Plan Defects

## Defect Summary
- **Total Defects**: 5 critical issues
- **Contract-Tests Alignment**: FLAWED → FIXED
- **BDD Fidelity**: PARTIAL → FIXED
- **ATDD Completeness**: INCOMPLETE → FIXED

---

## Defect 1: Backoff Formula Inconsistency (Critical) — ✅ RESOLVED

**Location**: `contract.md:14` vs `martin-fowler-tests.md:38-40`

**Issue**: 
- Contract says: `delay = base_delay * factor^attempt` (exponential without +1)
- Test expects: [100, 200, 400, 800] for attempts [0, 1, 2, 3] with base=100, factor=2.0
- **BUT** the test comment says "each delay > previous" which conflates monotonicity with correctness

**Missing Verification**: No test verifies the actual formula against the stated equation. Tests only verify monotonic increase.

**BDD Violation**: Given-When-Then scenarios should describe behavior, not implementation details. "delay before retry 2 is ~200ms" leaks implementation.

**Resolution**:
- Added explicit `test_exponential_backoff_formula_verification` that explicitly verifies `delay = base_delay * factor^attempt` for all 4 attempts with exact expected values
- Contract formula is now explicitly tested, not just monotonicity

---

## Defect 2: Scenario 10 Timeout Semantics are Contradictory (Critical) — ✅ RESOLVED

**Location**: `martin-fowler-tests.md:334-341`

**Issue**:
- Test says: "Each attempt has its own timeout check" AND "If total time exceeds timeout, returns TimeoutError"
- These are **mutually exclusive** interpretations:
  1. Per-attempt timeout: Each retry gets its own fresh 200ms window
  2. Total timeout: All attempts combined capped at 200ms
- Contract Q1 states: "total time MUST NOT exceed its configured timeout" — confirms total time interpretation
- Test description contradicts the contract it purports to verify

**ATDD Violation**: Test is describing implementation ("each attempt has its own timeout") rather than the accepted behavior from the contract.

**Resolution**:
- Rewrote Scenario 10 to clarify "TOTAL timeout" semantics
- Added explicit note referencing Contract Q1
- Removed contradictory "each attempt has its own timeout" language

---

## Defect 3: P5 Precondition Missing from Contract (Medium) — ✅ RESOLVED

**Location**: `contract.md:21-26` vs `contract.md:149`

**Issue**:
- Type encoding table (line 149) references P5: "factor > 1.0" as runtime-checked
- Preconditions section (lines 21-26) only lists P1-P4
- This creates ambiguity: Is P5 a precondition or not?

**Contract-Test Gap**: Tests reference P5 implicitly via violation test, but contract doesn't formalize it.

**Resolution**:
- Added P5 to formal Preconditions section: "A retry policy `factor` value MUST be strictly greater than 1.0 (exponential backoff multiplier)"
- Added violation examples for P5 in Violation Examples section (factor=1.0 and factor=0.5)
- Type encoding table now correctly references existing P5

---

## Defect 4: HalfOpen→Open Transition Not Covered (Medium) — ✅ RESOLVED

**Location**: `martin-fowler-tests.md` - Missing scenario

**Issue**:
- Test scenarios cover:
  - Closed→Open (after failures) ✓
  - Open→HalfOpen (after open_duration) ✓
  - HalfOpen→Closed (after success threshold) ✓
- **Missing**: HalfOpen→Open (when failure occurs during probe)
- Circuit breaker invariant I1 requires "state accurately reflects failure rate" — this transition is part of that behavior

**Testing Trophy Violation**: Missing integration-level scenario combining state machine transitions.

**Resolution**:
- Added Scenario 8b: "Circuit breaker returns to open from half-open after probe failure"
- Explicitly covers HalfOpen→Open transition
- Documents that failure_count resets and open_duration timer restarts
- Completes the HalfOpen→Open→HalfOpen cycle coverage

---

## Defect 5: Timeout-During-Backoff Scenario Missing (Medium) — ✅ RESOLVED

**Location**: `martin-fowler-tests.md` - Missing scenario

**Issue**:
- Scenario 3 describes retry with backoff delays
- Scenario 10 mentions timeout
- **No scenario tests**: What happens when a phase times out WHILE waiting in a backoff delay?
  - Does the timeout timer continue counting during backoff?
  - Does the total timeout include accumulated backoff delays?
- Contract Q1 says "total time" includes "scheduling overhead" but doesn't clarify if backoff delay counts

**Domain Gap**: This is a real edge case that affects the correctness of Q1.

**Resolution**:
- Added Scenario 10b: "Timeout fires during backoff delay"
- Explicitly tests that total elapsed time includes backoff delays
- Verifies timeout fires immediately upon backoff completion before next attempt
- References Contract Q1 to clarify total timeout semantics

---

## Verification Summary

| Defect | Status | Verification Method |
|--------|--------|---------------------|
| 1: Backoff formula verification | ✅ FIXED | Added `test_exponential_backoff_formula_verification` with exact formula values |
| 2: Scenario 10 contradiction | ✅ FIXED | Rewrote to use "TOTAL timeout" terminology, referenced Q1 |
| 3: P5 missing from contract | ✅ FIXED | Added P5 to Preconditions, added violation examples |
| 4: HalfOpen→Open missing | ✅ FIXED | Added Scenario 8b |
| 5: Timeout-during-backoff | ✅ FIXED | Added Scenario 10b |

---

## Verdict

**STATUS**: APPROVED ✅

All 5 defects have been resolved. The test plan now:
- Explicitly verifies the exponential backoff formula
- Uses consistent TOTAL timeout semantics aligned with Q1
- Formally includes P5 as a precondition with violation examples
- Covers the complete circuit breaker state machine including HalfOpen→Open
- Covers the timeout-during-backoff edge case

The contract and tests are now aligned and can serve as valid acceptance criteria.
