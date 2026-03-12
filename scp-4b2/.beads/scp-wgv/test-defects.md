# Test Defects - Bead scp-wgv

## STATUS: REJECTED

## Defects Found

### Critical Issues

1. **Missing ConfigError definition**
   - Location: contract.md lines 97, 112, 135 (references to `ConfigError`)
   - Issue: The contract references `ConfigError::InvalidTimeout`, `ConfigError::InvalidBaseDelay`, etc., but this error enum is never defined in the Error Taxonomy section
   - Required Fix: Add ConfigError enum to the Error Taxonomy

2. **Missing Deadline handling**
   - Location: contract.md lines 11, 40 (mentions "Deadline")
   - Issue: The contract mentions "Deadline: Absolute time by which an operation must complete" as a domain term but there's no `Deadline` struct, no `DeadlineExceeded` error variant, and no implementation
   - Required Fix: Add Deadline struct and DeadlineExceeded error variant

3. **Syntax error in contract**
   - Location: contract.md line 118
   - Issue: ````derive(Debug, Clone, Copy, PartialEq, Eq)]` should be `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
   - Required Fix: Fix the typo

### Missing Test Coverage

4. **Missing test for `calculate_delay` with max cap**
   - Location: martin-fowler-tests.md - test_max_delay_cap_applies exists but test_postcondition_max_delay_capped does not
   - Issue: While test_max_delay_cap_applies covers the scenario, there's no direct contract verification test
   - Required Fix: Add `test_postcondition_exponential_backoff_capped_by_max_delay`

5. **Missing test for `record_success` behavior**
   - Location: martin-fowler-tests.md
   - Issue: Circuit breaker has `record_success` method but no test verifies failure_count resets to 0
   - Required Fix: Add test case for record_success clearing failure count

6. **Missing test for `can_execute` in HalfOpen state**
   - Location: martin-fowler-tests.md
   - Issue: We test Open state rejecting calls (Q5) but not HalfOpen state allowing execution
   - Required Fix: Add test verifying can_execute() returns true in HalfOpen state

7. **Missing test for Deadline**
   - Location: martin-fowler-tests.md
   - Issue: Scenario 4 mentions Deadline but there's no corresponding test case
   - Required Fix: Add test for DeadlineExceeded error when global deadline passes

### Violation Test Parity Issues

8. **Violation tests incomplete**
   - Location: martin-fowler-tests.md lines 121-169
   - Issue: Contract has VIOLATES P6 (zero recovery timeout) but no corresponding test
   - Required Fix: Add test_violation_p6_zero_recovery_timeout

9. **Missing postcondition test for Q2**
   - Location: martin-fowler-tests.md
   - Issue: Postcondition Q2 (retries exhausted returns error) has no dedicated violation test
   - Required Fix: Add test_violation_q2_retries_exhausted_returns_error

10. **Missing postcondition test for Q4**
    - Location: martin-fowler-tests.md
    - Issue: Postcondition Q4 (HalfOpen after timeout) has no dedicated violation test
    - Required Fix: Add test_violation_q4_half_open_after_recovery_timeout

---

## Summary

The test plan has good structure (Given-When-Then, BDD naming) but suffers from:
- Incomplete contract definition (missing ConfigError, Deadline)
- Missing violation test parity with contract
- Missing edge case coverage for circuit breaker HalfOpen state
- Syntax error that would prevent compilation
