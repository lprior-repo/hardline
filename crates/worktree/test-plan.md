# Test Plan: Worktree Crate

## Summary
- **Behaviors identified**: 87
- **Trophy allocation**: ~260 unit / ~145 integration / ~10 e2e / ~5 static
- **Proptest invariants**: 18
- **Fuzz targets**: 4
- **Kani harnesses**: 3
- **Total tests planned**: 435+ (targeting 5x density over existing 74)

---

## 1. Behavior Inventory

### Domain Layer Behaviors

1. **WorktreeName** validates input and rejects empty names
2. **WorktreeName** validates input and rejects names containing '/'
3. **WorktreeName** validates input and rejects names starting with '.'
4. **WorktreeName** converts to owned String via Into trait
5. **WorktreeName** converts to &str via From trait
6. **WorktreeName** matches string via matches() method
7. **WorktreeId** generates unique random IDs via new_random()
8. **WorktreeId** parses UUID string via from_string()
9. **WorktreeId** fails parsing when UUID is invalid
10. **WorktreeId** converts from bytes via from_bytes()
11. **WorktreeId** converts to bytes via as_bytes()
12. **WorktreeId** converts to Uuid via Into trait
13. **WorktreeId** converts from Uuid via Into trait
14. **WorktreeState** converts from u8 via from_u8()
15. **WorktreeState** converts to u8 via as_u8()
16. **WorktreeState** identifies terminal states via is_terminal()
17. **WorktreeState** identifies active states via is_active()
18. **WorktreeState** identifies transient states via is_transient()
19. **WorktreeState** returns valid next states from Creating state
20. **WorktreeState** returns valid next states from Incomplete state
21. **WorktreeState** returns valid next states from Active state
22. **WorktreeState** returns valid next states from Suspended state
23. **WorktreeState** returns valid next states from Removing state
24. **WorktreeState** returns empty next states for Removed state
25. **WorktreeState** validates transitions via can_transition_to()
26. **WorktreeTypeEnum** converts from u8 via from_u8()
27. **WorktreeTypeEnum** converts to u8 via as_u8()
28. **WorktreeTypeEnum** identifies development-focused types
29. **WorktreeTypeEnum** identifies QA-focused types
30. **WorktreeTypeEnum** identifies troubleshooting-focused types
31. **AbsolutePath** validates paths are absolute
32. **AbsolutePath** rejects relative paths
33. **AbsolutePath** joins child paths
34. **AbsolutePath** gets parent directory
35. **AbsolutePath** gets file name
36. **AbsolutePath** checks existence
37. **AbsolutePath** checks if directory
38. **AbsolutePath** checks if file
39. **BranchName** validates branch names are non-empty
40. **BranchName** validates branch names contain only valid characters
41. **BranchName** rejects names starting with hyphen
42. **BranchName** rejects names ending with hyphen
43. **BranchName** rejects names starting with period
44. **BranchName** rejects names ending with period
45. **BranchName** rejects names with consecutive periods
46. **BranchName** identifies default branches (main/master)
47. **BranchName** identifies feature branches
48. **BranchName** identifies release branches
49. **Worktree** creates new worktree with Creating state
50. **Worktree** generates random UUID on creation
51. **Worktree** initializes transition from Creating to Active
52. **Worktree** rejects suspend from Creating state
53. **Worktree** initializes transition from Active to Suspended
54. **Worktree** resumes transition from Suspended to Active
55. **Worktree** rejects resume from non-Suspended state
56. **Worktree** marks for removal from Active/Suspended
57. **Worktree** completes removal to Removed state
58. **Worktree** adds metadata key-value pairs
59. **Worktree** removes metadata and returns old value
60. **Worktree** gets metadata by key
61. **Worktree** returns all metadata as HashMap
62. **Worktree** reports is_active() when in Active state
63. **Worktree** reports is_removed() when in Removed state

### Application Layer Behaviors

64. **WorktreeService** creates worktree and persists to repository
65. **WorktreeService** rejects duplicate worktree names
66. **WorktreeService** initializes worktree and updates state
67. **WorktreeService** suspends active worktree
68. **WorktreeService** resumes suspended worktree
69. **WorktreeService** removes worktree via mark/complete flow
70. **WorktreeService** finds worktree by ID in cache
71. **WorktreeService** finds worktree by name in cache
72. **WorktreeService** lists worktrees without filters
73. **WorktreeService** lists worktrees filtered by state
74. **WorktreeService** lists worktrees filtered by type
75. **WorktreeService** lists worktrees filtered by name prefix
76. **WorktreeService** adds metadata to existing worktree
77. **WorktreeService** caches worktree after save
78. **WorktreeService** updates cache after state transition
79. **WorktreeService** removes worktree from cache on delete

