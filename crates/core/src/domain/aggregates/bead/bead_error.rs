//! Bead error types.

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::beads::{DomainError, IssueState};

/// Bead state matches the beads `IssueState`.
///
/// Closed state requires a timestamp inline.
pub type BeadState = IssueState;

/// Errors that can occur during bead operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BeadError {
    /// Invalid title
    #[error("invalid title: {0}")]
    InvalidTitle(String),

    /// Invalid description
    #[error("invalid description: {0}")]
    InvalidDescription(String),

    /// Invalid state transition
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: BeadState, to: BeadState },

    /// Cannot modify closed bead
    #[error("cannot modify closed bead")]
    CannotModifyClosed,

    /// Timestamps are not monotonic
    #[error("timestamps must be monotonic: updated_at ({updated_at}) < created_at ({created_at})")]
    NonMonotonicTimestamps {
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    },

    /// Title is required
    #[error("title is required")]
    TitleRequired,

    /// Domain error from beads module
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_title_display() {
        let err = BeadError::InvalidTitle("too short".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("too short"));
        assert!(msg.contains("invalid title"));
    }

    #[test]
    fn invalid_description_display() {
        let err = BeadError::InvalidDescription("too long".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("too long"));
        assert!(msg.contains("invalid description"));
    }

    #[test]
    fn invalid_state_transition_display() {
        let err = BeadError::InvalidStateTransition {
            from: BeadState::Open,
            to: BeadState::Closed {
                closed_at: chrono::Utc::now(),
            },
        };
        let msg = format!("{err}");
        assert!(msg.contains("Open"));
    }

    #[test]
    fn cannot_modify_closed_display() {
        let err = BeadError::CannotModifyClosed;
        let msg = format!("{err}");
        assert!(msg.contains("cannot modify closed"));
    }

    #[test]
    fn non_monotonic_timestamps_display() {
        let err = BeadError::NonMonotonicTimestamps {
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now() - chrono::Duration::days(1),
        };
        let msg = format!("{err}");
        assert!(msg.contains("monotonic"));
    }

    #[test]
    fn title_required_display() {
        let err = BeadError::TitleRequired;
        let msg = format!("{err}");
        assert!(msg.contains("title is required"));
    }

    #[test]
    fn domain_error_display() {
        let inner = DomainError::NotFound("bd-123".to_string());
        let err = BeadError::Domain(inner);
        let msg = format!("{err}");
        assert!(msg.contains("bd-123"));
        assert!(msg.contains("domain error"));
    }

    #[test]
    fn all_variants_are_exhaustive() {
        let _ = BeadError::InvalidTitle(String::new());
        let _ = BeadError::InvalidDescription(String::new());
        let _ = BeadError::InvalidStateTransition {
            from: BeadState::Open,
            to: BeadState::Closed {
                closed_at: chrono::Utc::now(),
            },
        };
        let _ = BeadError::CannotModifyClosed;
        let _ = BeadError::NonMonotonicTimestamps {
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let _ = BeadError::TitleRequired;
        let _ = BeadError::Domain(DomainError::EmptyId);
    }
}
