//! Checkpoint data types for auto-checkpoint before risky operations.
//!
//! Pure data types with no side effects. These represent the domain concepts
//! of operation risk classification and checkpoint lifecycle.

use serde::{Deserialize, Serialize};

use crate::error::IsolateError;

/// Risk level of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum OperationRisk {
    /// Safe operations (list, status, context) - no checkpoint needed.
    #[default]
    Safe,
    /// Risky operations (batch, spawn, cleanup --force) - checkpoint required.
    Risky,
}

impl OperationRisk {
    /// Returns true if this operation requires a checkpoint.
    #[must_use]
    pub const fn needs_checkpoint(&self) -> bool {
        matches!(self, Self::Risky)
    }
}

/// Lifecycle state of a checkpoint record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointState {
    /// Checkpoint created, operation in progress.
    #[default]
    Pending,
    /// Operation succeeded, checkpoint no longer needed.
    Committed,
    /// Operation failed, checkpoint should be restored.
    NeedsRestore,
}

impl CheckpointState {
    /// Parse from database string representation.
    pub fn from_db(s: &str) -> std::result::Result<Self, IsolateError> {
        match s {
            "pending" => Ok(Self::Pending),
            "committed" => Ok(Self::Committed),
            "needs_restore" => Ok(Self::NeedsRestore),
            _ => Err(IsolateError::OperationFailed(format!(
                "invalid checkpoint state: '{s}'"
            ))),
        }
    }

    /// Convert to database string representation.
    #[must_use]
    pub fn as_db(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Committed => "committed",
            Self::NeedsRestore => "needs_restore",
        }
    }
}

/// A checkpoint record from the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointRecord {
    /// Unique checkpoint identifier (e.g., "auto-1234567890").
    pub id: String,
    /// Whether the operation completed successfully.
    pub committed: bool,
}

impl CheckpointRecord {
    /// Create a new checkpoint record.
    #[must_use]
    pub fn new(id: String, committed: bool) -> Self {
        Self { id, committed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_risk_needs_no_checkpoint() {
        assert!(!OperationRisk::Safe.needs_checkpoint());
    }

    #[test]
    fn risky_needs_checkpoint() {
        assert!(OperationRisk::Risky.needs_checkpoint());
    }

    #[test]
    fn risk_default_is_safe() {
        assert_eq!(OperationRisk::default(), OperationRisk::Safe);
    }

    #[test]
    fn risk_clone_eq() {
        let safe = OperationRisk::Safe;
        assert_eq!(safe.clone(), OperationRisk::Safe);
    }

    #[test]
    fn risk_copy() {
        let safe = OperationRisk::Safe;
        let copied = safe;
        assert_eq!(safe, copied);
    }

    #[test]
    fn risk_debug_format() {
        assert!(!format!("{:?}", OperationRisk::Safe).is_empty());
        assert!(!format!("{:?}", OperationRisk::Risky).is_empty());
    }

    #[test]
    fn checkpoint_state_from_db_valid() {
        assert_eq!(
            CheckpointState::from_db("pending").unwrap(),
            CheckpointState::Pending
        );
        assert_eq!(
            CheckpointState::from_db("committed").unwrap(),
            CheckpointState::Committed
        );
        assert_eq!(
            CheckpointState::from_db("needs_restore").unwrap(),
            CheckpointState::NeedsRestore
        );
    }

    #[test]
    fn checkpoint_state_from_db_invalid() {
        assert!(CheckpointState::from_db("bogus").is_err());
    }

    #[test]
    fn checkpoint_state_as_db_roundtrip() {
        for state in [
            CheckpointState::Pending,
            CheckpointState::Committed,
            CheckpointState::NeedsRestore,
        ] {
            assert_eq!(CheckpointState::from_db(state.as_db()).unwrap(), state);
        }
    }

    #[test]
    fn checkpoint_state_default_is_pending() {
        assert_eq!(CheckpointState::default(), CheckpointState::Pending);
    }

    #[test]
    fn checkpoint_record_new() {
        let record = CheckpointRecord::new("auto-123".to_string(), false);
        assert_eq!(record.id, "auto-123");
        assert!(!record.committed);
    }

    #[test]
    fn checkpoint_record_equality() {
        let a = CheckpointRecord::new("auto-1".to_string(), true);
        let b = CheckpointRecord::new("auto-1".to_string(), true);
        assert_eq!(a, b);
    }

    #[test]
    fn operation_risk_roundtrip() {
        for risk in [OperationRisk::Safe, OperationRisk::Risky] {
            let json = serde_json::to_string(&risk).unwrap();
            let parsed: OperationRisk = serde_json::from_str(&json).unwrap();
            assert_eq!(risk, parsed);
        }
    }

    #[test]
    fn operation_risk_default_is_safe() {
        let json = serde_json::to_string(&OperationRisk::default()).unwrap();
        let parsed: OperationRisk = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, OperationRisk::Safe);
    }

    #[test]
    fn checkpoint_state_all_variants_serialize_snake_case() {
        let pending_json = serde_json::to_string(&CheckpointState::Pending).unwrap();
        assert_eq!(pending_json, "\"pending\"");

        let committed_json = serde_json::to_string(&CheckpointState::Committed).unwrap();
        assert_eq!(committed_json, "\"committed\"");

        let needs_restore_json = serde_json::to_string(&CheckpointState::NeedsRestore).unwrap();
        assert_eq!(needs_restore_json, "\"needs_restore\"");
    }

    #[test]
    fn checkpoint_state_all_variants_roundtrip() {
        for state in [CheckpointState::Pending, CheckpointState::Committed, CheckpointState::NeedsRestore] {
            let json = serde_json::to_string(&state).unwrap();
            let parsed: CheckpointState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, parsed);
        }
    }

}
