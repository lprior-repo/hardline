# Martin Fowler Test Plan

## Happy Path Tests
- test_validate_session_name_returns_ok_for_valid_name
- test_validate_agent_id_returns_ok_for_valid_id
- test_validate_workspace_name_returns_ok_for_valid_name
- test_validate_task_id_returns_ok_for_valid_id
- test_validate_absolute_path_returns_ok_for_valid_path

## Error Path Tests
- test_validate_session_name_returns_error_for_empty_input
- test_validate_agent_id_returns_error_for_empty_input
- test_validate_workspace_name_returns_error_for_empty_input
- test_validate_task_id_returns_error_for_empty_input
- test_validate_absolute_path_returns_error_for_empty_input

## Error Path Tests
- test_session_name_handles_single_character_valid_input
- test_session_name_handles_unicode_characters
- test_session_name_handles_very_long_input
- test_session_name_allows_hyphen_underscore_alphanumeric
- test_agent_id_handles_single_character_valid_input
- test_agent_id_handles_unicode_characters
- test_workspace_name_handles_single_character_valid_input
- test_task_id_handles_single_character_valid_input
- test_absolute_path_handles_single_character_valid_input

## Null Byte Tests (P3 Precondition)
- test_session_name_rejects_null_byte
- test_agent_id_rejects_null_byte
- test_workspace_name_rejects_null_byte
- test_task_id_rejects_null_byte
- test_absolute_path_rejects_null_byte

## Individual Shell Metachar Tests Per Function
### Session Name Shell Metachar Tests
- test_validate_session_name_rejects_ampersand
- test_validate_session_name_rejects_semicolon
- test_validate_session_name_rejects_dollar_sign
- test_validate_session_name_rejects_hash
- test_validate_session_name_rejects_asterisk
- test_validate_session_name_rejects_parentheses
- test_validate_session_name_rejects_pipe
- test_validate_session_name_rejects_greater_less_than
- test_validate_session_name_rejects_brackets
- test_validate_session_name_rejects_braces
- test_validate_session_name_rejects_single_quote
- test_validate_session_name_rejects_double_quote
- test_validate_session_name_rejects_backtick
- test_validate_session_name_rejects_newline
### Agent ID Shell Metachar Tests
- test_validate_agent_id_rejects_ampersand
- test_validate_agent_id_rejects_semicolon
- test_validate_agent_id_rejects_dollar_sign
- test_validate_agent_id_rejects_pipe
- test_validate_agent_id_rejects_backtick
### Workspace Name Shell Metachar Tests
- test_validate_workspace_name_rejects_ampersand
- test_validate_workspace_name_rejects_semicolon
- test_validate_workspace_name_rejects_dollar_sign
- test_validate_workspace_name_rejects_pipe
- test_validate_workspace_name_rejects_backtick
### Task ID Shell Metachar Tests
- test_validate_task_id_rejects_ampersand
- test_validate_task_id_rejects_semicolon
- test_validate_task_id_rejects_dollar_sign
- test_validate_task_id_rejects_pipe
- test_validate_task_id_rejects_backtick
### Absolute Path Shell Metachar Tests
- test_validate_absolute_path_rejects_ampersand
- test_validate_absolute_path_rejects_semicolon
- test_validate_absolute_path_rejects_dollar_sign
- test_validate_absolute_path_rejects_pipe
- test_validate_absolute_path_rejects_backtick

## Contract Verification Tests
- test_precondition_empty_input_returns_error
- test_postcondition_shell_metachar_returns_error
- test_invariant_pure_function_no_side_effects

## Contract Violation Tests
- test_empty_session_name_violation_returns_empty_input_error
  Given: validate_session_name("")
  When: function is called with empty string
  Then: returns Err(ValidationError::EmptyInput) -- NOT a panic

- test_empty_agent_id_violation_returns_empty_input_error
  Given: validate_agent_id("")
  When: function is called with empty string
  Then: returns Err(ValidationError::EmptyInput) -- NOT a panic

- test_empty_workspace_name_violation_returns_empty_input_error
  Given: validate_workspace_name("")
  When: function is called with empty string
  Then: returns Err(ValidationError::EmptyInput) -- NOT a panic

- test_empty_task_id_violation_returns_empty_input_error
  Given: validate_task_id("")
  When: function is called with empty string
  Then: returns Err(ValidationError::EmptyInput) -- NOT a panic

- test_empty_path_violation_returns_empty_input_error
  Given: validate_absolute_path("")
  When: function is called with empty string
  Then: returns Err(ValidationError::EmptyInput) -- NOT a panic

- test_null_byte_in_session_name_violation_returns_shell_metachar_error
  Given: validate_session_name("foo\0bar")
  When: function is called with string containing null byte
  Then: returns Err(ValidationError::ShellMetacharacter) -- NOT a panic

- test_null_byte_in_path_violation_returns_shell_metachar_error
  Given: validate_absolute_path("/path\0/invalid")
  When: function is called with string containing null byte
  Then: returns Err(ValidationError::ShellMetacharacter) -- NOT a panic

- test_session_name_with_ampersand_violation_returns_shell_metachar_error
  Given: validate_session_name("foo&bar")
  When: function is called with string containing &
  Then: returns Err(ValidationError::ShellMetacharacter) -- NOT a panic

- test_agent_id_with_dollar_violation_returns_shell_metachar_error
  Given: validate_agent_id("agent$test")
  When: function is called with string containing $
  Then: returns Err(ValidationError::ShellMetacharacter) -- NOT a panic

- test_workspace_name_with_pipe_violation_returns_shell_metachar_error
  Given: validate_workspace_name("work|space")
  When: function is called with string containing |
  Then: returns Err(ValidationError::ShellMetacharacter) -- NOT a panic

- test_task_id_with_semicolon_violation_returns_shell_metachar_error
  Given: validate_task_id("task;cmd")
  When: function is called with string containing ;
  Then: returns Err(ValidationError::ShellMetacharacter) -- NOT a panic

- test_path_with_backtick_violation_returns_shell_metachar_error
  Given: validate_absolute_path("/path/with`backtick`")
  When: function is called with string containing `
  Then: returns Err(ValidationError::ShellMetacharacter) -- NOT a panic

## Given-When-Then Scenarios
### Scenario 1: Validate clean session name
Given: a valid session name with alphanumeric characters, hyphens, and underscores
When: validate_session_name is called
Then: returns Ok(())

### Scenario 2: Reject session name with shell metacharacter
Given: a session name containing the character "&"
When: validate_session_name is called
Then: returns Err(ValidationError::ShellMetacharacter)

### Scenario 3: Reject empty input
Given: an empty string
When: any validation function is called
Then: returns Err(ValidationError::EmptyInput)

### Scenario 4: Validate absolute path format
Given: a valid absolute path starting with "/"
When: validate_absolute_path is called
Then: returns Ok(()) if no shell metacharacters present
