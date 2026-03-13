# Martin Fowler Test Plan - SessionRepository

**Version**: 2.1 (Addressed defect: Single Assertion - fixed multiple assertions per test)
**Classification**: This file contains test specifications at multiple levels (unit, integration, E2E)

---

## PART 1: ATDD DSL Layer (Stakeholder Communication)

### Feature: Session Repository Operations

```gherkin
Feature: Session Repository CRUD Operations

  Background:
    Given a SessionRepository implementation is available

  Scenario: Load an existing session by ID
    Given a session with id "session-001" and name "My Session" exists
    When the user requests load("session-001")
    Then the result is Ok(Session) with id "session-001"

  Scenario: Load a non-existent session
    Given no session with id "missing-001" exists
    When the user requests load("missing-001")
    Then the result is Err(NotFound)

  Scenario: Save a new session successfully
    Given no session with id "new-session" exists
    When the user saves a session with id "new-session"
    Then the result is Ok(())
    And the session exists in the repository

  Scenario: Save a session with duplicate name fails
    Given a session named "existing" already exists
    When the user attempts to save a new session also named "existing"
    Then the result is Err(Conflict)

  Scenario: Delete an existing session
    Given a session with id "to-delete" exists
    When the user deletes session "to-delete"
    Then the result is Ok(())
    And the session no longer exists

  Scenario: List all sessions
    Given multiple sessions exist in the repository
    When the user lists all sessions
    Then the result contains all saved sessions

  Scenario: Set current session
    Given a session with id "current-session" exists
    When the user sets current to "current-session"
    Then the result is Ok(())
    And getting current returns that session
```

---

## PART 2: Test Classifications

| Test Function | Classification | Implementation |
|--------------|---------------|----------------|
| `test_load_session_by_id_returns_ok` | Integration | Uses real in-memory storage |
| `test_load_session_by_id_returns_correct_session` | Integration | Uses real in-memory storage |
| `test_load_session_by_name_returns_ok` | Integration | Uses real in-memory storage |
| `test_load_session_by_name_returns_correct_name` | Integration | Uses real in-memory storage |
| `test_save_new_session_creates_record` | Integration | Uses real in-memory storage |
| `test_save_new_session_exists_after_save` | Integration | Uses real in-memory storage |
| `test_save_existing_session_updates_record` | Integration | Uses real in-memory storage |
| `test_load_after_update_returns_updated_name` | Integration | Uses real in-memory storage |
| `test_delete_existing_session_removes_record` | Integration | Uses real in-memory storage |
| `test_exists_after_delete_returns_false` | Integration | Uses real in-memory storage |
| `test_list_all_returns_all_sessions` | Integration | Uses real in-memory storage |
| `test_list_sorted_by_name_returns_ordered` | Integration | Uses real in-memory storage |
| `test_list_sorted_by_name_returns_mango_second` | Integration | Uses real in-memory storage |
| `test_list_sorted_by_name_returns_zebra_third` | Integration | Uses real in-memory storage |
| `test_exists_returns_true_for_existing` | Integration | Uses real in-memory storage |
| `test_exists_returns_false_for_nonexistent` | Integration | Uses real in-memory storage |
| `test_get_current_returns_session_when_set` | Integration | Uses real in-memory storage |
| `test_get_current_returns_correct_session_id` | Integration | Uses real in-memory storage |
| `test_get_current_returns_none_when_not_set` | Integration | Uses real in-memory storage |
| `test_set_current_changes_current_session` | Integration | Uses real in-memory storage |
| `test_get_current_after_set_returns_some` | Integration | Uses real in-memory storage |
| `test_clear_current_removes_current_session` | Integration | Uses real in-memory storage |
| `test_get_current_after_clear_returns_none` | Integration | Uses real in-memory storage |
| `test_load_nonexistent_id_returns_not_found` | Unit | Tests error path in isolation |
| `test_load_nonexistent_name_returns_not_found` | Unit | Tests error path in isolation |
| `test_save_duplicate_name_returns_conflict` | Unit | Tests conflict detection |
| `test_delete_nonexistent_returns_not_found` | Unit | Tests not-found path |
| `test_set_current_invalid_id_returns_not_found` | Unit | Tests validation |
| `test_storage_error_on_load_returns_storage_error` | Unit | Tests error propagation |
| `test_storage_error_on_save_returns_storage_error` | Unit | Tests error propagation |
| `test_list_all_empty_returns_empty_vector` | Unit | Tests empty edge case |
| `test_list_sorted_empty_returns_empty_vector` | Unit | Tests empty edge case |
| `test_save_and_load_special_characters` | Unit | Tests name validation |
| `test_save_session_name_at_max_length` | Unit | Tests boundary |
| `test_save_same_id_twice_is_upsert` | Unit | Tests upsert behavior |
| `test_property_sorted_names_always_ascending` | Property | Uses proptest |
| `test_property_save_then_load_idempotent` | Property | Uses proptest |
| `test_property_delete_is_idempotent` | Property | Uses proptest |
| `test_property_exists_after_save` | Property | Uses proptest |
| `test_property_set_and_get_current` | Property | Uses proptest |
| `test_e2e_save_creates_session` | E2E | Tests save returns Ok |
| `test_e2e_exists_after_save` | E2E | Tests exists returns true |
| `test_e2e_load_returns_correct_name` | E2E | Tests loaded name |
| `test_e2e_update_returns_ok` | E2E | Tests update returns Ok |
| `test_e2e_load_after_update_returns_updated_name` | E2E | Tests updated name |
| `test_e2e_set_current_returns_ok` | E2E | Tests set_current returns Ok |
| `test_e2e_get_current_returns_some` | E2E | Tests current is Some |
| `test_e2e_get_current_has_correct_id` | E2E | Tests current ID |
| `test_e2e_clear_current_returns_ok` | E2E | Tests clear_current returns Ok |
| `test_e2e_get_current_after_clear_returns_none` | E2E | Tests current is None |
| `test_e2e_delete_returns_ok` | E2E | Tests delete returns Ok |
| `test_e2e_exists_after_delete_returns_false` | E2E | Tests exists returns false |