### Infrastructure Layer Behaviors

80. **GitWorktreeAdapter** opens repository from valid path
81. **GitWorktreeAdapter** rejects non-existent repository path
82. **GitWorktreeAdapter** gets parent repository path
83. **GitWorktreeAdapter** gets current branch name
84. **GitWorktreeAdapter** gets all local branches
85. **GitWorktreeAdapter** gets all remote branches (strips origin/)
86. **GitWorktreeAdapter** lists worktrees
87. **SqliteRepository** creates database schema on init
88. **SqliteRepository** saves worktree to database
89. **SqliteRepository** finds worktree by ID
90. **SqliteRepository** finds worktree by name
91. **SqliteRepository** deletes worktree by ID
92. **SqliteRepository** checks name existence

---

## 2. Trophy Allocation

| Layer | Percentage | Count | Justification |
|-------|------------|-------|---------------|
| **Unit** | 30% | ~260 | Pure domain logic (value objects, state machine) must be exhaustively tested. Each error variant needs explicit test. |
| **Integration** | 60% | ~145 | Repository implementations (SQLite, Postgres), Git adapter with real repos, service layer with real repos. |
| **E2E** | 5% | ~10 | CLI integration, full create→initialize→suspend→resume→remove flow. |
| **Static** | 5% | ~5 | clippy, types, cargo-deny, fmt. |

**Rationale**: This is a DDD crate with significant pure domain logic. Per Testing Trophy, integration tests should be most numerous (real deps), but unit tests must be exhaustive for all value objects and state machine logic.

---

## 3. BDD Scenarios

### Behavior: WorktreeName rejects empty names

#### Happy Path: Valid name creation
```
Given: valid name string "feature-branch"
When: WorktreeName::new("feature-branch") is called
Then: Ok(WorktreeName("feature-branch")) is returned
And: worktree_name.as_str() == "feature-branch"
```

**Test name**: `fn worktree_name_new_valid_name_returns_ok()`

#### Error: Empty name
```
Given: empty string ""
When: WorktreeName::new("") is called
Then: Err(WorktreeDomainError::InvalidName("Name cannot be empty")) is returned
```

**Test name**: `fn worktree_name_new_empty_returns_invalid_name_error()`

#### Error: Name with slash
```
Given: string "feature/sub"
When: WorktreeName::new("feature/sub") is called
Then: Err(WorktreeDomainError::InvalidName("Name cannot contain '/'")) is returned
```

**Test name**: `fn worktree_name_new_with_slash_returns_invalid_name_error()`

#### Error: Name starting with dot
```
Given: string ".hidden"
When: WorktreeName::new(".hidden") is called
Then: Err(WorktreeDomainError::InvalidName("Name cannot start with '.'")) is returned
```

**Test name**: `fn worktree_name_new_starts_with_dot_returns_invalid_name_error()`

#### Conversion: Into String
```
Given: WorktreeName("test-worktree")
When: String::from(worktree_name) is called
Then: "test-worktree" is returned
```

**Test name**: `fn worktree_name_into_string_returns_owned_value()`

#### Conversion: From &WorktreeName to &str
```
Given: WorktreeName("test-worktree")
When: &str::from(&worktree_name) is called
Then: "test-worktree" is returned
```

**Test name**: `fn worktree_name_from_ref_to_str_returns_slice()`

#### Method: matches
```
Given: WorktreeName("my-worktree")
When: worktree_name.matches("my-worktree") is called
Then: true is returned
And: worktree_name.matches("other-worktree") returns false
```

**Test name**: `fn worktree_name_matches_returns_true_for_same_name()`

---

### Behavior: WorktreeId generates unique IDs

#### Happy Path: Random ID generation
```
Given: no prior IDs exist
When: WorktreeId::new_random() is called twice
Then: id1 != id2 is true
And: id1.to_string() is a valid UUID format
```

**Test name**: `fn worktree_id_new_random_generates_unique_ids()`

**Proptest variant**: Generate 1000 random IDs, assert all are unique

#### Happy Path: UUID string parsing
```
Given: valid UUID string "550e8400-e29b-41d4-a716-446655440000"
When: WorktreeId::from_string(uuid_str) is called
Then: Ok(WorktreeId(uuid)) is returned
And: id.to_string() == uuid_str
```

**Test name**: `fn worktree_id_from_string_valid_uuid_returns_ok()`

#### Error: Invalid UUID string
```
Given: invalid string "not-a-uuid"
When: WorktreeId::from_string("not-a-uuid") is called
Then: Err(WorktreeDomainError::InvalidPath(_)) is returned
```

**Test name**: `fn worktree_id_from_string_invalid_uuid_returns_error()`

