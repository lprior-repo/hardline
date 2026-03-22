//! Error conversion tests.
//!
//! This module contains tests for error conversion functionality.

use std::path::PathBuf;

use crate::domain::{
    aggregates::{bead::BeadError, session::SessionError, workspace::WorkspaceError},
    builders::BuilderError,
    error_conversion::{
        AggregateErrorExt, IdentifierErrorExt,
    },
    identifiers::{IdentifierError, SessionName, WorkspaceName},
    repository::RepositoryError,
    workspace::WorkspaceState,
};

#[cfg(test)]
mod identifier_conversion_tests {
    use super::*;

    #[test]
    fn test_identifier_error_to_session_error() {
        let err = IdentifierError::Empty;
        let session_err: SessionError = err.into();
        assert!(matches!(session_err, SessionError::CannotActivate));

        let err = IdentifierError::TooLong {
            max: 63,
            actual: 100,
        };
        let session_err: SessionError = err.into();
        assert!(matches!(session_err, SessionError::CannotActivate));
    }

    #[test]
    fn test_identifier_error_to_workspace_error() {
        let err = IdentifierError::ContainsPathSeparators;
        let workspace_err: WorkspaceError = err.into();
        assert!(matches!(
            workspace_err,
            WorkspaceError::CannotUse(WorkspaceState::Creating)
        ));
    }

    #[test]
    fn test_identifier_error_to_bead_error() {
        let err = IdentifierError::Empty;
        let bead_err: BeadError = err.into();
        assert!(matches!(bead_err, BeadError::TitleRequired));

        let err = IdentifierError::InvalidFormat {
            details: "test".to_string(),
        };
        let bead_err: BeadError = err.into();
        assert!(matches!(bead_err, BeadError::InvalidTitle(_)));
    }

    #[test]
    fn test_identifier_error_ext() {
        let err = IdentifierError::Empty;
        let session_err = err.to_session_error();
        assert!(matches!(session_err, SessionError::CannotActivate));

        let err = IdentifierError::ContainsPathSeparators;
        let workspace_err = err.to_workspace_error();
        assert!(matches!(workspace_err, WorkspaceError::CannotUse(_)));
    }
}

#[cfg(test)]
mod repository_conversion_tests {
    use super::*;

    #[test]
    fn test_session_error_to_repository_error() {
        let err = SessionError::NameAlreadyExists(SessionName::parse("test").expect("valid name"));
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::Conflict(_)));

        let err = SessionError::WorkspaceNotFound(PathBuf::from("/test"));
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::NotFound(_)));
    }

    #[test]
    fn test_workspace_error_to_repository_error() {
        let err = WorkspaceError::PathNotFound(PathBuf::from("/test"));
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::NotFound(_)));

        let err = WorkspaceError::Removed;
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::NotFound(_)));

        let err =
            WorkspaceError::NameAlreadyExists(WorkspaceName::parse("test").expect("valid name"));
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::Conflict(_)));
    }

    #[test]
    fn test_bead_error_to_repository_error() {
        let err = BeadError::CannotModifyClosed;
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));

        let err = BeadError::TitleRequired;
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));
    }
}

#[cfg(test)]
mod context_conversion_tests {
    use super::*;

    #[test]
    fn test_into_repository_error_with_context() {
        let err = SessionError::NameAlreadyExists(SessionName::parse("test").expect("valid name"));
        let repo_err = err.in_context("session", "create");
        assert!(matches!(repo_err, RepositoryError::Conflict(_)));
        assert!(repo_err.to_string().contains("session"));

        let err = WorkspaceError::PathNotFound(PathBuf::from("/test"));
        let repo_err = err.on_load("workspace");
        assert!(matches!(repo_err, RepositoryError::NotFound(_)));
        assert!(repo_err.to_string().contains("load"));

        let err = BeadError::InvalidTitle("test".to_string());
        let repo_err = err.on_save("bead");
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));
        assert!(repo_err.to_string().contains("save"));
    }
}

#[cfg(test)]
mod builder_conversion_tests {
    use super::*;

    #[test]
    fn test_builder_error_to_session_error() {
        let err = BuilderError::MissingRequired { field: "name" };
        let session_err: SessionError = err.into();
        assert!(matches!(session_err, SessionError::CannotActivate));
    }

    #[test]
    fn test_builder_error_to_bead_error() {
        let err = BuilderError::MissingRequired { field: "title" };
        let bead_err: BeadError = err.into();
        assert!(matches!(bead_err, BeadError::TitleRequired));

        let err = BuilderError::InvalidValue {
            field: "title",
            reason: "too long".to_string(),
        };
        let bead_err: BeadError = err.into();
        assert!(matches!(bead_err, BeadError::InvalidTitle(_)));
    }

    #[test]
    fn test_builder_error_to_repository_error() {
        let err = BuilderError::MissingRequired { field: "id" };
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));

        let err = BuilderError::InvalidValue {
            field: "name",
            reason: "empty".to_string(),
        };
        let repo_err: RepositoryError = err.into();
        assert!(matches!(repo_err, RepositoryError::InvalidInput(_)));
    }
}
