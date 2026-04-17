//! Session creation service - Actions layer
//!
//! Session creator service that orchestrates validation and creation.
//!
//! # Actions Architecture
//!
//! Impure, time-dependent, I/O operations. Kept minimal at shell boundary.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};

use crate::domain::{
    repository::SessionRepository,
    session_create_errors::SessionCreateError,
    session_create_types::{SessionCreateInput, SessionCreateOutput, SessionLimits},
};

use super::session_create_validation::{
    validate_name_unique, validate_under_limit, validate_workspace_exists,
};

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
        status: crate::types::SessionStatus::Creating,
        state: crate::WorkspaceState::Created,
        workspace_path: input.workspace_path,
        branch: input.branch,
        created_at,
        updated_at: created_at,
        last_synced: None,
        metadata: crate::output::ValidatedMetadata::default(),
    }
}

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