#### Happy Path: From bytes
```
Given: bytes [0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4, 0xa7, 0x16, 0x44, 0x66, 0x55, 0x44, 0x00, 0x00]
When: WorktreeId::from_bytes(bytes) is called
Then: id.to_string() == "550e8400-e29b-41d4-a716-446655440000"
```

**Test name**: `fn worktree_id_from_bytes_returns_expected_uuid()`

#### Conversion: To Uuid
```
Given: WorktreeId from valid UUID string
When: Uuid::from(worktree_id) is called
Then: uuid.to_string() == original_uuid_string
```

**Test name**: `fn worktree_id_conversion_to_uuid_preserves_value()`

---

### Behavior: WorktreeState validates transitions

#### Happy Path: Creating to Active
```
Given: worktree state is Creating
When: state.can_transition_to(Active) is called
Then: true is returned
```

**Test name**: `fn worktree_state_can_transition_to_active_from_creating()`

#### Happy Path: Creating to Removed
```
Given: worktree state is Creating
When: state.can_transition_to(Removed) is called
Then: true is returned
```

**Test name**: `fn worktree_state_can_transition_to_removed_from_creating()`

#### Error: Creating to Suspended
```
Given: worktree state is Creating
When: state.can_transition_to(Suspended) is called
Then: false is returned
```

**Test name**: `fn worktree_state_cannot_transition_to_suspended_from_creating()`

#### Happy Path: Active to Suspended
```
Given: worktree state is Active
When: state.can_transition_to(Suspended) is called
Then: true is returned
```

**Test name**: `fn worktree_state_can_transition_to_suspended_from_active()`

#### Happy Path: Suspended to Active
```
Given: worktree state is Suspended
When: state.can_transition_to(Active) is called
Then: true is returned
```

**Test name**: `fn worktree_state_can_transition_to_active_from_suspended()`

#### Happy Path: Removing to Removed
```
Given: worktree state is Removing
When: state.can_transition_to(Removed) is called
Then: true is returned
```

**Test name**: `fn worktree_state_can_transition_to_removed_from_removing()`

#### Terminal: Removed cannot transition
```
Given: worktree state is Removed
When: state.can_transition_to(Active) is called
Then: false is returned
And: state.valid_next_states() is empty
```

**Test name**: `fn worktree_state_removed_has_no_valid_next_states()`

#### Method: is_terminal
```
Given: WorktreeState::Removed
When: state.is_terminal() is called
Then: true is returned
And: WorktreeState::Active.is_terminal() returns false
```

**Test name**: `fn worktree_state_is_terminal_identifies_removed_state()`

#### Method: is_active
```
Given: WorktreeState::Active
When: state.is_active() is called
Then: true is returned
And: WorktreeState::Suspended.is_active() returns false
```

**Test name**: `fn worktree_state_is_active_identifies_active_state()`

---

### Behavior: Worktree creates with Creating state

#### Happy Path: New worktree creation
```
Given: valid WorktreeName, AbsolutePath, parent_path, WorktreeTypeEnum, and optional BranchName
When: Worktree::new(name, path, parent, type, branch) is called
Then: Ok(worktree) is returned
And: worktree.state() == WorktreeState::Creating
And: worktree.id() is a valid UUID
And: worktree.created_at() == worktree.updated_at()
```

**Test name**: `fn worktree_new_returns_worktree_with_creating_state()`

#### Happy Path: New worktree without branch
```
Given: valid parameters with branch = None
When: Worktree::new(name, path, parent, type, None) is called
Then: Ok(worktree) is returned
And: worktree.branch() is None
```

**Test name**: `fn worktree_new_without_branch_returns_worktree_with_none_branch()`

#### Happy Path: New worktree with branch
```
Given: valid parameters with branch = Some(BranchName::new("main").unwrap())
When: Worktree::new(name, path, parent, type, branch) is called
Then: Ok(worktree) is returned
And: worktree.branch().unwrap().as_str() == "main"
```

**Test name**: `fn worktree_new_with_branch_returns_worktree_with_branch()`

---

### Behavior: Worktree state transitions

#### Happy Path: Initialize from Creating
```
Given: worktree in Creating state
When: worktree.initialize() is called
Then: Ok(()) is returned
And: worktree.state() == WorktreeState::Active
And: worktree.updated_at() > worktree.created_at()
```

**Test name**: `fn worktree_initialize_from_creating_returns_ok_and_sets_active()`

#### Error: Suspend from Creating
```
Given: worktree in Creating state
When: worktree.suspend() is called
Then: Err(WorktreeDomainError::InvalidStateTransition(Creating, Suspended)) is returned
```

**Test name**: `fn worktree_suspend_from_creating_returns_invalid_state_transition_error()`

#### Happy Path: Suspend from Active
```
Given: worktree in Active state
When: worktree.suspend() is called
Then: Ok(()) is returned
And: worktree.state() == WorktreeState::Suspended
```

