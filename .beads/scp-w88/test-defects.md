# Test Defects - scp-w88

## STATUS: REJECTED

The test plan violates multiple testing doctrines, particularly the Testing Trophy philosophy.

---

## CRITICAL DEFECTS

### 1. Testing Trophy Violation - No Real Execution (CRITICAL)
**Location**: Entire martin-fowler-tests.md
**Issue**: All tests only verify `cargo check` passes - they verify compilation, NOT runtime functionality
**Doctrine Violated**: Testing Trophy demands "tremendous amounts of Integration and E2E tests that validate the actual system works"

The scenarios crate provides:
- `Scenario::from_yaml()` - YAML parsing
- `ScenarioRunner::run()` - scenario execution  
- `Sanitizer::sanitize_result()` - information barrier sanitization
- `FeedbackLevel` enum with 5 levels controlling sanitization

**NONE of this runtime behavior is tested** - only that the code compiles.

---

### 2. Stub Test - test_invariant_i3_sanitizer_unchanged
**Location**: martin-fowler-tests.md lines 100-103
**Issue**: Only verifies FeedbackLevel enum variants exist, doesn't test sanitization behavior
```rust
// Current test (stub):
Given: Compiled sanitizer module
When: Checking FeedbackLevel enum variants  
Then: Level1 through Level5 all present
```

**Missing**: Actual runtime tests:
```rust
// Should test:
Given: Sanitizer with FeedbackLevel::Level1
When: sanitize_result called with full error details
Then: Returns only "pass" or "fail"

Given: Sanitizer with FeedbackLevel::Level5
When: sanitize_result called 
Then: Returns full unredacted output
```

---

### 3. No Integration Tests
**Issue**: Contract postcondition [I2] states "All existing tests continue to pass after changes" but test plan doesn't run existing tests
**Missing**: Integration tests that:
- Import scenarios crate into a test binary
- Run an actual scenario end-to-end
- Verify HTTP steps execute against a mock/test server
- Verify sanitized feedback at each level

---

### 4. No E2E Tests for Information Barrier
**Issue**: The core feature of this crate is the information barrier (sanitizer)
**Missing**: E2E tests that:
1. Create a scenario with multiple steps
2. Run it through ScenarioRunner
3. Verify sanitized output at each FeedbackLevel
4. Confirm Level1 strips ALL details while Level5 shows everything

---

### 5. No Tests for Scenario Parsing
**Issue**: The crate parses scenario YAML
**Missing**: Tests that:
- Parse valid YAML into Scenario struct
- Validate step structure
- Handle malformed YAML gracefully

---

## MINOR DEFECTS

### 6. Test Names Reference Implementation Files
**Location**: martin-fowler-tests.md lines 17-20
```rust
test_all_scenarios_modules_are_valid_rust
  Given: All source files (lib.rs, runner.rs, scenario.rs, sanitizer.rs)
```
**Issue**: Couples test to specific file names - brittle if refactored

---

### 7. No Property-Based Testing
**Opportunity**: Could test sanitizer with generated inputs at different levels

---

## REQUIRED FIXES

To pass review, the test plan must add:

1. **Integration Test**: Import scenarios crate, run a simple scenario with mocked HTTP
2. **E2E Tests for Sanitizer**: Test `sanitize_result` at all 5 FeedbackLevels
3. **E2E Tests for Scenario Parsing**: Test YAML parsing with valid/invalid inputs
4. **Run Existing Tests**: Execute `cargo test` to satisfy invariant [I2]
5. **Contract Test**: Verify crate works when imported by another crate

---

## EVIDENCE

The scenarios crate (`/home/lewis/src/scp/crates/scenarios/src/sanitizer.rs`) contains:
- `Sanitizer::new()` - constructor
- `Sanitizer::sanitize_result()` - 397 lines of sanitization logic
- `FeedbackLevel` methods: `exposes_error_type()`, `exposes_stack_trace()`, etc.

The test plan never calls any of these - only verifies they compile.
