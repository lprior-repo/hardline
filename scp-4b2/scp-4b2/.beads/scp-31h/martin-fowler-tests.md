# Martin Fowler Test Plan

## Happy Path Tests
- test_agent_id_creates_from_valid_string
- test_workspace_name_creates_from_valid_string
- test_task_id_creates_from_valid_string
- test_absolute_path_creates_from_valid_absolute_path
- test_title_creates_from_valid_string
- test_description_creates_from_valid_string
- test_labels_creates_from_unique_strings
- test_depends_on_creates_from_valid_bead_id
- test_priority_creates_from_valid_value
- test_issue_type_creates_from_valid_type

## Error Path Tests
- test_agent_id_returns_error_for_empty_string
- test_workspace_name_returns_error_for_empty_string
- test_workspace_name_returns_error_for_whitespace_only
- test_workspace_name_returns_error_for_too_long
- test_task_id_returns_error_for_empty_string
- test_absolute_path_returns_error_for_relative_path
- test_absolute_path_returns_error_for_empty_string
- test_title_returns_error_for_empty_string
- test_description_returns_error_for_too_long
- test_labels_returns_error_for_duplicates
- test_labels_returns_error_for_too_many
- test_depends_on_returns_error_for_empty_string
- test_priority_returns_error_for_negative
- test_priority_returns_error_for_value_above_4
- test_issue_type_returns_error_for_invalid_type

## Edge Case Tests
- test_agent_id_handles_unicode_characters
- test_workspace_name_handles_unicode_characters
- test_title_handles_unicode_characters
- test_description_handles_empty_string
- test_labels_handles_empty_vector
- test_labels_handles_single_label
- test_priority_handles_boundary_values
- test_absolute_path_handles_paths_with_special_chars

## Contract Verification Tests
- test_agent_id_as_str_returns_inner_value
- test_agent_id_into_inner_returns_owned_string
- test_workspace_name_as_str_returns_inner_value
- test_workspace_name_into_inner_returns_owned_string
- test_task_id_as_str_returns_inner_value
- test_task_id_into_inner_returns_owned_string
- test_absolute_path_as_str_returns_inner_value
- test_absolute_path_into_inner_returns_owned_string
- test_title_as_str_returns_inner_value
- test_title_into_inner_returns_owned_string
- test_description_as_str_returns_inner_value
- test_description_into_inner_returns_owned_string
- test_labels_as_slice_returns_inner_slice
- test_labels_into_inner_returns_owned_vec
- test_depends_on_as_str_returns_inner_value
- test_depends_on_into_inner_returns_owned_string
- test_priority_as_u8_returns_inner_value
- test_priority_into_inner_returns_owned_u8
- test_issue_type_as_str_returns_inner_value
- test_issue_type_into_inner_returns_owned_string

## Contract Violation Tests
- test_agent_id_empty_violation_returns_invalid_identifier_error
  Given: `AgentId::new("")`
  When: constructor is called with empty string
  Then: returns `Err(SessionError::InvalidIdentifier("AgentId cannot be empty"))`

- test_workspace_name_empty_violation_returns_invalid_identifier_error
  Given: `WorkspaceName::new("")`
  When: constructor is called with empty string
  Then: returns `Err(SessionError::InvalidIdentifier("WorkspaceName cannot be empty"))`

- test_workspace_name_whitespace_violation_returns_invalid_identifier_error
  Given: `WorkspaceName::new("   ")`
  When: constructor is called with whitespace only
  Then: returns `Err(SessionError::InvalidIdentifier("WorkspaceName cannot be empty"))`

- test_workspace_name_too_long_violation_returns_invalid_identifier_error
  Given: `WorkspaceName::new("a".repeat(101))`
  When: constructor is called with string exceeding max length
  Then: returns `Err(SessionError::InvalidIdentifier(...))`

- test_task_id_empty_violation_returns_invalid_identifier_error
  Given: `TaskId::new("")`
  When: constructor is called with empty string
  Then: returns `Err(SessionError::InvalidIdentifier("TaskId cannot be empty"))`

- test_absolute_path_relative_violation_returns_invalid_path_error
  Given: `AbsolutePath::new("relative/path")`
  When: constructor is called with relative path
  Then: returns `Err(SessionError::InvalidPath(...))`

- test_absolute_path_empty_violation_returns_invalid_path_error
  Given: `AbsolutePath::new("")`
  When: constructor is called with empty string
  Then: returns `Err(SessionError::InvalidPath(...))`

- test_title_empty_violation_returns_invalid_identifier_error
  Given: `Title::new("")`
  When: constructor is called with empty string
  Then: returns `Err(SessionError::InvalidIdentifier("Title cannot be empty"))`

- test_description_too_long_violation_returns_invalid_identifier_error
  Given: `Description::new("a".repeat(10001))`
  When: constructor is called with string exceeding max length
  Then: returns `Err(SessionError::InvalidIdentifier(...))`

- test_labels_duplicate_violation_returns_invalid_identifier_error
  Given: `Labels::new(vec!["a".to_string(), "a".to_string()])`
  When: constructor is called with duplicate labels
  Then: returns `Err(SessionError::InvalidIdentifier(...))`

- test_depends_on_empty_violation_returns_invalid_identifier_error
  Given: `DependsOn::new("")`
  When: constructor is called with empty string
  Then: returns `Err(SessionError::InvalidIdentifier(...))`

- test_priority_above_range_violation_returns_invalid_priority_error
  Given: `Priority::new(5)`
  When: constructor is called with value > 4
  Then: returns `Err(SessionError::InvalidPriority(...))`

- test_priority_max_violation_returns_invalid_priority_error
  Given: `Priority::new(255)`
  When: constructor is called with max u8
  Then: returns `Err(SessionError::InvalidPriority(...))`

- test_issue_type_invalid_violation_returns_invalid_issue_type_error
  Given: `IssueType::new("invalid")`
  When: constructor is called with invalid issue type
  Then: returns `Err(SessionError::InvalidIssueType(...))`

## Given-When-Then Scenarios
### Scenario 1: Creating all domain types successfully
Given: Valid input for all domain types
When: Each type's constructor is called with valid input
Then:
- All types return Ok with correct inner value
- All accessors return expected values
- All types implement Display correctly

### Scenario 2: Validation failures for each type
Given: Invalid input for each domain type
When: Each type's constructor is called with invalid input
Then:
- All constructors return Err with appropriate error variant
- Error messages are descriptive

### Scenario 3: Serialization and deserialization
Given: All domain types with valid values
When: Types are serialized and deserialized
Then:
- Serialization produces valid JSON
- Deserialization reconstructs original value
- All traits (Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize) work correctly
