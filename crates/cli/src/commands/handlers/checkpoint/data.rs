//! Data types for the checkpoint command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.

use serde::{Deserialize, Serialize};

/// Options for the checkpoint command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct CheckpointOptions {
    /// Which checkpoint action to perform.
    pub action: CheckpointAction,
    /// Output format.
    pub format: OutputFormat,
}

/// Which checkpoint action to perform.
#[derive(Debug, Clone)]
pub enum CheckpointAction {
    /// Create a new checkpoint with optional description.
    Create {
        /// Optional human-readable description.
        description: Option<String>,
    },
    /// Restore to a specific checkpoint.
    Restore {
        /// The checkpoint ID to restore.
        checkpoint_id: String,
    },
    /// List all checkpoints.
    List,
}

/// Output from the checkpoint command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CheckpointOutput {
    /// A checkpoint was created.
    Created {
        /// Unique checkpoint identifier.
        checkpoint_id: String,
        /// Sessions that were recorded as metadata-only (no file backup).
        metadata_only: Vec<String>,
        /// Optional human-readable description of the checkpoint.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// A checkpoint was restored.
    Restored {
        /// The checkpoint ID that was restored.
        checkpoint_id: String,
    },
    /// List of checkpoints.
    List {
        /// All known checkpoints.
        checkpoints: Vec<CheckpointInfo>,
    },
}

/// Information about a single checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    /// Unique checkpoint identifier.
    pub id: String,
    /// ISO 8601 timestamp when the checkpoint was created.
    pub created_at: String,
    /// Number of sessions included in this checkpoint.
    pub session_count: usize,
    /// Optional human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Generate a checkpoint ID from the current timestamp.
///
/// Pure function: returns a unique string of the form `chk-{hex}`.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn generate_checkpoint_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());

    format!("chk-{millis:x}")
}

// Re-export OutputFormat from scp_core for convenience.
pub use scp_core::OutputFormat;

#[cfg(test)]
mod tests {
    use super::*;

    // -- CheckpointAction tests --

    #[test]
    fn checkpoint_action_create_with_description() {
        let action = CheckpointAction::Create {
            description: Some("test checkpoint".to_string()),
        };
        assert!(
            matches!(action, CheckpointAction::Create { description } if description == Some("test checkpoint".to_string()))
        );
    }

    #[test]
    fn checkpoint_action_create_without_description() {
        let action = CheckpointAction::Create { description: None };
        assert!(
            matches!(action, CheckpointAction::Create { description } if description.is_none())
        );
    }

    #[test]
    fn checkpoint_action_restore() {
        let action = CheckpointAction::Restore {
            checkpoint_id: "chk-abc123".to_string(),
        };
        assert!(
            matches!(action, CheckpointAction::Restore { checkpoint_id } if checkpoint_id == "chk-abc123")
        );
    }

    #[test]
    fn checkpoint_action_list() {
        let action = CheckpointAction::List;
        assert!(matches!(action, CheckpointAction::List));
    }

    // -- CheckpointOutput serialization tests --

    #[test]
    fn checkpoint_output_created_serialization() {
        let output = CheckpointOutput::Created {
            checkpoint_id: "chk-abc123".to_string(),
            metadata_only: vec!["session1".to_string()],
            description: None,
        };
        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.expect("just checked is_ok");
        assert!(json_str.contains("Created"));
        assert!(json_str.contains("chk-abc123"));
    }

    #[test]
    fn checkpoint_output_restored_serialization() {
        let output = CheckpointOutput::Restored {
            checkpoint_id: "chk-def456".to_string(),
        };
        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.expect("just checked is_ok");
        assert!(json_str.contains("Restored"));
        assert!(json_str.contains("chk-def456"));
    }

    #[test]
    fn checkpoint_output_list_empty_serialization() {
        let output = CheckpointOutput::List {
            checkpoints: vec![],
        };
        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.expect("just checked is_ok");
        assert!(json_str.contains("List"));
        assert!(json_str.contains("checkpoints"));
    }

