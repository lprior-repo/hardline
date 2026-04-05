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
}