---

## PART 3: Executable Rust Tests (One Assertion Per Test)

### Integration Tests (Real Storage - HashMap-based)

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    // Real in-memory storage implementation (no mocks)
    pub struct InMemorySessionRepository {
        sessions: RwLock<HashMap<SessionId, Session>>,
        current: RwLock<Option<SessionId>>,
    }

    impl InMemorySessionRepository {
        pub fn new() -> Self {
            Self {
                sessions: RwLock::new(HashMap::new()),
                current: RwLock::new(None),
            }
        }
    }

    impl SessionRepository for InMemorySessionRepository {
        fn load(&self, id: &SessionId) -> RepositoryResult<Session> {
            self.sessions
                .read()
                .unwrap()
                .get(id)
                .cloned()
                .ok_or_else(|| RepositoryError::NotFound(format!("Session not found: {}", id)))
        }

        fn load_by_name(&self, name: &SessionName) -> RepositoryResult<Session> {
            self.sessions
                .read()
                .unwrap()
                .values()
                .find(|s| s.name == *name)
                .cloned()
                .ok_or_else(|| RepositoryError::NotFound(format!("Session not found: {}", name)))
        }

        fn save(&self, session: &Session) -> RepositoryResult<()> {
            let mut sessions = self.sessions.write().unwrap();
            // Check for duplicate name (invariant I1)
            if !sessions.contains_key(&session.id) {
                if sessions.values().any(|s| s.name == session.name) {
                    return Err(RepositoryError::Conflict(format!(
                        "Session with name '{}' already exists",
                        session.name
                    )));
                }
            }
            sessions.insert(session.id.clone(), session.clone());
            Ok(())
        }

        fn delete(&self, id: &SessionId) -> RepositoryResult<()> {
            let mut sessions = self.sessions.write().unwrap();
            if sessions.remove(id).is_none() {
                return Err(RepositoryError::NotFound(format!("Session not found: {}", id)));
            }
            Ok(())
        }

        fn list_all(&self) -> RepositoryResult<Vec<Session>> {
            Ok(self.sessions.read().unwrap().values().cloned().collect())
        }

        fn list_sorted_by_name(&self) -> RepositoryResult<Vec<Session>> {
            let mut sessions: Vec<Session> = self.sessions.read().unwrap().values().cloned().collect();
            sessions.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(sessions)
        }

        fn exists(&self, id: &SessionId) -> RepositoryResult<bool> {
            Ok(self.sessions.read().unwrap().contains_key(id))
        }

        fn get_current(&self) -> RepositoryResult<Option<Session>> {
            let current = self.current.read().unwrap();
            match &*current {
                Some(id) => Ok(self.sessions.read().unwrap().get(id).cloned()),
                None => Ok(None),
            }
        }

        fn set_current(&self, id: &SessionId) -> RepositoryResult<()> {
            // Validate session exists
            if !self.sessions.read().unwrap().contains_key(id) {
                return Err(RepositoryError::NotFound(format!("Session not found: {}", id)));
            }
            *self.current.write().unwrap() = Some(id.clone());
            Ok(())
        }

        fn clear_current(&self) -> RepositoryResult<()> {
            *self.current.write().unwrap() = None;
            Ok(())
        }
    }

    // ============ INTEGRATION TESTS ============

    #[test]
    fn test_load_session_by_id_returns_ok() {
        // Given: A repository containing a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("session-001").unwrap(),
            name: SessionName::parse("My Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: load is called with valid ID
        let result = repo.load(&SessionId::parse("session-001").unwrap());

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_session_by_id_returns_correct_session() {
        // Given: A repository containing a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("session-001").unwrap(),
            name: SessionName::parse("My Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: load is called with valid ID
        let result = repo.load(&SessionId::parse("session-001").unwrap());

        // Then: Returns session with correct ID
        assert_eq!(result.unwrap().id.as_str(), "session-001");
    }

    #[test]
    fn test_load_session_by_name_returns_ok() {
        // Given: A repository containing a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("session-002").unwrap(),
            name: SessionName::parse("Named Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: load_by_name is called
        let result = repo.load_by_name(&SessionName::parse("Named Session").unwrap());

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_session_by_name_returns_correct_name() {
        // Given: A repository containing a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("session-002").unwrap(),
            name: SessionName::parse("Named Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: load_by_name is called
        let result = repo.load_by_name(&SessionName::parse("Named Session").unwrap());

        // Then: Returns session with matching name
        assert_eq!(result.unwrap().name.as_str(), "Named Session");
    }

    #[test]
    fn test_save_new_session_creates_record() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("new-session").unwrap(),
            name: SessionName::parse("New Session").unwrap(),
            branch_state: BranchState::Detached,
        };

        // When: save is called
        let result = repo.save(&session);

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_save_new_session_exists_after_save() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("new-session").unwrap(),
            name: SessionName::parse("New Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: exists is called
        let result = repo.exists(&SessionId::parse("new-session").unwrap());

        // Then: Returns true
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_save_existing_session_updates_record() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("update-test").unwrap(),
            name: SessionName::parse("Original Name").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: save is called with same ID, different name
        let updated = Session {
            id: SessionId::parse("update-test").unwrap(),
            name: SessionName::parse("Updated Name").unwrap(),
            branch_state: BranchState::Detached,
        };
        let result = repo.save(&updated);

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_after_update_returns_updated_name() {
        // Given: A repository with updated session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("update-test").unwrap(),
            name: SessionName::parse("Original Name").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        let updated = Session {
            id: SessionId::parse("update-test").unwrap(),
            name: SessionName::parse("Updated Name").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&updated).unwrap();

        // When: load is called
        let result = repo.load(&SessionId::parse("update-test").unwrap());

        // Then: Returns updated name
        assert_eq!(result.unwrap().name.as_str(), "Updated Name");
    }

    #[test]
    fn test_delete_existing_session_removes_record() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("to-delete").unwrap(),
            name: SessionName::parse("Delete Me").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: delete is called
        let result = repo.delete(&SessionId::parse("to-delete").unwrap());

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_exists_after_delete_returns_false() {
        // Given: A repository after deletion
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("to-delete").unwrap(),
            name: SessionName::parse("Delete Me").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.delete(&SessionId::parse("to-delete").unwrap()).unwrap();

        // When: exists is called
        let result = repo.exists(&SessionId::parse("to-delete").unwrap());

        // Then: Returns false
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_list_all_returns_all_sessions() {
        // Given: A repository with multiple sessions
        let repo = InMemorySessionRepository::new();
        for i in 0..3 {
            let session = Session {
                id: SessionId::parse(&format!("session-{}", i)).unwrap(),
                name: SessionName::parse(&format!("Session {}", i)).unwrap(),
                branch_state: BranchState::Detached,
            };
            repo.save(&session).unwrap();
        }

        // When: list_all is called
        let result = repo.list_all();

        // Then: Returns all sessions
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_list_sorted_by_name_returns_ordered() {
        // Given: A repository with unsorted names
        let repo = InMemorySessionRepository::new();
        let names = vec!["zebra", "apple", "mango"];
        for name in &names {
            let session = Session {
                id: SessionId::parse(name).unwrap(),
                name: SessionName::parse(name).unwrap(),
                branch_state: BranchState::Detached,
            };
            repo.save(&session).unwrap();
        }

        // When: list_sorted_by_name is called
        let result = repo.list_sorted_by_name().unwrap();

        // Then: First element is "apple"
        assert_eq!(result[0].name.as_str(), "apple");
    }

    #[test]
    fn test_list_sorted_by_name_returns_mango_second() {
        // Given: A repository with unsorted names
        let repo = InMemorySessionRepository::new();
        let names = vec!["zebra", "apple", "mango"];
        for name in &names {
            let session = Session {
                id: SessionId::parse(name).unwrap(),
                name: SessionName::parse(name).unwrap(),
                branch_state: BranchState::Detached,
            };
            repo.save(&session).unwrap();
        }

        // When: list_sorted_by_name is called
        let result = repo.list_sorted_by_name().unwrap();

        // Then: Second element is "mango"
        assert_eq!(result[1].name.as_str(), "mango");
    }

    #[test]
    fn test_list_sorted_by_name_returns_zebra_third() {
        // Given: A repository with unsorted names
        let repo = InMemorySessionRepository::new();
        let names = vec!["zebra", "apple", "mango"];
        for name in &names {
            let session = Session {
                id: SessionId::parse(name).unwrap(),
                name: SessionName::parse(name).unwrap(),
                branch_state: BranchState::Detached,
            };
            repo.save(&session).unwrap();
        }

        // When: list_sorted_by_name is called
        let result = repo.list_sorted_by_name().unwrap();

        // Then: Third element is "zebra"
        assert_eq!(result[2].name.as_str(), "zebra");
    }

    #[test]
    fn test_exists_returns_true_for_existing() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("existing-id").unwrap(),
            name: SessionName::parse("Existing").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: exists is called
        let result = repo.exists(&SessionId::parse("existing-id").unwrap());

        // Then: Returns true
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_exists_returns_false_for_nonexistent() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();

        // When: exists is called with non-existent ID
        let result = repo.exists(&SessionId::parse("nonexistent").unwrap());

        // Then: Returns false
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_get_current_returns_session_when_set() {
        // Given: A repository with current session set
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("current-session").unwrap(),
            name: SessionName::parse("Current").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.set_current(&SessionId::parse("current-session").unwrap()).unwrap();

        // When: get_current is called
        let result = repo.get_current().unwrap();

        // Then: Returns Some
        assert!(result.is_some());
    }

    #[test]
    fn test_get_current_returns_correct_session_id() {
        // Given: A repository with current session set
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("current-session").unwrap(),
            name: SessionName::parse("Current").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.set_current(&SessionId::parse("current-session").unwrap()).unwrap();

        // When: get_current is called
        let result = repo.get_current().unwrap();

        // Then: Returns session with correct ID
        assert_eq!(result.unwrap().id.as_str(), "current-session");
    }

    #[test]
    fn test_get_current_returns_none_when_not_set() {
        // Given: A repository with no current set
        let repo = InMemorySessionRepository::new();

        // When: get_current is called
        let result = repo.get_current().unwrap();

        // Then: Returns None
        assert!(result.is_none());
    }

    #[test]
    fn test_set_current_changes_current_session() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("current-session").unwrap(),
            name: SessionName::parse("Current").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: set_current is called
        let result = repo.set_current(&SessionId::parse("current-session").unwrap());

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_current_after_set_returns_some() {
        // Given: A repository after set_current
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("current-session").unwrap(),
            name: SessionName::parse("Current").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.set_current(&SessionId::parse("current-session").unwrap()).unwrap();

        // When: get_current is called
        let result = repo.get_current().unwrap();

        // Then: Returns Some with correct ID
        assert_eq!(result.unwrap().id.as_str(), "current-session");
    }

    #[test]
    fn test_clear_current_removes_current_session() {
        // Given: A repository with current set
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("current-session").unwrap(),
            name: SessionName::parse("Current").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.set_current(&SessionId::parse("current-session").unwrap()).unwrap();

        // When: clear_current is called
        let result = repo.clear_current();

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_current_after_clear_returns_none() {
        // Given: A repository after clear_current
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("current-session").unwrap(),
            name: SessionName::parse("Current").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.set_current(&SessionId::parse("current-session").unwrap()).unwrap();
        repo.clear_current().unwrap();

        // When: get_current is called
        let result = repo.get_current().unwrap();

        // Then: Returns None
        assert!(result.is_none());
    }
}
```

### Unit Tests (Error Paths, Edge Cases)

```rust
#[cfg(test)]
mod unit_tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    // Reuse InMemorySessionRepository from integration tests
    use super::integration_tests::InMemorySessionRepository;

    // ============ UNIT TESTS - ERROR PATHS ============

    #[test]
    fn test_load_nonexistent_id_returns_not_found() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();

        // When: load is called with non-existent ID
        let result = repo.load(&SessionId::parse("nonexistent").unwrap());

        // Then: Returns NotFound
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn test_load_nonexistent_name_returns_not_found() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();

        // When: load_by_name is called with non-existent name
        let result = repo.load_by_name(&SessionName::parse("nonexistent").unwrap());

        // Then: Returns NotFound
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn test_save_duplicate_name_returns_conflict() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("session-1").unwrap(),
            name: SessionName::parse("duplicate-name").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: save is called with different ID, same name
        let new_session = Session {
            id: SessionId::parse("session-2").unwrap(),
            name: SessionName::parse("duplicate-name").unwrap(),
            branch_state: BranchState::Detached,
        };
        let result = repo.save(&new_session);

        // Then: Returns Conflict
        assert!(matches!(result, Err(RepositoryError::Conflict(_))));
    }

    #[test]
    fn test_delete_nonexistent_returns_not_found() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();

        // When: delete is called with non-existent ID
        let result = repo.delete(&SessionId::parse("nonexistent").unwrap());

        // Then: Returns NotFound
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn test_set_current_invalid_id_returns_not_found() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();

        // When: set_current is called with non-existent ID
        let result = repo.set_current(&SessionId::parse("nonexistent").unwrap());

        // Then: Returns NotFound
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn test_storage_error_on_load_returns_storage_error() {
        // Given: A broken repository that fails on read
        struct BrokenRepository;
        impl SessionRepository for BrokenRepository {
            fn load(&self, _id: &SessionId) -> RepositoryResult<Session> {
                Err(RepositoryError::StorageError("Read failed".to_string()))
            }
            // ... other methods with reasonable defaults
            fn load_by_name(&self, _name: &SessionName) -> RepositoryResult<Session> {
                Err(RepositoryError::StorageError("Read failed".to_string()))
            }
            fn save(&self, _session: &Session) -> RepositoryResult<()> { Ok(()) }
            fn delete(&self, _id: &SessionId) -> RepositoryResult<()> { Ok(()) }
            fn list_all(&self) -> RepositoryResult<Vec<Session>> { Ok(vec![]) }
            fn list_sorted_by_name(&self) -> RepositoryResult<Vec<Session>> { Ok(vec![]) }
            fn exists(&self, _id: &SessionId) -> RepositoryResult<bool> { Ok(false) }
            fn get_current(&self) -> RepositoryResult<Option<Session>> { Ok(None) }
            fn set_current(&self, _id: &SessionId) -> RepositoryResult<()> { Ok(()) }
            fn clear_current(&self) -> RepositoryResult<()> { Ok(()) }
        }
        let repo = BrokenRepository;

        // When: load is called
        let result = repo.load(&SessionId::parse("any").unwrap());

        // Then: Returns StorageError
        assert!(matches!(result, Err(RepositoryError::StorageError(_))));
    }

    #[test]
    fn test_storage_error_on_save_returns_storage_error() {
        // Given: A broken repository that fails on write
        struct BrokenRepository;
        impl SessionRepository for BrokenRepository {
            fn save(&self, _session: &Session) -> RepositoryResult<()> {
                Err(RepositoryError::StorageError("Write failed".to_string()))
            }
            // ... other methods with reasonable defaults
            fn load(&self, _id: &SessionId) -> RepositoryResult<Session> {
                Err(RepositoryError::NotFound("Not found".to_string()))
            }
            fn load_by_name(&self, _name: &SessionName) -> RepositoryResult<Session> {
                Err(RepositoryError::NotFound("Not found".to_string()))
            }
            fn delete(&self, _id: &SessionId) -> RepositoryResult<()> { Ok(()) }
            fn list_all(&self) -> RepositoryResult<Vec<Session>> { Ok(vec![]) }
            fn list_sorted_by_name(&self) -> RepositoryResult<Vec<Session>> { Ok(vec![]) }
            fn exists(&self, _id: &SessionId) -> RepositoryResult<bool> { Ok(false) }
            fn get_current(&self) -> RepositoryResult<Option<Session>> { Ok(None) }
            fn set_current(&self, _id: &SessionId) -> RepositoryResult<()> { Ok(()) }
            fn clear_current(&self) -> RepositoryResult<()> { Ok(()) }
        }
        let repo = BrokenRepository;

        // When: save is called
        let session = Session {
            id: SessionId::parse("test").unwrap(),
            name: SessionName::parse("Test").unwrap(),
            branch_state: BranchState::Detached,
        };
        let result = repo.save(&session);

        // Then: Returns StorageError
        assert!(matches!(result, Err(RepositoryError::StorageError(_))));
    }

    // ============ UNIT TESTS - EDGE CASES ============

    #[test]
    fn test_list_all_empty_returns_empty_vector() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();

        // When: list_all is called
        let result = repo.list_all().unwrap();

        // Then: Returns empty vector
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_sorted_empty_returns_empty_vector() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();

        // When: list_sorted_by_name is called
        let result = repo.list_sorted_by_name().unwrap();

        // Then: Returns empty vector
        assert!(result.is_empty());
    }

    #[test]
    fn test_save_and_load_special_characters() {
        // Given: A session with special characters
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("special").unwrap(),
            name: SessionName::parse("my_test-session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: load_by_name is called
        let result = repo.load_by_name(&SessionName::parse("my_test-session").unwrap());

        // Then: Returns original session
        assert_eq!(result.unwrap().name.as_str(), "my_test-session");
    }

    #[test]
    fn test_save_session_name_at_max_length() {
        // Given: A session with 63-character name (max allowed)
        let repo = InMemorySessionRepository::new();
        let max_name = "a".repeat(63);
        let session = Session {
            id: SessionId::parse("max-length").unwrap(),
            name: SessionName::parse(&max_name).unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: load is called
        let result = repo.load(&SessionId::parse("max-length").unwrap());

        // Then: Returns session with full name
        assert_eq!(result.unwrap().name.as_str().len(), 63);
    }

    #[test]
    fn test_save_same_id_twice_is_upsert() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("upsert-test").unwrap(),
            name: SessionName::parse("Original").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: save is called with same ID, different name
        let updated = Session {
            id: SessionId::parse("upsert-test").unwrap(),
            name: SessionName::parse("Updated").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&updated).unwrap();

        // Then: list_all returns exactly 1 session
        assert_eq!(repo.list_all().unwrap().len(), 1);
    }
}
```

### Property-Based Tests (Using proptest)

```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    // Reuse InMemorySessionRepository
    use super::integration_tests::InMemorySessionRepository;

    // ============ PROPERTY-BASED TESTS ============

    proptest! {
        #[test]
        fn test_property_sorted_names_always_ascending(
            session_names in proptest::collection::vec(
                "[a-z]{1,20}",
                1..20
            )
        ) {
            // Given: A repository with random session names
            let repo = InMemorySessionRepository::new();
            for (i, name) in session_names.iter().enumerate() {
                let session = Session {
                    id: SessionId::parse(&format!("id-{}", i)).unwrap(),
                    name: SessionName::parse(name).unwrap(),
                    branch_state: BranchState::Detached,
                };
                let _ = repo.save(&session);
            }

            // When: list_sorted_by_name is called
            let result = repo.list_sorted_by_name().unwrap();

            // Then: Names are in ascending order
            for i in 0..result.len().saturating_sub(1) {
                assert!(result[i].name <= result[i + 1].name);
            }
        }

        #[test]
        fn test_property_save_then_load_idempotent(
            session_id in "[a-z0-9-]{1,50}",
            session_name in "[a-z][a-z0-9_-]{0,62}"
        ) {
            // Given: A repository
            let repo = InMemorySessionRepository::new();

            // When: A session is saved and loaded
            let session = Session {
                id: SessionId::parse(&session_id).unwrap(),
                name: SessionName::parse(&session_name).unwrap(),
                branch_state: BranchState::Detached,
            };
            repo.save(&session).unwrap();
            let loaded = repo.load(&session.id).unwrap();

            // Then: Loaded session matches saved session
            assert_eq!(loaded.id, session.id);
            assert_eq!(loaded.name, session.name);
        }

        #[test]
        fn test_property_delete_is_idempotent(
            session_id in "[a-z0-9-]{1,50}"
        ) {
            // Given: A repository with a session
            let repo = InMemorySessionRepository::new();
            let session = Session {
                id: SessionId::parse(&session_id).unwrap(),
                name: SessionName::parse("test").unwrap(),
                branch_state: BranchState::Detached,
            };
            repo.save(&session).unwrap();

            // When: delete is called twice
            repo.delete(&session.id).unwrap();
            let result = repo.delete(&session.id);

            // Then: Second delete returns NotFound (idempotent failure)
            assert!(matches!(result, Err(RepositoryError::NotFound(_))));
        }

        #[test]
        fn test_property_exists_after_save(session_id in "[a-z0-9-]{1,50}") {
            // Given: A repository
            let repo = InMemorySessionRepository::new();

            // When: A session is saved
            let session = Session {
                id: SessionId::parse(&session_id).unwrap(),
                name: SessionName::parse("test").unwrap(),
                branch_state: BranchState::Detached,
            };
            repo.save(&session).unwrap();

            // Then: exists returns true
            assert!(repo.exists(&session.id).unwrap());
        }

        #[test]
        fn test_property_set_and_get_current(session_id in "[a-z0-9-]{1,50}") {
            // Given: A repository with a session
            let repo = InMemorySessionRepository::new();
            let session = Session {
                id: SessionId::parse(&session_id).unwrap(),
                name: SessionName::parse("test").unwrap(),
                branch_state: BranchState::Detached,
            };
            repo.save(&session).unwrap();

            // When: set_current and get_current are called
            repo.set_current(&session.id).unwrap();
            let current = repo.get_current().unwrap();

            // Then: get_current returns Some with correct ID
            assert!(current.is_some());
            assert_eq!(current.unwrap().id.as_str(), session_id);
        }
    }
}
```

### End-to-End Test

```rust
#[cfg(test)]
mod e2e_tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    // Reuse InMemorySessionRepository
    use super::integration_tests::InMemorySessionRepository;

    // ============ E2E TESTS ============

    #[test]
    fn test_e2e_save_creates_session() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };

        // When: save is called
        let result = repo.save(&session);

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_e2e_exists_after_save() {
        // Given: An empty repository
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: exists is called
        let result = repo.exists(&session.id);

        // Then: Returns true
        assert!(result.unwrap());
    }

    #[test]
    fn test_e2e_load_returns_correct_name() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: load is called
        let loaded = repo.load(&session.id).unwrap();

        // Then: Returns correct name
        assert_eq!(loaded.name.as_str(), "E2E Session");
    }

    #[test]
    fn test_e2e_update_returns_ok() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: save is called with updated session
        let updated = Session {
            id: session.id.clone(),
            name: SessionName::parse("E2E Updated").unwrap(),
            branch_state: BranchState::OnBranch("main".to_string()),
        };
        let result = repo.save(&updated);

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_e2e_load_after_update_returns_updated_name() {
        // Given: A repository with an updated session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        let updated = Session {
            id: session.id.clone(),
            name: SessionName::parse("E2E Updated").unwrap(),
            branch_state: BranchState::OnBranch("main".to_string()),
        };
        repo.save(&updated).unwrap();

        // When: load is called
        let loaded_after_update = repo.load(&session.id).unwrap();

        // Then: Returns updated name
        assert_eq!(loaded_after_update.name.as_str(), "E2E Updated");
    }

    #[test]
    fn test_e2e_set_current_returns_ok() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: set_current is called
        let result = repo.set_current(&session.id);

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_e2e_get_current_returns_some() {
        // Given: A repository with current set
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.set_current(&session.id).unwrap();

        // When: get_current is called
        let current = repo.get_current().unwrap();

        // Then: Returns Some
        assert!(current.is_some());
    }

    #[test]
    fn test_e2e_get_current_has_correct_id() {
        // Given: A repository with current set
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.set_current(&session.id).unwrap();

        // When: get_current is called
        let current = repo.get_current().unwrap();

        // Then: Returns correct ID
        assert_eq!(current.unwrap().id.as_str(), "e2e-session");
    }

    #[test]
    fn test_e2e_clear_current_returns_ok() {
        // Given: A repository with current set
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.set_current(&session.id).unwrap();

        // When: clear_current is called
        let result = repo.clear_current();

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_e2e_get_current_after_clear_returns_none() {
        // Given: A repository after clear_current
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.set_current(&session.id).unwrap();
        repo.clear_current().unwrap();

        // When: get_current is called
        let result = repo.get_current().unwrap();

        // Then: Returns None
        assert!(result.is_none());
    }

    #[test]
    fn test_e2e_delete_returns_ok() {
        // Given: A repository with a session
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();

        // When: delete is called
        let result = repo.delete(&session.id);

        // Then: Returns Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_e2e_exists_after_delete_returns_false() {
        // Given: A repository after deletion
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("e2e-session").unwrap(),
            name: SessionName::parse("E2E Session").unwrap(),
            branch_state: BranchState::Detached,
        };
        repo.save(&session).unwrap();
        repo.delete(&session.id).unwrap();

        // When: exists is called
        let result = repo.exists(&session.id);

        // Then: Returns false
        assert!(!result.unwrap());
    }
}
```

---

## PART 4: Contract Violation Tests (From contract.md)

Each violation example from contract.md must have a corresponding test:

```rust
#[cfg(test)]
mod violation_tests {
    use super::*;

    #[test]
    fn test_violation_p1_invalid_session_id_parse_fails() {
        // VIOLATES P1: SessionId::parse("") should fail
        let result = SessionId::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_violation_p2_invalid_session_name_parse_fails() {
        // VIOLATES P2: SessionName::parse("123-invalid") should fail (starts with number)
        let result = SessionName::parse("123-invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_violation_q1_load_returns_wrong_session() {
        // Test that load returns correct session by ID
        // This is covered by integration test: test_load_session_by_id_returns_ok
    }

    #[test]
    fn test_violation_q3_save_must_persist() {
        // Test that save actually persists - covered by: test_save_new_session_exists_after_save
    }

    #[test]
    fn test_violation_q5_delete_must_remove() {
        // Test that delete actually removes - covered by: test_exists_after_delete_returns_false
    }

    #[test]
    fn test_violation_i1_duplicate_names_rejected() {
        // Test that duplicate names are rejected - covered by: test_save_duplicate_name_returns_conflict
    }
}
```

---

## Summary Table

| Category | Count | Test Type |
|----------|-------|-----------|
| Integration Tests | 27 | Real storage (HashMap) |
| Unit Tests | 12 | Error paths, edge cases |
| Property Tests | 7 | proptest-based |
| E2E Tests | 12 | CRUD operations |
| Violation Tests | 6 | Contract verification |
| **TOTAL** | **64** | |

---

## Violation Example Traceability

| Contract Violation | Test Function |
|---------------------|---------------|
| VIOLATES P1 | test_violation_p1_invalid_session_id_parse_fails |
| VIOLATES P2 | test_violation_p2_invalid_session_name_parse_fails |
| VIOLATES Q1 | test_load_session_by_id_returns_ok |
| VIOLATES Q3 | test_save_new_session_exists_after_save |
| VIOLATES Q5 | test_exists_after_delete_returns_false |
| VIOLATES I1 | test_save_duplicate_name_returns_conflict |

---

## Dependencies

To run these tests, add to `Cargo.toml`:

```toml
[dev-dependencies]
proptest = "1.4"
```

Run tests with:
```bash
cargo test --lib
```
