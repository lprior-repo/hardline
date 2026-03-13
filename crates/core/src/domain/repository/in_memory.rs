//! In-memory repository implementations for testing.
//!
//! Provides thread-safe in-memory implementations of repository traits
//! for use in unit tests without requiring real persistence.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::RwLock;

use crate::domain::identifiers::{SessionId, SessionName};

use super::error::RepositoryError;
use super::session::{Session, SessionRepository};
use super::RepositoryResult;

/// In-memory session repository for testing.
///
/// Uses RwLock for concurrent read access and HashMap for O(1) lookups.
/// Enforces invariant I1: duplicate session names are rejected.
pub struct InMemorySessionRepository {
    sessions: RwLock<HashMap<SessionId, Session>>,
    current: RwLock<Option<SessionId>>,
}

impl InMemorySessionRepository {
    /// Create a new in-memory repository.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            current: RwLock::new(None),
        }
    }
}

impl Default for InMemorySessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRepository for InMemorySessionRepository {
    fn load(&self, id: &SessionId) -> RepositoryResult<Session> {
        self.sessions
            .read()
            .map_err(|e| RepositoryError::storage_error(e.to_string()))?
            .get(id)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("session", id))
    }

    fn load_by_name(&self, name: &SessionName) -> RepositoryResult<Session> {
        self.sessions
            .read()
            .map_err(|e| RepositoryError::storage_error(e.to_string()))?
            .values()
            .find(|s| s.name == *name)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("session", name))
    }

    fn save(&self, session: &Session) -> RepositoryResult<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| RepositoryError::storage_error(e.to_string()))?;

        // Check for duplicate name (Invariant I1: no two sessions can have same SessionName)
        let name_exists = sessions
            .values()
            .any(|s| s.name == session.name && s.id != session.id);

        if name_exists {
            return Err(RepositoryError::conflict(format!(
                "Session with name '{}' already exists",
                session.name
            )));
        }

        sessions.insert(session.id.clone(), session.clone());
        Ok(())
    }

    fn delete(&self, id: &SessionId) -> RepositoryResult<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|e| RepositoryError::storage_error(e.to_string()))?;

        sessions
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| RepositoryError::not_found("session", id))
    }

    fn list_all(&self) -> RepositoryResult<Vec<Session>> {
        self.sessions
            .read()
            .map_err(|e| RepositoryError::storage_error(e.to_string()))
            .map(|map| map.values().cloned().collect())
    }

    fn get_current(&self) -> RepositoryResult<Option<Session>> {
        let current = self
            .current
            .read()
            .map_err(|e| RepositoryError::storage_error(e.to_string()))?;

        match &*current {
            Some(id) => Ok(self
                .sessions
                .read()
                .map_err(|e| RepositoryError::storage_error(e.to_string()))?
                .get(id)
                .cloned()),
            None => Ok(None),
        }
    }

    fn set_current(&self, id: &SessionId) -> RepositoryResult<()> {
        // Validate session exists before setting current
        let sessions = self
            .sessions
            .read()
            .map_err(|e| RepositoryError::storage_error(e.to_string()))?;

        if !sessions.contains_key(id) {
            return Err(RepositoryError::not_found("session", id));
        }

        drop(sessions);

        *self
            .current
            .write()
            .map_err(|e| RepositoryError::storage_error(e.to_string()))? = Some(id.clone());

        Ok(())
    }

    fn clear_current(&self) -> RepositoryResult<()> {
        *self
            .current
            .write()
            .map_err(|e| RepositoryError::storage_error(e.to_string()))? = None;
        Ok(())
    }
}

// ============ TESTS ============

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::session::BranchState;

    use super::*;

    #[test]
    fn test_save_and_load() {
        let repo = InMemorySessionRepository::new();
        let session = Session {
            id: SessionId::parse("test-session").unwrap(),
            name: SessionName::parse("test-session").unwrap(),
            branch: BranchState::Detached,
            workspace_path: PathBuf::from("/tmp/test"),
        };

        repo.save(&session).unwrap();
        let loaded = repo.load(&session.id).unwrap();

        assert_eq!(loaded.id.as_str(), "test-session");
        assert_eq!(loaded.name.as_str(), "test-session");
    }

    #[test]
    fn test_save_duplicate_name_returns_conflict() {
        let repo = InMemorySessionRepository::new();

        let session1 = Session {
            id: SessionId::parse("session-1").unwrap(),
            name: SessionName::parse("duplicate-name").unwrap(),
            branch: BranchState::Detached,
            workspace_path: PathBuf::from("/tmp/test"),
        };
        repo.save(&session1).unwrap();

        let session2 = Session {
            id: SessionId::parse("session-2").unwrap(),
            name: SessionName::parse("duplicate-name").unwrap(),
            branch: BranchState::Detached,
            workspace_path: PathBuf::from("/tmp/test2"),
        };
        let result = repo.save(&session2);

        assert!(matches!(result, Err(RepositoryError::Conflict(_))));
    }

    #[test]
    fn test_same_id_twice_is_upsert() {
        let repo = InMemorySessionRepository::new();

        let session1 = Session {
            id: SessionId::parse("upsert-test").unwrap(),
            name: SessionName::parse("Original").unwrap(),
            branch: BranchState::Detached,
            workspace_path: PathBuf::from("/tmp/test"),
        };
        repo.save(&session1).unwrap();

        let session2 = Session {
            id: SessionId::parse("upsert-test").unwrap(),
            name: SessionName::parse("Updated").unwrap(),
            branch: BranchState::Detached,
            workspace_path: PathBuf::from("/tmp/test"),
        };
        repo.save(&session2).unwrap();

        let all = repo.list_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name.as_str(), "Updated");
    }

    #[test]
    fn test_delete_nonexistent_returns_not_found() {
        let repo = InMemorySessionRepository::new();
        let result = repo.delete(&SessionId::parse("nonexistent").unwrap());
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn test_set_current_and_get_current() {
        let repo = InMemorySessionRepository::new();

        let session = Session {
            id: SessionId::parse("current-session").unwrap(),
            name: SessionName::parse("Current").unwrap(),
            branch: BranchState::Detached,
            workspace_path: PathBuf::from("/tmp/test"),
        };
        repo.save(&session).unwrap();

        repo.set_current(&session.id).unwrap();
        let current = repo.get_current().unwrap();

        assert!(current.is_some());
        assert_eq!(current.unwrap().id.as_str(), "current-session");
    }

    #[test]
    fn test_list_sorted_by_name() {
        let repo = InMemorySessionRepository::new();

        for name in ["zebra", "apple", "mango"] {
            let session = Session {
                id: SessionId::parse(name).unwrap(),
                name: SessionName::parse(name).unwrap(),
                branch: BranchState::Detached,
                workspace_path: PathBuf::from("/tmp/test"),
            };
            repo.save(&session).unwrap();
        }

        let sorted = repo.list_sorted_by_name().unwrap();

        assert_eq!(sorted[0].name.as_str(), "apple");
        assert_eq!(sorted[1].name.as_str(), "mango");
        assert_eq!(sorted[2].name.as_str(), "zebra");
    }
}
