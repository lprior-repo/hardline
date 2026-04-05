//! Error types for the isolate domain.

use thiserror::Error;

/// Errors that can occur in the isolate domain.
#[derive(Error, Debug)]
pub enum IsolateError {
    /// An invalid state transition was attempted.
    #[error("invalid state transition: '{from}' -> '{to}'")]
    InvalidTransition {
        from: String,
        to: String,
    },

    /// A workspace state string could not be parsed.
    #[error("invalid workspace state: '{0}'. Valid: created, working, ready, merged, abandoned, conflict")]
    InvalidState(String),

    /// A workspace operation failed.
    #[error("workspace operation failed: {0}")]
    OperationFailed(String),

    /// An invalid workspace ID was provided.
    #[error("invalid workspace id: {0}")]
    InvalidWorkspaceId(String),

    /// An invalid bead ID was provided.
    #[error("invalid bead id: {0}")]
    InvalidBeadId(String),

    /// The workspace guard was dropped without being properly committed.
    #[error("workspace guard dropped without commit: workspace '{workspace_id}' needs cleanup")]
    GuardNotCommitted { workspace_id: String },
}

/// Result alias for isolate operations.
pub type Result<T> = std::result::Result<T, IsolateError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_transition_display() {
        let err = IsolateError::InvalidTransition {
            from: "Created".into(),
            to: "Merged".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Created"));
        assert!(msg.contains("Merged"));
        assert!(msg.contains("invalid state transition"));
    }

    #[test]
    fn invalid_state_display() {
        let err = IsolateError::InvalidState("bogus".into());
        let msg = format!("{err}");
        assert!(msg.contains("bogus"));
        assert!(msg.contains("Valid"));
    }

    #[test]
    fn operation_failed_display() {
        let err = IsolateError::OperationFailed("disk full".into());
        let msg = format!("{err}");
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn error_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<IsolateError>();
    }

    #[test]
    fn result_alias_ok() {
        fn ok_result() -> Result<String> {
            Ok("fine".into())
        }
        assert!(ok_result().is_ok());
    }

    #[test]
    fn result_alias_err() {
        fn err_result() -> Result<String> {
            Err(IsolateError::OperationFailed("fail".into()))
        }
        assert!(err_result().is_err());
    }

    #[test]
    fn invalid_workspace_id_display() {
        let err = IsolateError::InvalidWorkspaceId("empty id".into());
        let msg = format!("{err}");
        assert!(msg.contains("workspace id"));
        assert!(msg.contains("empty id"));
    }

    #[test]
    fn invalid_bead_id_display() {
        let err = IsolateError::InvalidBeadId("empty id".into());
        let msg = format!("{err}");
        assert!(msg.contains("bead id"));
        assert!(msg.contains("empty id"));
    }

    #[test]
    fn guard_not_committed_display() {
        let err = IsolateError::GuardNotCommitted {
            workspace_id: "iso-123".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("guard dropped"));
        assert!(msg.contains("iso-123"));
    }
}