    #[test]
    fn checkpoint_output_list_with_checkpoints_serialization() {
        let output = CheckpointOutput::List {
            checkpoints: vec![
                CheckpointInfo {
                    id: "chk-1".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    session_count: 3,
                    description: Some("first checkpoint".to_string()),
                },
                CheckpointInfo {
                    id: "chk-2".to_string(),
                    created_at: "2024-01-02T00:00:00Z".to_string(),
                    session_count: 5,
                    description: None,
                },
            ],
        };
        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "serialization should succeed");
        let json_str = json.expect("just checked is_ok");
        assert!(json_str.contains("chk-1"));
        assert!(json_str.contains("chk-2"));
        assert!(json_str.contains("first checkpoint"));
    }

    #[test]
    fn checkpoint_output_roundtrip() {
        let output = CheckpointOutput::Created {
            checkpoint_id: "chk-roundtrip".to_string(),
            metadata_only: vec![],
            description: None,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: CheckpointOutput = serde_json::from_str(&json).expect("deserialize");
        assert!(
            matches!(deserialized, CheckpointOutput::Created { checkpoint_id, .. } if checkpoint_id == "chk-roundtrip")
        );
    }

    // -- CheckpointInfo tests --

    #[test]
    fn checkpoint_info_with_description() {
        let info = CheckpointInfo {
            id: "chk-test".to_string(),
            created_at: "2024-06-15T10:30:00Z".to_string(),
            session_count: 10,
            description: Some("Test description".to_string()),
        };
        assert_eq!(info.id, "chk-test");
        assert_eq!(info.created_at, "2024-06-15T10:30:00Z");
        assert_eq!(info.session_count, 10);
        assert_eq!(info.description, Some("Test description".to_string()));
    }

    #[test]
    fn checkpoint_info_without_description() {
        let info = CheckpointInfo {
            id: "chk-test2".to_string(),
            created_at: "2024-06-15T10:30:00Z".to_string(),
            session_count: 0,
            description: None,
        };
        assert!(info.description.is_none());
        assert_eq!(info.session_count, 0);
    }

    #[test]
    fn checkpoint_info_clone() {
        let info = CheckpointInfo {
            id: "chk-orig".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            session_count: 5,
            description: Some("original".to_string()),
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, info.id);
        assert_eq!(cloned.created_at, info.created_at);
        assert_eq!(cloned.session_count, info.session_count);
        assert_eq!(cloned.description, info.description);
    }

    // -- generate_checkpoint_id tests --

    #[test]
    fn generate_checkpoint_id_format() {
        let id = generate_checkpoint_id();
        assert!(id.starts_with("chk-"), "ID should start with 'chk-': {id}");
        let hex_part = &id[4..];
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "ID suffix should be hex: {hex_part}"
        );
    }

    #[test]
    fn generate_checkpoint_id_uniqueness() {
        let id1 = generate_checkpoint_id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = generate_checkpoint_id();
        assert_ne!(id1, id2, "Consecutive IDs should differ");
    }

    // -- CheckpointOptions tests --

    #[test]
    fn checkpoint_options_create() {
        let options = CheckpointOptions {
            action: CheckpointAction::Create {
                description: Some("new checkpoint".to_string()),
            },
            format: OutputFormat::Json,
        };
        assert!(matches!(options.action, CheckpointAction::Create { .. }));
        assert!(options.format.is_json());
    }

    #[test]
    fn checkpoint_options_list() {
        let options = CheckpointOptions {
            action: CheckpointAction::List,
            format: OutputFormat::Json,
        };
        assert!(matches!(options.action, CheckpointAction::List));
        assert!(options.format.is_json());
    }

    #[test]
    fn checkpoint_options_restore() {
        let options = CheckpointOptions {
            action: CheckpointAction::Restore {
                checkpoint_id: "chk-restore-me".to_string(),
            },
            format: OutputFormat::Json,
        };
        assert!(
            matches!(options.action, CheckpointAction::Restore { checkpoint_id } if checkpoint_id == "chk-restore-me")
        );
    }
}
