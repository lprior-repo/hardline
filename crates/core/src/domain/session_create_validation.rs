//! Session creation validation - Calculations layer
//!
//! Pure validation functions for session creation preconditions.
//!
//! # Calculations Architecture
//!
//! Pure functions: time-independent, referential transparency, no side effects.
//! These functions validate preconditions without performing I/O (except where noted).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::domain::{
    identifiers::AbsolutePath,
    repository::{RepositoryError, SessionRepository},
    session_create_errors::SessionCreateError,
    session_create_types::SessionLimits,
};

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
pub fn validate_name_unique<R>(
    name: &crate::domain::identifiers::SessionName,
    repository: &R,
) -> Result<(), SessionCreateError>
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
