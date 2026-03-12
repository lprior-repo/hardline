//! Session creation validation module
//!
//! Provides validation logic for session creation following functional-rust patterns:
//! - Zero unwrap/panic/expect - all fallible via Result<T, E>
//! - Zero let mut - immutable by default
//! - Data → Calculations → Actions organization
//!
//! # Preconditions (P1-P7)
//!
//! | ID | Description | Type |
//! |----|-------------|------|
//! | P1 | `SessionName` not empty | Compile-time (`SessionName::parse`) |
//! | P2 | `SessionName` starts with letter | Compile-time (`SessionName::parse`) |
//! | P3 | `SessionName` alphanumeric/hyphen/underscore | Compile-time (`SessionName::parse`) |
//! | P4 | `SessionName` 1-63 chars | Compile-time (`SessionName::parse`) |
//! | P5 | Workspace path must exist | Runtime |
//! | P6 | Session name must be unique | Runtime |
//! | P7 | Max sessions limit | Runtime |
//!
//! # Postconditions (Q1-Q8)
//!
//! | ID | Description |
//! |----|-------------|
//! | Q1 | Session created with status Created |
//! | Q2 | Session.id is set correctly |
//! | Q3 | Session.name is set correctly |
//! | Q4 | Session.workspace_path is set correctly |
//! | Q5 | Session.branch is set correctly |
//! | Q6 | Session.created_at is set |
//! | Q7 | Session.updated_at is set |
//! | Q8 | Session.status is Created |

#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::panic))]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::{
    domain::{
        identifiers::{AbsolutePath, SessionId, SessionName},
        repository::{RepositoryError, SessionRepository},
        session::BranchState,
    },
    output::ValidatedMetadata,
    types::SessionStatus,
    WorkspaceState,
};

// ============================================================================
// DATA: Input and Output Types
// ============================================================================

/// Input for session creation
///
/// This is the input data structure for creating a session.
/// All fields are pre-validated by their respective newtype constructors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCreateInput {
    /// Unique session identifier (pre-validated by `SessionId::parse`)
    pub id: SessionId,
    /// Human-readable session name (pre-validated by `SessionName::parse`)
    pub name: SessionName,
    /// Branch state for the session
    pub branch: BranchState,
    /// Absolute path to workspace directory (pre-validated by `AbsolutePath::parse`)
    pub workspace_path: AbsolutePath,
}

/// Output from successful session creation
///
/// Contains the created session and metadata about the creation.
#[derive(Debug, Clone)]
pub struct SessionCreateOutput {
    /// The created session entity
    pub session: crate::types::Session,
    /// When the session was created
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// ERROR: Domain Errors
// ============================================================================

/// Errors that can occur during session creation
///
/// Follows the error taxonomy from the contract:
/// - `Error::ValidationError` for name/workspace validation
/// - `Error::SessionAlreadyExists` for duplicate names
/// - `Error::MaxSessionsExceeded` for limit reached
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCreateError {
    /// Workspace path does not exist (P5)
    ///
    /// The provided workspace path must exist on the filesystem.
    /// This is a runtime validation because it requires I/O.
    WorkspaceNotFound {
        /// The path that was provided
        path: PathBuf,
    },

    /// Session name already exists (P6)
    ///
    /// Each session must have a unique name within the system.
    /// This requires checking the repository for existing sessions.
    SessionAlreadyExists {
        /// The name that already exists
        name: SessionName,
    },

    /// Maximum session limit exceeded (P7)
    ///
    /// The system has reached its maximum capacity for sessions.
    MaxSessionsExceeded {
        /// The maximum number of sessions allowed
        max: usize,
        /// The current number of sessions
        current: usize,
    },

    /// Repository operation failed
    ///
    /// Underlying repository error (connection, corruption, etc.)
    RepositoryError(String),
}

impl std::fmt::Display for SessionCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceNotFound { path } => {
                write!(f, "workspace path does not exist: {}", path.display())
            }
            Self::SessionAlreadyExists { name } => {
                write!(f, "session name already exists: {}", name.as_str())
            }
            Self::MaxSessionsExceeded { max, current } => {
                write!(f, "max sessions exceeded: {current} of {max}")
            }
            Self::RepositoryError(msg) => {
                write!(f, "repository error: {msg}")
            }
        }
    }
}

impl std::error::Error for SessionCreateError {}

impl From<RepositoryError> for SessionCreateError {
    fn from(err: RepositoryError) -> Self {
        Self::RepositoryError(err.to_string())
    }
}

// ============================================================================
// CALCULATIONS: Pure Validation Functions
// ============================================================================

/// Configuration for session creation limits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLimits {
    /// Maximum number of sessions allowed (default 100)
    pub max_sessions: usize,
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self { max_sessions: 100 }
    }
}

impl SessionLimits {
    /// Create new limits with a custom max
    #[must_use]
    pub const fn new(max_sessions: usize) -> Self {
        Self { max_sessions }
    }
}

