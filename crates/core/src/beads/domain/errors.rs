//! Domain errors for the beads issue tracker.
//!
//! Structured errors using thiserror for explicit error handling.

use thiserror::Error;

/// Errors that can occur in the beads domain.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("ID cannot be empty")]
    EmptyId,

    #[error("ID must match pattern: {0}")]
    InvalidIdPattern(String),

    #[error("Title cannot be empty")]
    EmptyTitle,

    #[error("Title exceeds maximum length of {max} characters (got {got})")]
    TitleTooLong { max: usize, got: usize },

    #[error("Description exceeds maximum length of {max} characters")]
    DescriptionTooLong { max: usize },

    #[error("Invalid datetime format: {0}")]
    InvalidDatetime(String),

    #[error("Issue not found: {0}")]
    NotFound(String),

    #[error("Duplicate issue ID: {0}")]
    DuplicateId(String),

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Closed issues must have a closed_at timestamp")]
    ClosedWithoutTimestamp,

    #[error("Invalid filter criteria: {0}")]
    InvalidFilter(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_id_display() {
        let err = DomainError::EmptyId;
        let msg = format!("{err}");
        assert!(msg.contains("cannot be empty"));
    }

    #[test]
    fn invalid_id_pattern_display() {
        let err = DomainError::InvalidIdPattern("bd-xyz".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bd-xyz"));
        assert!(msg.contains("pattern"));
    }

    #[test]
    fn empty_title_display() {
        let err = DomainError::EmptyTitle;
        let msg = format!("{err}");
        assert!(msg.contains("Title cannot be empty"));
    }

    #[test]
    fn title_too_long_display() {
        let err = DomainError::TitleTooLong { max: 100, got: 200 };
        let msg = format!("{err}");
        assert!(msg.contains("100"));
        assert!(msg.contains("200"));
        assert!(msg.contains("exceeds"));
    }

    #[test]
    fn description_too_long_display() {
        let err = DomainError::DescriptionTooLong { max: 500 };
        let msg = format!("{err}");
        assert!(msg.contains("500"));
        assert!(msg.contains("Description"));
    }

    #[test]
    fn invalid_datetime_display() {
        let err = DomainError::InvalidDatetime("not-a-date".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("not-a-date"));
        assert!(msg.contains("datetime"));
    }

    #[test]
    fn not_found_display() {
        let err = DomainError::NotFound("bd-abc123".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bd-abc123"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn duplicate_id_display() {
        let err = DomainError::DuplicateId("bd-abc123".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bd-abc123"));
        assert!(msg.contains("Duplicate"));
    }

    #[test]
    fn invalid_state_transition_display() {
        let err = DomainError::InvalidStateTransition {
            from: "Open".to_string(),
            to: "Closed".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Open"));
        assert!(msg.contains("Closed"));
    }

    #[test]
    fn closed_without_timestamp_display() {
        let err = DomainError::ClosedWithoutTimestamp;
        let msg = format!("{err}");
        assert!(msg.contains("closed_at"));
    }

    #[test]
    fn invalid_filter_display() {
        let err = DomainError::InvalidFilter("bad criteria".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("bad criteria"));
        assert!(msg.contains("filter"));
    }

    #[test]
    fn all_variants_are_exhaustive() {
        let _ = DomainError::EmptyId;
        let _ = DomainError::InvalidIdPattern(String::new());
        let _ = DomainError::EmptyTitle;
        let _ = DomainError::TitleTooLong { max: 0, got: 0 };
        let _ = DomainError::DescriptionTooLong { max: 0 };
        let _ = DomainError::InvalidDatetime(String::new());
        let _ = DomainError::NotFound(String::new());
        let _ = DomainError::DuplicateId(String::new());
        let _ = DomainError::InvalidStateTransition {
            from: String::new(),
            to: String::new(),
        };
        let _ = DomainError::ClosedWithoutTimestamp;
        let _ = DomainError::InvalidFilter(String::new());
    }
}