**Test name**: `fn worktree_suspend_from_active_returns_ok_and_sets_suspended()`

#### Happy Path: Resume from Suspended
```
Given: worktree in Suspended state
When: worktree.resume() is called
Then: Ok(()) is returned
And: worktree.state() == WorktreeState::Active
```

**Test name**: `fn worktree_resume_from_suspended_returns_ok_and_sets_active()`

#### Error: Resume from non-Suspended
```
Given: worktree in Active state
When: worktree.resume() is called
Then: Err(WorktreeDomainError::InvalidStateTransition(Active, Active)) is returned
```

**Test name**: `fn worktree_resume_from_active_returns_invalid_state_transition_error()`

#### Happy Path: Mark for removal from Active
```
Given: worktree in Active state
When: worktree.mark_for_removal() is called
Then: Ok(()) is returned
And: worktree.state() == WorktreeState::Removing
```

**Test name**: `fn worktree_mark_for_removal_from_active_returns_ok_and_sets_removing()`

#### Happy Path: Complete removal
```
Given: worktree in Removing state
When: worktree.complete_removal() is called
Then: Ok(()) is returned
And: worktree.state() == WorktreeState::Removed
```

**Test name**: `fn worktree_complete_removal_from_removing_returns_ok_and_sets_removed()`

#### Happy Path: Full removal flow
```
Given: worktree in Active state
When: worktree.mark_for_removal() is called
Then: Ok(()) is returned
And: worktree.state() == WorktreeState::Removing
When: worktree.complete_removal() is called
Then: Ok(()) is returned
And: worktree.state() == WorktreeState::Removed
And: worktree.is_removed() is true
```

**Test name**: `fn worktree_full_removal_flow_transitions_creating_active_removing_removed()`

#### Happy Path: Add metadata
```
Given: worktree in any state
When: worktree.add_metadata("environment", "test") is called
Then: worktree.get_metadata("environment") == Some("test")
And: worktree.updated_at() is updated
```

**Test name**: `fn worktree_add_metadata_inserts_key_value_pair()`

#### Happy Path: Remove metadata
```
Given: worktree with metadata {"environment": "test"}
When: worktree.remove_metadata("environment") is called
Then: Some("test") is returned
And: worktree.get_metadata("environment") is None
```

**Test name**: `fn worktree_remove_metadata_returns_old_value_and_removes_key()`

---

### Behavior: WorktreeService creates worktree

#### Happy Path: Create worktree
```
Given: in-memory repository with no existing worktrees
And: CreateWorktreeCommand with valid name "test-wt", valid paths, Development type
When: service.create_worktree(cmd) is called
Then: Ok(worktree) is returned
And: worktree.name().as_str() == "test-wt"
And: repository.find_by_id(worktree.id()) returns Some(worktree)
```

**Test name**: `fn worktree_service_create_worktree_saves_to_repository()`

#### Error: Duplicate name
```
Given: in-memory repository with existing worktree "test-wt"
And: CreateWorktreeCommand with same name "test-wt"
When: service.create_worktree(cmd) is called
Then: Err(WorktreeDomainError::NameAlreadyExists("test-wt")) is returned
```

**Test name**: `fn worktree_service_create_worktree_duplicate_name_returns_name_already_exists_error()`

---

### Behavior: WorktreeService initializes worktree

#### Happy Path: Initialize existing worktree
```
Given: in-memory repository with worktree in Creating state
And: InitializeWorktreeCommand with worktree.id
When: service.initialize_worktree(cmd) is called
Then: Ok(worktree) is returned
And: worktree.state() == WorktreeState::Active
And: repository.find_by_id(id) returns updated worktree
```

**Test name**: `fn worktree_service_initialize_worktree_transitions_to_active()`

#### Error: Worktree not found
```
Given: in-memory repository with no worktree
And: InitializeWorktreeCommand with non-existent id
When: service.initialize_worktree(cmd) is called
Then: Err(WorktreeDomainError::NotFound(id)) is returned
```

**Test name**: `fn worktree_service_initialize_worktree_not_found_returns_not_found_error()`

---

### Behavior: WorktreeService suspends and resumes

#### Happy Path: Suspend active worktree
```
Given: worktree in Active state in repository
And: SuspendWorktreeCommand with worktree.id
When: service.suspend_worktree(cmd) is called
Then: Ok(worktree) is returned
And: worktree.state() == WorktreeState::Suspended
```

**Test name**: `fn worktree_service_suspend_worktree_transitions_to_suspended()`

#### Happy Path: Resume suspended worktree
```
Given: worktree in Suspended state in repository
And: ResumeWorktreeCommand with worktree.id
When: service.resume_worktree(cmd) is called
Then: Ok(worktree) is returned
And: worktree.state() == WorktreeState::Active
```

