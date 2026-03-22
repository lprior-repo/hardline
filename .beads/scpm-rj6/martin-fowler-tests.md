# Martin Fowler Test Plan

## Bead: scpm-rj6 - Session Repository SQLite Implementation

## Happy Path Tests

### Scenario: test_saves_valid_session_and_retrieves_by_id
Given: A valid Session with id "session-uuid-123", name "test-session", state Created
When: save(session) is called followed by find_by_id("session-uuid-123")
Then: Returns Ok(Some(session)) with all fields preserved

### Scenario: test_saves_valid_session_and_retrieves_by_name
Given: A valid Session with name "my-session"
When: save(session) is called followed by find_by_name("my-session")
Then: Returns Ok(Some(session)) with matching name

### Scenario: test_lists_all_sessions
Given: Three valid Sessions with names "session-1", "session-2", "session-3"
When: list() is called after saving all three
Then: Returns Ok(vec![session1, session2, session3])

### Scenario: test_deletes_existing_session
Given: A saved Session with id "session-to-delete"
When: delete("session-to-delete") is called
Then: find_by_id("session-to-delete") returns Ok(None)

## Error Path Tests

### Scenario: test_find_by_id_returns_not_found_for_nonexistent
Given: No session with id "nonexistent-id" in database
When: find_by_id("nonexistent-id") is called
Then: Returns Ok(None) (not an error - absence is not a failure)

### Scenario: test_find_by_name_returns_not_found_for_nonexistent
Given: No session with name "nonexistent" in database
When: find_by_name("nonexistent") is called
Then: Returns Ok(None)

### Scenario: test_delete_returns_not_found_for_nonexistent
Given: No session with id "nonexistent" in database
When: delete("nonexistent") is called
Then: Returns Err(SessionError::NotFound("nonexistent"))

### Scenario: test_save_persists_immediately
Given: A valid Session
When: save(session) is called, then the database is queried directly
Then: The session appears in database immediately (WAL flush)

### Scenario: test_save_same_id_twice_updates
Given: A Session with id "session-123" saved with state Active
When: A different Session with same id "session-123" but state Completed is saved
Then: find_by_id("session-123") returns the updated session with state Completed

## Edge Case Tests

### Scenario: test_handles_empty_database_gracefully
Given: An empty database
When: list() is called
Then: Returns Ok(vec![]) (empty vec, not error)

### Scenario: test_list_after_delete_returns_correct_count
Given: Three sessions saved, then one deleted
When: list() is called
Then: Returns vec with 2 sessions

### Scenario: test_session_with_unicode_name
Given: A Session with name "test-会话"
When: save(session) is called
Then: find_by_name("test-会话") returns Ok(Some(session))

### Scenario: test_session_with_special_characters_in_branch
Given: A Session with branch OnBranch { name: "feature/test" }
When: save(session) is called and retrieved
Then: Branch name is preserved as "feature/test"

## Contract Verification Tests

### Scenario: test_precondition_db_initialized
Given: SqliteDatabaseService that failed to connect (returns Error)
When: save(session) is attempted
Then: Returns Err(SessionError::RepositoryError)

### Scenario: test_postcondition_write_persists
Given: A valid session
When: save(session) returns Ok(())
Then: A subsequent find_by_id returns Ok(Some(session)) (write actually persisted)

### Scenario: test_invariant_valid_uuid_primary_key
Given: Sessions are saved and retrieved
When: find_by_id is called with valid UUID
Then: SessionId validation passes (no InvalidIdentifier error)

### Scenario: test_invariant_wal_serialization
Given: Multiple concurrent save operations
When: All completes without SQLITE_BUSY errors
Then: SqliteDatabaseService WAL mode is functioning (serialization via WAL guarantee)

## Contract Violation Tests

### Scenario: test_p3_violation_empty_session_id
Given: An empty string session ID
When: find_by_id("") is called
Then: Returns Err(SessionError::InvalidIdentifier("SessionId cannot be empty"))

### Scenario: test_p4_violation_empty_session_name
Given: An invalid SessionName (empty string)
When: SessionName::parse("") is called
Then: Returns Err(IdentifierError::Empty)

## Given-When-Then Scenarios

### Scenario 1: Successful Session Persistence
Given: A SqliteDatabaseService connected to in-memory database, schema migrated
And: A valid Session entity with name "test-persistence"
When: repository.save(session) is called
Then: Returns Ok(())
And: repository.find_by_id(session.id) returns Ok(Some(session))
And: repository.find_by_name("test-persistence") returns Ok(Some(session))
And: repository.list() returns a vector containing the session

### Scenario 2: Session Not Found After Delete
Given: A SqliteDatabaseService connected to in-memory database
And: A saved session with id "to-delete"
When: repository.delete("to-delete") is called
Then: Returns Ok(())
When: repository.find_by_id("to-delete") is called
Then: Returns Ok(None)

### Scenario 3: Duplicate Session Name Allowed
Given: A SqliteDatabaseService connected to in-memory database
And: Two different sessions with different IDs but same name "duplicate"
When: Both sessions are saved
Then: Both save operations return Ok(())
And: repository.find_by_name("duplicate") returns Ok(Some(first_session))

### Scenario 4: State Transitions Persisted Correctly
Given: A session in Created state
When: session.transition(Activated) is called, then saved
Then: Retrieved session has state Active
And: Retrieved session has same id, name, created_at as original
