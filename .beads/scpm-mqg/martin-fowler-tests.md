# Martin Fowler Test Plan: TaskId Value Object

## Happy Path Tests
- test_parses_valid_taskid_with_numeric_suffix
  Given: "bd-123abc"
  When: TaskId::parse() is called
  Then: returns Ok(TaskId) with correct internal value

- test_parses_valid_taskid_with_alphanumeric_hex
  Given: "bd-deadbeef"
  When: TaskId::parse() is called
  Then: returns Ok(TaskId) with correct internal value

- test_parses_valid_taskid_case_insensitive_hex
  Given: "bd-ABCDEF"
  When: TaskId::parse() is called
  Then: returns Ok(TaskId) with correct internal value

- test_try_from_str_trait_valid_input
  Given: "&str" containing "bd-f00bar"
  When: TryFrom<&str> is invoked
  Then: returns Ok(TaskId)

## Error Path Tests
- test_parse_empty_string_returns_invalid_input_error
  Given: empty string ""
  When: TaskId::parse() is called
  Then: returns Err(Error::InvalidInput)

- test_parse_missing_prefix_returns_invalid_prefix_error
  Given: "abc-123"
  When: TaskId::parse() is called
  Then: returns Err(Error::InvalidPrefix)

- test_parse_invalid_hex_returns_invalid_hex_error
  Given: "bd-xyz"
  When: TaskId::parse() is called
  Then: returns Err(Error::InvalidHex)

- test_parse_empty_suffix_returns_empty_suffix_error
  Given: "bd-"
  When: TaskId::parse() is called
  Then: returns Err(Error::EmptySuffix)

- test_try_from_str_trait_invalid_input
  Given: "&str" containing invalid format
  When: TryFrom<&str> is invoked with invalid input
  Then: returns Err(Error::InvalidPrefix) or appropriate error variant

## Edge Case Tests
- test_single_hex_digit_suffix
  Given: "bd-a"
  When: TaskId::parse() is called
  Then: returns Ok(TaskId) with suffix "a"

- test_maximum_hex_length
  Given: "bd-ffffffffffffffffffffffffffffffff" (32 hex chars)
  When: TaskId::parse() is called
  Then: returns Ok(TaskId)

- test_mixed_case_hex
  Given: "bd-AbCdEf123456"
  When: TaskId::parse() is called
  Then: returns Ok(TaskId)

## Contract Verification Tests
- test_to_string_always_starts_with_bd_prefix
  Given: valid TaskId created from "bd-abc123"
  When: to_string() is called
  Then: result starts with "bd-"

- test_as_str_returns_valid_string_slice
  Given: valid TaskId created from "bd-abc123"
  When: as_str() is called
  Then: returns "bd-abc123"

- test_roundtrip_parse_to_string
  Given: valid TaskId created from "bd-abc123"
  When: to_string() is parsed again
  Then: returns equivalent TaskId

- test_equality_based_on_value
  Given: two TaskId instances parsed from same string "bd-abc123"
  When: they are compared with ==
  Then: they are equal

- test_display_trait_outputs_correct_format
  Given: valid TaskId created from "bd-abc123"
  When: format!("{}", task_id) is called
  Then: returns "bd-abc123"

## Contract Violation Tests
- test_precondition_p1_violation_returns_invalid_input
  Given: empty string
  When: TaskId::parse("") is called
  Then: returns Err(Error::InvalidInput) -- NOT a panic

- test_precondition_p2_violation_returns_invalid_prefix
  Given: "abc-123"
  When: TaskId::parse("abc-123") is called
  Then: returns Err(Error::InvalidPrefix) -- NOT a panic

- test_precondition_p3_violation_returns_invalid_hex
  Given: "bd-xyz"
  When: TaskId::parse("bd-xyz") is called
  Then: returns Err(Error::InvalidHex) -- NOT a panic

- test_precondition_p4_violation_returns_empty_suffix
  Given: "bd-"
  When: TaskId::parse("bd-") is called
  Then: returns Err(Error::EmptySuffix) -- NOT a panic

## Given-When-Then Scenarios

### Scenario 1: Parse Valid TaskId
Given: a string "bd-123abc" that is well-formed
When: TaskId::parse() is called
Then:
- Returns Ok containing a TaskId instance
- The TaskId's internal value equals "bd-123abc"
- TaskId::to_string() returns "bd-123abc"
- TaskId::as_str() returns "bd-123abc"

### Scenario 2: Reject Invalid Prefix
Given: a string "abc-123" missing "bd-" prefix
When: TaskId::parse() is called
Then:
- Returns Err(Error::InvalidPrefix)
- Does NOT panic
- Error message indicates the prefix issue

### Scenario 3: Reject Invalid Hex Characters
Given: a string "bd-xyz" with non-hex suffix
When: TaskId::parse() is called
Then:
- Returns Err(Error::InvalidHex)
- Does NOT panic
- Error message identifies invalid hex characters

### Scenario 4: Reject Empty Suffix
Given: a string "bd-" with no characters after prefix
When: TaskId::parse() is called
Then:
- Returns Err(Error::EmptySuffix)
- Does NOT panic
