# Test Review Defects

## STATUS: PARTIALLY FIXED

**Review Date**: 2026-03-11
**Bead**: scp-4b2
**Reviewer**: Test Reviewer Agent

---

## Defects Status

### Defect 1: Missing P6 Violation Test (Critical) - FIXED ✓
- **Type**: Contract Violation Test Parity
- **Location**: `.beads/scp-4b2/martin-fowler-tests.md`
- **Resolution**: Added `test_p6_violation_empty_checkpoint_path_returns_validation_error`

### Defect 2: Missing Error Variants in Error Taxonomy (High) - PENDING IMPLEMENTATION
- **Type**: Contract/Implementation Mismatch
- **Location**: `crates/core/src/error.rs`
- **Description**: The contract specifies error variants that don't exist in the codebase:
  - `WaitTimeout` (error code 55) - used in Q2 postcondition
  - `BatchRollbackFailed` (error code 57) - used in Q5 postcondition
  - `CheckpointError` (error code 58) - referenced in error taxonomy
  - `InvalidWaitMode` - used in P2 violation
- **Resolution**: Will be added during IMPLEMENTATION phase in State 3

### Defect 3: Missing E2E/Integration Tests (Medium) - PENDING
- **Type**: Testing Trophy Gap
- **Location**: `.beads/scp-4b2/martin-fowler-tests.md`
- **Description**: Test plan lacks explicit tests that execute the actual CLI binary end-to-end.
- **Resolution**: Can be added as integration tests in implementation phase

---

## Summary

| Priority | Defect | Status |
|----------|--------|--------|
| Critical | Missing P6 violation test | FIXED ✓ |
| High | Missing error variants | DEFERRED to Implementation |
| Medium | Missing E2E tests | DEFERRED to Implementation |

**Note**: Critical defect is fixed. High/Medium defects are implementation concerns and will be addressed during State 3 (Implementation).