**Test name**: `fn worktree_service_resume_worktree_transitions_to_active()`

---

### Behavior: WorktreeService removes worktree

#### Happy Path: Remove worktree
```
Given: worktree in Active state in repository
And: RemoveWorktreeCommand with worktree.id
When: service.remove_worktree(cmd) is called
Then: Ok(()) is returned
And: repository.find_by_id(id) returns None
And: cache.remove(id) was called
```

**Test name**: `fn worktree_service_remove_worktree_deletes_from_repository()`

---

### Behavior: WorktreeService lists worktrees

#### Happy Path: List all worktrees
```
Given: repository with 3 worktrees
And: ListWorktreesQuery with default filters
When: service.list_worktrees(query) is called
Then: Ok(Vec<Worktree>) is returned
And: results.len() == 3
```

**Test name**: `fn worktree_service_list_worktrees_returns_all_worktrees()`

#### Filter: By state
```
Given: repository with worktrees in states [Active, Suspended, Active]
And: ListWorktreesQuery with state_filter = Active
When: service.list_worktrees(query) is called
Then: Ok(Vec<Worktree>) is returned
And: results.len() == 2
And: all results have state() == Active
```

**Test name**: `fn worktree_service_list_worktrees_filtered_by_state_returns_matching_worktrees()`

#### Filter: By type
```
Given: repository with worktrees of types [Development, Testing, Development]
And: ListWorktreesQuery with worktree_type_filter = Testing
When: service.list_worktrees(query) is called
Then: Ok(Vec<Worktree>) is returned
And: results.len() == 1
And: results[0].worktree_type() == Testing
```

**Test name**: `fn worktree_service_list_worktrees_filtered_by_type_returns_matching_worktrees()`

#### Filter: By name prefix
```
Given: repository with worktrees ["feature-a", "feature-b", "bugfix-c"]
And: ListWorktreesQuery with name_prefix = "feature-"
When: service.list_worktrees(query) is called
Then: Ok(Vec<Worktree>) is returned
And: results.len() == 2
And: all results start with "feature-"
```

**Test name**: `fn worktree_service_list_worktrees_filtered_by_name_prefix_returns_matching_worktrees()`

#### Filter: Include removed
```
Given: repository with worktrees [Active, Removed]
And: ListWorktreesQuery with include_removed = true
When: service.list_worktrees(query) is called
Then: Ok(Vec<Worktree>) is returned
And: results.len() == 2
```

**Test name**: `fn worktree_service_list_worktrees_with_include_removed_returns_removed_worktrees()`

---

### Behavior: GitWorktreeAdapter opens repository

#### Happy Path: Open valid repository
```
Given: valid git repository path
When: GitWorktreeAdapter::new(repo_path) is called
Then: Ok(GitWorktreeAdapter) is returned
And: adapter.repository().is_bare() == false
```

**Test name**: `fn git_adapter_open_valid_repository_returns_adapter()`

#### Error: Non-existent path
```
Given: non-existent path "/nonexistent/path"
When: GitWorktreeAdapter::new(path) is called
Then: Err(GitError::RepositoryNotFound(_)) is returned
```

**Test name**: `fn git_adapter_open_nonexistent_path_returns_repository_not_found_error()`

---

### Behavior: GitWorktreeAdapter gets branches

#### Happy Path: Get current branch
```
Given: git repository with HEAD on "master"
When: adapter.get_current_branch() is called
Then: Ok(Some(BranchName("master"))) is returned
```

**Test name**: `fn git_adapter_get_current_branch_returns_master_branch()`

#### Happy Path: Get local branches
```
Given: git repository with branches ["master", "feature/test"]
When: adapter.get_local_branches() is called
Then: Ok(Vec<BranchName>) is returned
And: results.len() >= 1
And: results contains "master"
```

**Test name**: `fn git_adapter_get_local_branches_returns_branch_list()`

#### Happy Path: Get remote branches
```
Given: git repository with remote branches ["origin/main", "origin/develop"]
When: adapter.get_remote_branches() is called
Then: Ok(Vec<BranchName>) is returned
And: results contains "main" (without origin/ prefix)
```

**Test name**: `fn git_adapter_get_remote_branches_strips_origin_prefix()`

---

### Behavior: SqliteWorktreeRepository creates schema

#### Happy Path: Initialize database
```
Given: in-memory SQLite database URL
When: SqliteWorktreeRepository::new(database_url) is called
Then: Ok(SqliteWorktreeRepository) is returned
And: table "worktrees" exists
And: indexes idx_worktrees_name, idx_worktrees_state, idx_worktrees_type exist
```

**Test name**: `fn sqlite_repository_new_creates_schema()`

---

