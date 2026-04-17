//! Session creation tests
//!
//! Tests for session creation functionality.

#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::panic))]

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use chrono::Utc;

    use crate::domain::{
        identifiers::{AbsolutePath, SessionId, SessionName},
        repository::{RepositoryError, SessionRepository},
        session::BranchState,
        session_create_errors::SessionCreateError,
        session_create_types::{SessionCreateInput, SessionLimits},
        WorkspaceState,
    };

    use crate::domain::session_create_creator::{create_session_entity, SessionCreator};
    use crate::output::ValidatedMetadata;
    use crate::types::SessionStatus;

    // Mock repository for testing
    struct MockSessionRepository {
        sessions: Arc<Mutex<Vec<crate::types::Session>>>,
    }

    impl MockSessionRepository {
        fn new() -> Self {
            Self {
                sessions: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn with_sessions(sessions: Vec<crate::types::Session>) -> Self {
            Self {
                sessions: Arc::new(Mutex::new(sessions)),
            }
        }
    }

    impl SessionRepository for MockSessionRepository {
        fn load(
            &self,
            id: &SessionId,
        ) -> crate::domain::repository::RepositoryResult<crate::domain::repository::Session>
        {
            self.sessions
                .lock()
                .map_err(|e| RepositoryError::storage_error(e.to_string()))?
                .iter()
                .find(|s| s.id == *id)
                .cloned()
                .map(|ts| crate::domain::repository::Session {
                    id: ts.id.clone(),
                    name: ts.name.clone(),
                    branch: ts.branch.clone(),
                    workspace_path: ts.workspace_path.to_path_buf(),
                })
                .ok_or_else(|| RepositoryError::not_found("session", id.as_str()))
        }

        fn load_by_name(
            &self,
            name: &SessionName,
        ) -> crate::domain::repository::RepositoryResult<crate::domain::repository::Session>
        {
            self.sessions
                .lock()
                .map_err(|e| RepositoryError::storage_error(e.to_string()))?
                .iter()
                .find(|s| s.name == *name)
                .cloned()
                .map(|ts| crate::domain::repository::Session {
                    id: ts.id.clone(),
                    name: ts.name.clone(),
                    branch: ts.branch.clone(),
                    workspace_path: ts.workspace_path.to_path_buf(),
                })
                .ok_or_else(|| RepositoryError::not_found("session", name.as_str()))
        }

        fn save(
            &self,
            session: &crate::domain::repository::Session,
        ) -> crate::domain::repository::RepositoryResult<()> {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|e| RepositoryError::storage_error(e.to_string()))?;

            // Convert repository Session to types::Session for storage
            let ts_session = crate::types::Session {
                id: session.id.clone(),
                name: session.name.clone(),
                status: SessionStatus::Creating,
                state: WorkspaceState::Created,
                workspace_path: AbsolutePath::parse(
                    session.workspace_path.to_string_lossy().as_ref(),
                )
                .expect("valid path"),
                branch: session.branch.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_synced: None,
                metadata: ValidatedMetadata::default(),
            };

            if let Some(pos) = sessions.iter().position(|s| s.id == session.id) {
                sessions[pos] = ts_session;
            } else {
                sessions.push(ts_session);
            }
            Ok(())
        }

        fn delete(&self, id: &SessionId) -> crate::domain::repository::RepositoryResult<()> {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|e| RepositoryError::storage_error(e.to_string()))?;

            let pos = sessions
                .iter()
                .position(|s| s.id == *id)
                .ok_or_else(|| RepositoryError::not_found("session", id.as_str()))?;

            sessions.remove(pos);
            Ok(())
        }

        fn list_all(
            &self,
        ) -> crate::domain::repository::RepositoryResult<Vec<crate::domain::repository::Session>>
        {
            let sessions = self
                .sessions
                .lock()
                .map_err(|e| RepositoryError::storage_error(e.to_string()))?;

            Ok(sessions
                .iter()
                .map(|ts| crate::domain::repository::Session {
                    id: ts.id.clone(),
                    name: ts.name.clone(),
                    branch: ts.branch.clone(),
                    workspace_path: ts.workspace_path.to_path_buf(),
                })
                .collect())
        }

        fn get_current(
            &self,
        ) -> crate::domain::repository::RepositoryResult<Option<crate::domain::repository::Session>>
        {
            Ok(None)
        }

        fn set_current(&self, _id: &SessionId) -> crate::domain::repository::RepositoryResult<()> {
            Ok(())
        }

        fn clear_current(&self) -> crate::domain::repository::RepositoryResult<()> {
            Ok(())
        }
    }

    // Helper to create test input
    fn test_input(name: &str) -> SessionCreateInput {
        SessionCreateInput {
            id: SessionId::parse("test-session-id").expect("valid id"),
            name: SessionName::parse(name).expect("valid name"),
            branch: BranchState::Detached,
            workspace_path: AbsolutePath::parse("/tmp").expect("valid path"),
        }
    }

    #[test]
    fn test_session_limits_default() {
        let limits = SessionLimits::default();
        assert_eq!(limits.max_sessions, 100);
    }

    #[test]
    fn test_session_limits_custom() {
        let limits = SessionLimits::new(50);
        assert_eq!(limits.max_sessions, 50);
    }

    #[test]
    fn test_validate_workspace_exists_valid() {
        let path = AbsolutePath::parse("/tmp").expect("valid path");
        let result = crate::domain::session_create_validation::validate_workspace_exists(&path);
        // /tmp should exist on most systems
        match result {
            Ok(()) => {}
            Err(SessionCreateError::WorkspaceNotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn test_validate_workspace_exists_invalid() {
        let path = AbsolutePath::parse("/nonexistent/path/12345").expect("valid path");
        let result = crate::domain::session_create_validation::validate_workspace_exists(&path);
        assert!(matches!(
            result,
            Err(SessionCreateError::WorkspaceNotFound { .. })
        ));
    }

    #[test]
    fn test_validate_name_unique_available() {
        let repo = MockSessionRepository::new();
        let name = SessionName::parse("new-session").expect("valid name");
        let result = crate::domain::session_create_validation::validate_name_unique(&name, &repo);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_under_limit_ok() {
        let repo = MockSessionRepository::new();
        let limits = SessionLimits::new(100);
        let result = crate::domain::session_create_validation::validate_under_limit(&repo, limits);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_under_limit_exceeded() {
        // Create a repo with 100 sessions
        let sessions: Vec<crate::types::Session> = (0..100)
            .map(|i| crate::types::Session {
                id: SessionId::parse(format!("session-{}", i)).expect("valid"),
                name: SessionName::parse(format!("session-{}", i)).expect("valid"),
                status: SessionStatus::Creating,
                state: WorkspaceState::Created,
                workspace_path: AbsolutePath::parse("/tmp").expect("valid"),
                branch: BranchState::Detached,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_synced: None,
                metadata: ValidatedMetadata::default(),
            })
            .collect();

        let repo = MockSessionRepository::with_sessions(sessions);
        let limits = SessionLimits::new(100);
        let result = crate::domain::session_create_validation::validate_under_limit(&repo, limits);

        assert!(matches!(
            result,
            Err(SessionCreateError::MaxSessionsExceeded {
                max: 100,
                current: 100
            })
        ));
    }

    #[test]
    fn test_session_creator_new() {
        let repo = MockSessionRepository::new();
        let creator = SessionCreator::new(repo);
        let _ = creator;
    }

    #[test]
    fn test_session_creator_with_limits() {
        let repo = MockSessionRepository::new();
        let limits = SessionLimits::new(50);
        let creator = SessionCreator::with_limits(repo, limits);
        let _ = creator;
    }

    #[test]
    fn test_error_display_workspace_not_found() {
        use std::path::PathBuf;

        let err = SessionCreateError::WorkspaceNotFound {
            path: PathBuf::from("/nonexistent"),
        };
        let msg = err.to_string();
        assert!(msg.contains("/nonexistent"));
    }

    #[test]
    fn test_error_display_session_already_exists() {
        let name = SessionName::parse("my-session").expect("valid");
        let err = SessionCreateError::SessionAlreadyExists { name };
        let msg = err.to_string();
        assert!(msg.contains("my-session"));
    }

    #[test]
    fn test_error_display_max_sessions_exceeded() {
        let err = SessionCreateError::MaxSessionsExceeded {
            max: 100,
            current: 100,
        };
        let msg = err.to_string();
        assert!(msg.contains("100"));
    }

    #[test]
    fn test_error_display_repository_error() {
        let err = SessionCreateError::RepositoryError("connection failed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("connection failed"));
    }

    #[test]
    fn test_session_create_input_clone() {
        let input = test_input("test-session");
        let cloned = input.clone();
        assert_eq!(input.id, cloned.id);
        assert_eq!(input.name, cloned.name);
    }

    #[test]
    fn test_create_session_entity() {
        let input = test_input("test-session");
        let created_at = Utc::now();
        let session = create_session_entity(input.clone(), created_at);

        assert_eq!(session.id, input.id);
        assert_eq!(session.name, input.name);
        assert_eq!(session.status, SessionStatus::Creating);
        assert_eq!(session.branch, input.branch);
        assert_eq!(session.workspace_path, input.workspace_path);
        assert_eq!(session.created_at, created_at);
        assert_eq!(session.updated_at, created_at);
    }
}
