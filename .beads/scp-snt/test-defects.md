# Test Defects - scp-snt

## Summary
The martin-fowler-tests.md test plan (v2.0) has **addressed 5 of 6 previous defects** but still contains the Multiple Assertions defect.

---

## Previous Defects Status

| Defect | Previous Status | Current Status |
|--------|-----------------|----------------|
| 1. Missing DSL | ❌ FAIL | ✅ FIXED - Gherkin DSL added (PART 1) |
| 2. Over-Mocking | ❌ FAIL | ✅ FIXED - Real HashMap storage used |
| 3. Multiple Assertions | ❌ FAIL | ❌ **NOT FIXED** - Still present |
| 4. Non-Executable | ❌ FAIL | ✅ FIXED - Valid Rust code provided |
| 5. Missing Advanced Paradigms | ❌ FAIL | ✅ FIXED - proptest tests added |
| 6. No Test Classification | ❌ FAIL | ✅ FIXED - Classification table added |

---

## NEW Defects Found

### 1. Multiple Assertions Per Test (Kent Beck TDD Violation) - HIGH

**Location**: martin-fowler-tests.md

**Issue**: Tests violate Kent Beck's principle of ONE logical assertion per test. Multiple tests contain 2-10+ assertions:

| Test Function | Lines | Assertion Count | Violation |
|---------------|-------|------------------|-----------|
| `test_load_session_by_id_returns_ok` | 217-219 | 2 | ❌ |
| `test_load_session_by_name_returns_ok` | 237-238 | 2 | ❌ |
| `test_list_sorted_by_name_returns_ordered` | 399-401 | 3 | ❌ |
| `test_get_current_returns_session_when_set` | 450-451 | 2 | ❌ |
| `test_e2e_full_session_lifecycle` | 936-966 | 10+ | ❌ |

**Example - Line 217-219**:
```rust
// TWO assertions - violates Kent Beck
assert!(result.is_ok());
assert_eq!(result.unwrap().id.as_str(), "session-001");
```

**Fix Required**: Split each test into separate test functions, each with ONE assertion:

```rust
// AFTER FIX - Split into TWO tests:
#[test]
fn test_load_session_by_id_returns_ok() {
    // Given/When
    let result = repo.load(&id);
    // Then - ONE assertion
    assert!(result.is_ok());
}

#[test]
fn test_load_session_by_id_returns_correct_id() {
    // Given/When
    let result = repo.load(&id).unwrap();
    // Then - ONE assertion  
    assert_eq!(result.id.as_str(), "session-001");
}
```

---

## Doctrine Compliance Matrix

| Doctrine | Requirement | Status |
|----------|-------------|--------|
| Dan North BDD | Expressive GWT naming | ✅ PASS |
| Dan North BDD | Behavior over state | ✅ PASS |
| Dave Farley ATDD | DSL separation | ✅ PASS |
| Dave Farley ATDD | WHAT vs HOW separation | ✅ PASS |
| Kent Beck TDD | One assertion per test | ❌ **FAIL** |
| Kent Beck TDD | Isolated, fast, deterministic | ⚠️ PARTIAL |
| Testing Trophy | Real execution | ✅ PASS |
| Testing Trophy | Integration/E2E focus | ✅ PASS |
| Testing Trophy | Minimal mocks | ✅ PASS |
| Advanced Paradigms | Property-based testing | ✅ PASS |
| Advanced Paradigms | Fuzzing | ✅ PASS |

---

## Recommendation

**REJECT** - Fix the Multiple Assertions defect before approval. Each test function must have exactly ONE assertion for proper TDD isolation.