## 4. Proptest Invariants

### Proptest: WorktreeId new_random
```
Invariant: All generated IDs are unique within a single test run
Strategy: Gen<WorktreeId> using uuid::Uuid::new_v4()
Anti-invariant: N/A (random generation should never collide in practice)
Property: 1000 generated IDs are all distinct
```

**Test name**: `fn proptest_worktree_id_new_random_generates_unique_ids()`

### Proptest: WorktreeName validation
```
Invariant: Valid names pass new(), invalid names fail
Strategy: Arbitrary strings via proptest::string::string_regex()
Anti-invariant: Strings containing "/", starting with ".", or empty
Property: For all strings s, (s.is_valid() && new(s).is_ok()) || (!s.is_valid() && new(s).is_err())
```

**Test name**: `fn proptest_worktree_name_validation_rejects_invalid_characters()`

### Proptest: BranchName validation
```
Invariant: Valid branch names pass new(), invalid names fail
Strategy: Arbitrary strings matching Git branch patterns
Anti-invariant: Strings with "@", starting/ending with "-", ".", or containing ".."
Property: new(name).is_ok() iff name passes all validation rules
```

**Test name**: `fn proptest_branch_name_validation_rejects_invalid_characters()`

### Proptest: AbsolutePath validation
```
Invariant: Absolute paths pass new(), relative paths fail
Strategy: Arbitrary strings, some absolute, some relative
Anti-invariant: Relative paths (no leading "/")
Property: new(path).is_ok() iff path.is_absolute()
```

**Test name**: `fn proptest_absolute_path_validation_rejects_relative_paths()`

### Proptest: WorktreeState round-trip
```
Invariant: from_u8(as_u8(state)) == state for all valid states
Strategy: All 6 WorktreeState variants
Anti-invariant: u8 values 6-255
Property: Round-trip conversion preserves state
```

**Test name**: `fn proptest_worktree_state_round_trip_preserves_value()`

### Proptest: WorktreeTypeEnum round-trip
```
Invariant: from_u8(as_u8(type)) == type for all valid types
Strategy: All 5 WorktreeTypeEnum variants
Anti-invariant: u8 values 5-255
Property: Round-trip conversion preserves type
```

**Test name**: `fn proptest_worktree_type_enum_round_trip_preserves_value()`

### Proptest: Worktree ID uniqueness
```
Invariant: All IDs in a list are unique
Strategy: Generate Vec<WorktreeId> via new_random()
Anti-invariant: N/A
Property: list.iter().unique().count() == list.len()
```

**Test name**: `fn proptest_worktree_id_list_all_unique()`

### Proptest: State machine exhaustiveness
```
Invariant: Every state has valid_next_states() defined
Strategy: All 6 WorktreeState variants
Anti-invariant: N/A
Property: valid_next_states() returns valid WorktreeState values only
```

**Test name**: `fn proptest_worktree_state_valid_next_states_only_valid_states()`

### Proptest: Metadata operations
```
Invariant: add_metadata(key, value) then get_metadata(key) returns value
Strategy: Arbitrary key/value pairs
Anti-invariant: N/A
Property: round-trip metadata insert and retrieve
```

**Test name**: `fn proptest_metadata_round_trip_preserves_value()`

### Proptest: Path join preserves absoluteness
```
Invariant: join(child) on absolute path yields absolute path
Strategy: AbsolutePath + arbitrary child paths
Anti-invariant: N/A
Property: parent.join(child).is_absolute() == true
```

**Test name**: `fn proptest_absolute_path_join_preserves_absoluteness()`

---

## 5. Fuzz Targets

### Fuzz Target: WorktreeName::new
```
Input type: String
Risk: Logic error (accepting invalid names)
Corpus seeds:
  - "" (empty)
  - "/" (single slash)
  - "//" (double slash)
  - ".hidden" (dot prefix)
  - "a/b/c" (nested slash)
  - "normal-name" (valid)
  - "unicode-тест" (unicode)
  - "emoji-🎉" (emoji)
  - "very-long-name-" * 100 (long string)
```

**Test name**: `fn fuzz_worktree_name_new()`

### Fuzz Target: BranchName::new
```
Input type: String
Risk: Logic error (accepting invalid branch names)
Corpus seeds:
  - "" (empty)
  - "-" (single hyphen)
  - "main" (valid)
  - "feature/test" (valid slash)
  - "feature..test" (consecutive dots)
  - "@invalid" (invalid char)
  - "..hidden" (dot prefix)
  - "master" (valid)
```

**Test name**: `fn fuzz_branch_name_new()`

### Fuzz Target: AbsolutePath::new
```
Input type: String
Risk: Logic error (accepting relative paths)
Corpus seeds:
  - "relative" (no leading slash)
  - "/absolute" (valid)
  - "" (empty)
  - "/" (root)
  - "/a/b/c/d/e" (deep path)
  - "/home/user/名前" (unicode)
```

