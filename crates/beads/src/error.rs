use thiserror::Error;

#[derive(Error, Debug)]
pub enum BeadError {
    #[error("Bead not found: {0}")]
    NotFound(String),

    #[error("Bead already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid bead ID: {0}")]
    InvalidId(String),

    #[error("Invalid title: {0}")]
    InvalidTitle(String),

    #[error("Invalid description: {0}")]
    InvalidDescription(String),

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Dependency cycle detected: {0}")]
    DependencyCycle(String),

    #[error("Bead is blocked by: {0:?}")]
    BlockedBy(Vec<String>),

    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type Result<T> = std::result::Result<T, BeadError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays_id() {
        let err = BeadError::NotFound("abc-123".into());
        let msg = err.to_string();
        assert!(msg.contains("abc-123"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn already_exists_displays_id() {
        let err = BeadError::AlreadyExists("abc-123".into());
        let msg = err.to_string();
        assert!(msg.contains("abc-123"));
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn invalid_id_displays_reason() {
        let err = BeadError::InvalidId("bad chars!".into());
        let msg = err.to_string();
        assert!(msg.contains("bad chars!"));
        assert!(msg.contains("Invalid bead ID"));
    }

    #[test]
    fn invalid_title_displays_reason() {
        let err = BeadError::InvalidTitle("too long".into());
        let msg = err.to_string();
        assert!(msg.contains("too long"));
        assert!(msg.contains("Invalid title"));
    }

    #[test]
    fn invalid_state_transition_displays_from_and_to() {
        let err = BeadError::InvalidStateTransition {
            from: "Open".into(),
            to: "Closed".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Open"));
        assert!(msg.contains("Closed"));
        assert!(msg.contains("Invalid state transition"));
    }

    #[test]
    fn dependency_cycle_displays_reason() {
        let err = BeadError::DependencyCycle("self-loop".into());
        let msg = err.to_string();
        assert!(msg.contains("self-loop"));
        assert!(msg.contains("Dependency cycle"));
    }

    #[test]
    fn blocked_by_displays_blockers() {
        let err = BeadError::BlockedBy(vec!["b1".into(), "b2".into()]);
        let msg = err.to_string();
        assert!(msg.contains("b1"));
        assert!(msg.contains("b2"));
        assert!(msg.contains("blocked by"));
    }

    #[test]
    fn invalid_dependency_displays_reason() {
        let err = BeadError::InvalidDependency("nonexistent".into());
        let msg = err.to_string();
        assert!(msg.contains("nonexistent"));
        assert!(msg.contains("Invalid dependency"));
    }

    #[test]
    fn database_error_displays_reason() {
        let err = BeadError::Database("connection refused".into());
        let msg = err.to_string();
        assert!(msg.contains("connection refused"));
        assert!(msg.contains("Database error"));
    }

    #[test]
    fn serialization_error_displays_reason() {
        let err = BeadError::Serialization("json parse failed".into());
        let msg = err.to_string();
        assert!(msg.contains("json parse failed"));
        assert!(msg.contains("Serialization error"));
    }

    #[test]
    fn result_type_works_with_ok() {
        let result: Result<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn result_type_works_with_err() {
        let result: Result<i32> = Err(BeadError::NotFound("x".into()));
        assert!(result.is_err());
    }

    #[test]
    fn error_is_debug() {
        let err = BeadError::NotFound("test".into());
        let debug_fmt = format!("{:?}", err);
        assert!(debug_fmt.contains("NotFound"));
    }

    #[test]
    fn error_variants_are_exhaustive() {
        // Ensure all variants can be constructed
        let _ = BeadError::NotFound(String::new());
        let _ = BeadError::AlreadyExists(String::new());
        let _ = BeadError::InvalidId(String::new());
        let _ = BeadError::InvalidTitle(String::new());
        let _ = BeadError::InvalidDescription(String::new());
        let _ = BeadError::InvalidStateTransition {
            from: String::new(),
            to: String::new(),
        };
        let _ = BeadError::DependencyCycle(String::new());
        let _ = BeadError::BlockedBy(vec![]);
        let _ = BeadError::InvalidDependency(String::new());
        let _ = BeadError::Database(String::new());
        let _ = BeadError::Serialization(String::new());
    }
}
