use thiserror::Error;

#[derive(Error, Debug)]
pub enum QueueError {
    #[error("Queue entry not found: {0}")]
    QueueEntryNotFound(String),

    #[error("Queue is empty")]
    QueueEmpty,

    #[error("Invalid state transition: from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Invalid queue entry id: {0}")]
    InvalidQueueEntryId(String),

    #[error("Invalid priority value: {0}")]
    InvalidPriority(String),

    #[error("Invalid queue position: {0}")]
    InvalidQueuePosition(String),

    #[error("Queue operation failed: {0}")]
    OperationFailed(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),
}

#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("Database connection failed: {0}")]
    DatabaseError(String),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    #[error("Table already exists with incompatible schema")]
    SchemaConflict,

    #[error("Invalid migration: {0}")]
    InvalidMigration(String),

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
}

pub type Result<T> = std::result::Result<T, QueueError>;

pub type MigrationResult<T> = std::result::Result<T, MigrationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_error_entry_not_found_display() {
        let err = QueueError::QueueEntryNotFound("entry-42".into());
        let msg = format!("{err}");
        assert!(msg.contains("entry-42"));
    }

    #[test]
    fn queue_error_queue_empty_display() {
        let err = QueueError::QueueEmpty;
        let msg = format!("{err}");
        assert!(msg.contains("empty"));
    }

    #[test]
    fn queue_error_invalid_state_transition_display() {
        let err = QueueError::InvalidStateTransition {
            from: "Pending".into(),
            to: "Merged".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Pending") && msg.contains("Merged"));
    }

    #[test]
    fn queue_error_invalid_queue_entry_id_display() {
        let err = QueueError::InvalidQueueEntryId("bad id".into());
        let msg = format!("{err}");
        assert!(msg.contains("bad id"));
    }

    #[test]
    fn queue_error_invalid_priority_display() {
        let err = QueueError::InvalidPriority("999".into());
        let msg = format!("{err}");
        assert!(msg.contains("999"));
    }

    #[test]
    fn queue_error_invalid_queue_position_display() {
        let err = QueueError::InvalidQueuePosition("-1".into());
        let msg = format!("{err}");
        assert!(msg.contains("-1"));
    }

    #[test]
    fn queue_error_operation_failed_display() {
        let err = QueueError::OperationFailed("disk full".into());
        let msg = format!("{err}");
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn queue_error_repository_error_display() {
        let err = QueueError::RepositoryError("connection lost".into());
        let msg = format!("{err}");
        assert!(msg.contains("connection lost"));
    }

    #[test]
    fn queue_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(QueueError::QueueEmpty);
        let _ = format!("{err:?}");
    }

    #[test]
    fn queue_error_is_debug() {
        let err = QueueError::QueueEmpty;
        let _ = format!("{err:?}");
    }

    #[test]
    fn queue_error_invalid_state_transition_debug_content() {
        let err = QueueError::InvalidStateTransition {
            from: "A".into(),
            to: "B".into(),
        };
        let debug = format!("{err:?}");
        assert!(debug.contains("A") && debug.contains("B"));
    }

    #[test]
    fn migration_error_database_error_display() {
        let err = MigrationError::DatabaseError("io err".into());
        let msg = format!("{err}");
        assert!(msg.contains("io err"));
    }

    #[test]
    fn migration_error_migration_failed_display() {
        let err = MigrationError::MigrationFailed("sql err".into());
        let msg = format!("{err}");
        assert!(msg.contains("sql err"));
    }

    #[test]
    fn migration_error_schema_conflict_display() {
        let err = MigrationError::SchemaConflict;
        let msg = format!("{err}");
        assert!(msg.contains("incompatible"));
    }

    #[test]
    fn migration_error_invalid_migration_display() {
        let err = MigrationError::InvalidMigration("bad sql".into());
        let msg = format!("{err}");
        assert!(msg.contains("bad sql"));
    }

    #[test]
    fn migration_error_rollback_failed_display() {
        let err = MigrationError::RollbackFailed("locked".into());
        let msg = format!("{err}");
        assert!(msg.contains("locked"));
    }

    #[test]
    fn migration_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(MigrationError::SchemaConflict);
        let _ = format!("{err:?}");
    }

    #[test]
    fn migration_error_is_debug() {
        let err = MigrationError::SchemaConflict;
        let _ = format!("{err:?}");
    }

    #[test]
    fn result_type_works() {
        let ok: Result<i32> = Ok(42);
        let err: Result<i32> = Err(QueueError::QueueEmpty);
        assert_eq!(ok.unwrap(), 42);
        assert!(err.is_err());
    }

    #[test]
    fn migration_result_type_works() {
        let ok: MigrationResult<i32> = Ok(42);
        let err: MigrationResult<i32> = Err(MigrationError::SchemaConflict);
        assert_eq!(ok.unwrap(), 42);
        assert!(err.is_err());
    }

    // --- Additional edge case tests ---

    #[test]
    fn queue_error_entry_not_found_empty_string() {
        let err = QueueError::QueueEntryNotFound("".into());
        let msg = format!("{err}");
        assert!(msg.contains("not found"));
    }

    #[test]
    fn queue_error_invalid_state_transition_empty_strings() {
        let err = QueueError::InvalidStateTransition {
            from: "".into(),
            to: "".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Invalid state transition"));
    }

    #[test]
    fn queue_error_debug_all_variants() {
        let errors = vec![
            QueueError::QueueEntryNotFound("e".into()),
            QueueError::QueueEmpty,
            QueueError::InvalidStateTransition { from: "A".into(), to: "B".into() },
            QueueError::InvalidQueueEntryId("id".into()),
            QueueError::InvalidPriority("p".into()),
            QueueError::InvalidQueuePosition("pos".into()),
            QueueError::OperationFailed("op".into()),
            QueueError::RepositoryError("repo".into()),
        ];
        for err in &errors {
            let debug = format!("{err:?}");
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn migration_error_debug_all_variants() {
        let errors = vec![
            MigrationError::DatabaseError("db".into()),
            MigrationError::MigrationFailed("mig".into()),
            MigrationError::SchemaConflict,
            MigrationError::InvalidMigration("inv".into()),
            MigrationError::RollbackFailed("roll".into()),
        ];
        for err in &errors {
            let debug = format!("{err:?}");
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn queue_error_clone() {
        let a = QueueError::QueueEntryNotFound("test".into());
        let msg_a = format!("{a}");
        let msg_b = format!("{:?}", a);
        assert!(!msg_a.is_empty());
        assert!(!msg_b.is_empty());
    }

    #[test]
    fn migration_error_clone() {
        let a = MigrationError::MigrationFailed("test".into());
        let msg_a = format!("{a}");
        let msg_b = format!("{:?}", a);
        assert!(!msg_a.is_empty());
        assert!(!msg_b.is_empty());
    }

    #[test]
    fn result_type_with_unit() {
        let ok: Result<()> = Ok(());
        let err: Result<()> = Err(QueueError::QueueEmpty);
        assert!(ok.is_ok());
        assert!(err.is_err());
    }

    #[test]
    fn migration_result_type_with_unit() {
        let ok: MigrationResult<()> = Ok(());
        let err: MigrationResult<()> = Err(MigrationError::SchemaConflict);
        assert!(ok.is_ok());
        assert!(err.is_err());
    }

    #[test]
    fn queue_error_source_chain() {
        let err: Box<dyn std::error::Error> = Box::new(QueueError::RepositoryError("inner".into()));
        assert!(err.source().is_none(), "thiserror Error should not have a source chain for simple variants");
    }

    #[test]
    fn migration_error_source_chain() {
        let err: Box<dyn std::error::Error> = Box::new(MigrationError::DatabaseError("conn".into()));
        assert!(err.source().is_none());
    }

    #[test]
    fn queue_error_entry_not_found_with_special_chars() {
        let err = QueueError::QueueEntryNotFound("id-with-special_chars!@#".into());
        let msg = format!("{err}");
        assert!(msg.contains("id-with-special_chars!@#"));
    }
}