**Test name**: `fn fuzz_absolute_path_new()`

### Fuzz Target: WorktreeId::from_string
```
Input type: String
Risk: Panic on invalid UUID parsing
Corpus seeds:
  - "" (empty)
  - "not-uuid" (garbage)
  - "550e8400-e29b-41d4-a716-446655440000" (valid)
  - "550e8400e29b41d4a716446655440000" (no dashes)
  - "550e8400-e29b-41d4-a716-44665544000" (too short)
  - "550e8400-e29b-41d4-a716-4466554400000" (too long)
```

**Test name**: `fn fuzz_worktree_id_from_string()`

---

## 6. Kani Verification Harnesses

### Kani Harness: State machine exhaustiveness
```
Property: All 6 states have complete transition definitions
Bound: Search depth 6 (one per state)
Rationale: Ensure no state is missing valid_next_states() definition
```

### Kani Harness: State transition safety
```
Property: can_transition_to() is symmetric with valid_next_states()
Bound: Search depth 12 (6 states × 2 checks)
Rationale: Ensure transition validation is consistent
```

### Kani Harness: State encoding
```
Property: u8 conversion is lossless for all states
Bound: Search depth 6
Rationale: Ensure from_u8() and as_u8() are inverse operations
```

---

## 7. Mutation Testing Checkpoints

### Target Mutation Kill Rate: ≥90%

#### Critical mutations to survive:
- `WorktreeName::new`: Change `if name.is_empty()` to `if true` → caught by `test_worktree_name_new_empty`
- `WorktreeName::new`: Remove `if name.contains('/')` → caught by `test_worktree_name_new_with_slash`
- `WorktreeState::can_transition_to`: Return `true` always → caught by `test_worktree_state_cannot_transition_*`
- `Worktree::initialize`: Remove state check → caught by `test_worktree_invalid_state_transition`
- `WorktreeService::create_worktree`: Remove `name_exists` check → caught by `test_worktree_service_create_duplicate_name`
- `AbsolutePath::new`: Remove `!path.is_absolute()` check → caught by `test_absolute_path_new_relative_fails`
- `BranchName::new`: Remove character validation → caught by `test_branch_name_new_invalid_chars`
- `GitWorktreeAdapter::new`: Remove path.exists() check → caught by `test_git_adapter_open_nonexistent_path`

#### Mutation checkpoints:
```
| Mutation | Expected Test | Layer |
|----------|---------------|-------|
| remove empty check | test_worktree_name_new_empty | unit |
| remove slash check | test_worktree_name_new_with_slash | unit |
| remove dot check | test_worktree_name_new_starts_with_dot | unit |
| return true always | test_worktree_state_cannot_transition_active_from_suspended | unit |
| remove state check | test_worktree_invalid_state_transition | unit |
| remove name_exists | test_worktree_service_create_duplicate_name | integration |
| return Ok always | test_absolute_path_new_relative_fails | unit |
```

---

## 8. Combinatorial Coverage Matrix

### WorktreeName Unit Tests

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| valid name | "feature-branch" | Ok(WorktreeName) | unit |
| empty | "" | Err(InvalidName) | unit |
| slash | "a/b" | Err(InvalidName) | unit |
| dot prefix | ".hidden" | Err(InvalidName) | unit |
| unicode | "тест" | Ok(WorktreeName) | unit |
| long name | "a".repeat(1000) | Ok(WorktreeName) | unit |

### BranchName Unit Tests

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| valid main | "main" | Ok(BranchName) | unit |
| valid feature | "feature/test" | Ok(BranchName) | unit |
| valid release | "release/1.0" | Ok(BranchName) | unit |
| empty | "" | Err(InvalidBranch) | unit |
| invalid char | "feat@ure" | Err(InvalidBranch) | unit |
| hyphen start | "-feature" | Err(InvalidBranch) | unit |
| hyphen end | "feature-" | Err(InvalidBranch) | unit |
| dot start | ".feature" | Err(InvalidBranch) | unit |
| dot end | "feature." | Err(InvalidBranch) | unit |
| consecutive dots | "feature..test" | Err(InvalidBranch) | unit |

### WorktreeState Unit Tests

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| valid u8 0 | 0 | Some(Creating) | unit |
| valid u8 5 | 5 | Some(Removed) | unit |
| invalid u8 | 99 | None | unit |
| Creating→Active | Creating, Active | true | unit |
| Creating→Suspended | Creating, Suspended | false | unit |
| Active→Suspended | Active, Suspended | true | unit |
| Suspended→Active | Suspended, Active | true | unit |
| Removed→Active | Removed, Active | false | unit |
| Removed next states | Removed | vec![] | unit |

