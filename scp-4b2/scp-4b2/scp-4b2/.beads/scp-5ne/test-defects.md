# Test Defects

## Contract-Test Parity Violation
- **Severity**: HIGH
- **Location**: contract.md lines 26-32 vs martin-fowler-tests.md
- **Defect**: Error taxonomy defines specific error types (InvalidSessionName, InvalidAgentId, InvalidWorkspaceName, InvalidTaskId, InvalidPath) but violation examples and tests use generic ShellMetacharacter
- **Required Fix**: Align all tests to use specific error types matching the contract, OR update contract to use ShellMetacharacter as the single error type

## Inconsistent Postconditions
- **Severity**: HIGH
- **Location**: contract.md Q1-Q5 vs violation examples
- **Defect**: Postconditions state "returns Ok(()) for valid, Err(ValidationError) for invalid" but doesn't specify which error variant. Violation examples use ShellMetacharacter for all shell metachar inputs
- **Required Fix**: Make postconditions explicit about which error variant is returned for which failure mode

## Incomplete Shell Metachar Coverage for Non-Session Functions
- **Severity**: MEDIUM
- **Location**: martin-fowler-tests.md lines 32-35
- **Defect**: validate_agent_id, validate_workspace_name, validate_task_id, validate_absolute_path only have generic "rejects_shell_metacharacters" tests without enumerating each shell metachar like session_name does
- **Required Fix**: Add individual shell metachar rejection tests for each function, or consolidate with parameterized tests

## Missing P2 Precondition Test
- **Severity**: MEDIUM
- **Location**: contract.md line 11 vs martin-fowler-tests.md
- **Defect**: P2 specifies validate_absolute_path should reject null bytes, but no test exists for this
- **Required Fix**: Add test_validate_absolute_path_rejects_null_byte

## Test Names Not Fully Descriptive
- **Severity**: LOW
- **Location**: martin-fowler-tests.md lines 38-41
- **Defect**: Edge case tests use generic names like "test_validate_handles_unicode_characters" without specifying which validator function
- **Required Fix**: Make test names explicit about which function they're testing