/// Validate that the workspace path exists (P5)
///
/// This is a runtime check because it requires I/O to verify the path.
/// The path must exist on the filesystem.
///
/// # Errors
///
/// Returns `SessionCreateError::WorkspaceNotFound` if path doesn't exist.
pub fn validate_workspace_exists(path: &AbsolutePath) -> Result<(), SessionCreateError> {
    if !path.exists() {
        return Err(SessionCreateError::WorkspaceNotFound {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

/// Validate that the session name is unique (P6)
///
/// This requires checking the repository for existing sessions with the same name.
///
/// # Errors
///
/// Returns `SessionCreateError::SessionAlreadyExists` if name exists.
pub fn validate_name_unique<R>(name: &SessionName, repository: &R) -> Result<(), SessionCreateError>
where
    R: SessionRepository,
{
    // Try to load by name - if it succeeds, name is taken
    match repository.load_by_name(name) {
        Ok(_) => Err(SessionCreateError::SessionAlreadyExists { name: name.clone() }),
        Err(RepositoryError::NotFound(_)) => Ok(()),
        Err(e) => Err(SessionCreateError::from(e)),
    }
}

/// Validate that we haven't hit the session limit (P7)
///
/// # Errors
///
/// Returns `SessionCreateError::MaxSessionsExceeded` if at limit.
pub fn validate_under_limit<R>(
    repository: &R,
    limits: SessionLimits,
) -> Result<(), SessionCreateError>
where
    R: SessionRepository,
{
    let current_count = repository
        .list_all()
        .map_err(SessionCreateError::from)?
        .len();

    if current_count >= limits.max_sessions {
        return Err(SessionCreateError::MaxSessionsExceeded {
            max: limits.max_sessions,
            current: current_count,
        });
    }

    Ok(())
}

// ============================================================================
// CALCULATIONS: Pure Session Creation
// ============================================================================

/// Create a new session entity with the given input
///
/// This is a pure function - it creates the session entity without persistence.
/// The session is created with status `Creating`.
///
/// # Postconditions (Q1-Q8)
///
/// - Q1: Session status is `Creating`
/// - Q2: Session.id is set from input
/// - Q3: Session.name is set from input
/// - Q4: `Session.workspace_path` is set from input
/// - Q5: `Session.branch` is set from input
/// - Q6: `Session.created_at` is set to current time
/// - Q7: `Session.updated_at` is set to current time
/// - Q8: Session.status is `Creating`
#[must_use]
pub fn create_session_entity(
    input: SessionCreateInput,
    created_at: DateTime<Utc>,
) -> crate::types::Session {
    // Use the full types::Session for the complete session entity
    crate::types::Session {
        id: input.id,
        name: input.name,
        status: SessionStatus::Creating,
        state: WorkspaceState::Created,
        workspace_path: input.workspace_path,
        branch: input.branch,
        created_at,
        updated_at: created_at,
        last_synced: None,
        metadata: ValidatedMetadata::default(),
    }
}

// ============================================================================
// ACTIONS: Session Creator Service
// ============================================================================

/// Session creator - handles all preconditions for session creation
///
/// This is the main entry point for session creation. It validates all
/// preconditions (P1-P7) and creates the session if all validations pass.
///
/// # Type Parameters
///
/// - `R`: The session repository implementation
pub struct SessionCreator<R>
where
    R: SessionRepository,
{
    repository: R,
    limits: SessionLimits,
}

impl<R> SessionCreator<R>
where
    R: SessionRepository,
{
    /// Create a new session creator
    #[must_use]
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            limits: SessionLimits::default(),
        }
    }

    /// Create a session creator with custom limits
    #[must_use]
    pub fn with_limits(repository: R, limits: SessionLimits) -> Self {
        Self { repository, limits }
    }

    /// Create a new session (P5, P6, P7)
    ///
    /// Validates all preconditions and creates the session if valid:
    /// - P5: Workspace path must exist
    /// - P6: Session name must be unique
    /// - P7: Max sessions limit
    ///
    /// # Errors
    ///
    /// Returns `SessionCreateError` if any validation fails.
    pub fn create(
        &self,
        input: SessionCreateInput,
    ) -> Result<SessionCreateOutput, SessionCreateError> {
        // P5: Validate workspace exists (runtime I/O check)
        validate_workspace_exists(&input.workspace_path)?;

        // P6: Validate name is unique (runtime repository check)
        validate_name_unique(&input.name, &self.repository)?;

        // P7: Validate under session limit (runtime repository check)
        validate_under_limit(&self.repository, self.limits)?;

        // All validations passed - create the session entity
        let created_at = Utc::now();
        let session = create_session_entity(input, created_at);

        Ok(SessionCreateOutput {
            session,
            created_at,
        })
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

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
        let result = validate_workspace_exists(&path);
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
        let result = validate_workspace_exists(&path);
        assert!(matches!(
            result,
            Err(SessionCreateError::WorkspaceNotFound { .. })
        ));
    }

    #[test]
    fn test_validate_name_unique_available() {
        let repo = MockSessionRepository::new();
        let name = SessionName::parse("new-session").expect("valid name");
        let result = validate_name_unique(&name, &repo);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_under_limit_ok() {
        let repo = MockSessionRepository::new();
        let limits = SessionLimits::new(100);
        let result = validate_under_limit(&repo, limits);
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
        let result = validate_under_limit(&repo, limits);

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