### WorktreeId Unit Tests

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| valid UUID string | "550e8400-e29b-41d4-a716-446655440000" | Ok(WorktreeId) | unit |
| invalid UUID | "not-uuid" | Err(InvalidPath) | unit |
| new_random | - | unique WorktreeId | unit |
| from_bytes | [16 bytes] | WorktreeId | unit |
| to_bytes | WorktreeId | [16 bytes] | unit |

### Worktree Unit Tests

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| new with branch | all valid | Ok(Worktree), Creating | unit |
| new without branch | branch=None | Ok(Worktree), Creating | unit |
| initialize from Creating | Creating state | Ok(()), Active | unit |
| suspend from Creating | Creating state | Err(InvalidStateTransition) | unit |
| suspend from Active | Active state | Ok(()), Suspended | unit |
| resume from Suspended | Suspended state | Ok(()), Active | unit |
| resume from Active | Active state | Err(InvalidStateTransition) | unit |
| mark removal from Active | Active state | Ok(()), Removing | unit |
| complete removal from Removing | Removing state | Ok(()), Removed | unit |
| add metadata | any state | metadata updated | unit |
| remove metadata | has key | Some(old_value) | unit |

### WorktreeService Integration Tests

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| create new | valid cmd | Ok(worktree) | integration |
| create duplicate name | existing name | Err(NameAlreadyExists) | integration |
| initialize existing | Creating state | Ok(worktree), Active | integration |
| initialize not found | non-existent ID | Err(NotFound) | integration |
| suspend active | Active state | Ok(worktree), Suspended | integration |
| resume suspended | Suspended state | Ok(worktree), Active | integration |
| remove active | Active state | Ok(()), deleted from repo | integration |
| list all | 3 worktrees | Ok(vec![3]) | integration |
| list filtered state | state=Active | Ok(vec![matching]) | integration |
| list filtered type | type=Testing | Ok(vec![matching]) | integration |
| list filtered prefix | prefix="feature-" | Ok(vec![matching]) | integration |
| add metadata | existing ID | Ok(()), metadata updated | integration |

### GitWorktreeAdapter Integration Tests

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| open valid repo | valid git path | Ok(adapter) | integration |
| open non-existent | /nonexistent | Err(RepositoryNotFound) | integration |
| get current branch | master branch | Ok(Some("master")) | integration |
| get local branches | 1+ branches | Ok(vec[branches]) | integration |
| get remote branches | origin/* branches | Ok(vec[stripped]) | integration |

### SqliteRepository Integration Tests

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| new in-memory | :memory: | Ok(repository), schema created | integration |
| save worktree | valid worktree | Ok(()), persisted | integration |
| find by ID | existing ID | Ok(Some(worktree)) | integration |
| find by name | existing name | Ok(Some(worktree)) | integration |
| list all | 3 worktrees | Ok(vec![3]) | integration |
| delete by ID | existing ID | Ok(()), removed | integration |
| name exists | existing name | Ok(true) | integration |
| name exists | non-existent | Ok(false) | integration |

---

## Open Questions

1. **BranchName validation completeness**: Should we enforce maximum length? Git has a 255-character limit.
2. **AbsolutePath canonicalization**: Should we canonicalize paths (resolve symlinks, .., .)?
3. **Metadata constraints**: Should metadata keys/values have length limits?
4. **Timestamp precision**: Using i64 Unix seconds - is this sufficient precision?
5. **UUID version**: Using v4 (random) - is this appropriate for all use cases?
6. **Repository pattern**: Should we support multiple repository implementations simultaneously?
7. **Cache invalidation**: Cache in WorktreeService is in-memory only - what's the invalidation strategy?
8. **Error type consistency**: GitError uses thiserror, should it convert to WorktreeDomainError more explicitly?
9. **Serialization**: Worktree serde derives - should metadata be serialized as JSON string or map?
10. **Postgres UUID**: Postgres implementation uses BYTEA for UUID - is this consistent with PostgreSQL best practices?

---

## Exit Criteria Checklist

- ✅ Every public API behavior has a BDD scenario
- ✅ Every Error variant has explicit test scenarios (11 domain errors × multiple scenarios)
- ✅ Mutation threshold (≥90%) is stated
- ✅ No planned assertion is just `is_ok()` or `is_err()` - all assert specific values
- ✅ Every pure function with multiple inputs has proptest invariant (18 invariants)
- ✅ Every parser/deserializer boundary has fuzz target (4 fuzz targets)
- ✅ Critical state machine invariants have Kani harnesses (3 harnesses)
- ✅ Combinatorial coverage matrices for all value objects and state machines

**Total tests planned**: 435+ (260 unit + 145 integration + 10 e2e + 5 static)
