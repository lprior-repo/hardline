//! Data types for the integrity command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the integrity command,
//! which manages workspace corruption detection, validation, repair,
//! and backup operations.

use serde::{Deserialize, Serialize};

use scp_core::workspace_integrity::{BackupMetadata, ValidationResult};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the integrity command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct IntegrityOptions {
    /// subcommand to run
    pub subcommand: IntegritySubcommand,
}

/// Integrity subcommands
#[derive(Debug, Clone)]
pub enum IntegritySubcommand {
    /// Validate workspace integrity
    Validate {
        /// Workspace name or path
        workspace: String,
    },
    /// Repair corrupted workspace
    Repair {
        /// Workspace name or path
        workspace: String,
        /// Force repair without confirmation
        force: bool,
    },
    /// List available backups
    BackupList,
    /// restore from backup
    BackupRestore {
        /// backup ID to restore
        backup_id: String,
        /// force restore without confirmation
        force: bool,
    },
}

// ============================================================================
// Output Types
// ============================================================================

/// validation response for CLI output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    /// workspace name
    pub workspace: String,
    /// absolute path to workspace
    pub path: String,
    /// Whether workspace is valid
    pub is_valid: bool,
    /// number of issues detected
    pub issue_count: usize,
    /// detailed validation result
    pub validation: ValidationResult,
}

/// repair response for CLI output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairResponse {
    /// workspace name
    pub workspace: String,
    /// whether repair was successful
    pub success: bool,
    /// repair summary message
    pub summary: String,
}

/// backup list response for CLI output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupListResponse {
    /// list of backups
    pub backups: Vec<BackupMetadata>,
    /// total count
    pub count: usize,
}
/// Restore response for CLI output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResponse {
    /// workspace name
    pub workspace: String,
    /// backup ID that was restored
    pub backup_id: String,
    /// whether restore was successful
    pub success: bool,
    /// restore summary message
    pub summary: String,
}

// ============================================================================
// Output format helpers
// ============================================================================

/// Output format for CLI display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityOutputFormat {
    /// Human-readable text output
    Human,
    /// JSON output for automation
    Json,
}

impl IntegrityOutputFormat {
    /// check if this is JSON format
    #[must_use]
    pub const fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }
}

impl From<&str> for IntegrityOutputFormat {
    fn from(s: &str) -> Self {
        match s {
            "json" => Self::Json,
            _ => Self::Human,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- IntegrityOutputFormat --

    #[test]
    fn output_format_human_is_not_json() {
        assert!(!IntegrityOutputFormat::Human.is_json());
    }

    #[test]
    fn output_format_json_is_json() {
        assert!(IntegrityOutputFormat::Json.is_json());
    }
    #[test]
    fn output_format_from_str_json() {
        assert_eq!(IntegrityOutputFormat::from("json"), IntegrityOutputFormat::Json);
    }
    #[test]
    fn output_format_from_str_human() {
        assert_eq!(IntegrityOutputFormat::from("human"), IntegrityOutputFormat::Human);
    }
    #[test]
    fn output_format_from_str_default_is_human() {
        assert_eq!(IntegrityOutputFormat::from("anything"), IntegrityOutputFormat::Human);
    }

    // -- IntegrityOptions construction --

    #[test]
    fn integrity_options_validate() {
        let opts = IntegrityOptions {
            subcommand: IntegritySubcommand::Validate {
                workspace: "my-ws".to_string(),
            },
        };
        match opts.subcommand {
            IntegritySubcommand::Validate { workspace } => assert_eq!(workspace, "my-ws"),
            }
            other => panic!("Expected Validate, got {other:?}"),
        }
    }

    #[test]
    fn integrity_options_repair() {
        let opts = IntegrityOptions {
            subcommand: IntegritySubcommand::Repair {
                workspace: "broken-ws".to_string(),
                force: true,
            },
        };
        match opts.subcommand {
            IntegritySubcommand::Repair { workspace, force } => {
                assert_eq!(workspace, "broken-ws");
                assert!(force);
            }
            other => panic!("Expected Repair, got {other:?}"),
        }
    }

    #[test]
    fn integrity_options_backup_list() {
        let opts = IntegrityOptions {
            subcommand: IntegritySubcommand::BackupList,
        };
        assert!(matches!(opts.subcommand, IntegritySubcommand::BackupList));
    }

    #[test]
    fn integrity_options_backup_restore() {
        let opts = IntegrityOptions {
            subcommand: IntegritySubcommand::BackupRestore {
                backup_id: "backup-123".to_string(),
                force: false,
            },
        };
        match opts.subcommand {
            IntegritySubcommand::BackupRestore { backup_id, force } => {
                assert_eq!(backup_id, "backup-123");
                assert!(!force);
            }
            other => panic!("Expected BackupRestore, got {other:?}"),
        }
    }

    // -- RepairResponse --

    #[test]
    fn repair_response_serialization_roundtrip() {
        let response = RepairResponse {
            workspace: "test-ws".to_string(),
            success: true,
            summary: "Fixed locks".to_string(),
        };
        let json = serde_json::to_string(&response).expect("serialize");
        let deserialized: RepairResponse = serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.workspace, "test-ws");
        assert!(deserialized.success);
        assert_eq!(deserialized.summary, "Fixed locks");
    }

    // -- RestoreResponse --

    #[test]
    fn restore_response_serialization_roundtrip() {
        let response = RestoreResponse {
            workspace: "ws".to_string(),
            backup_id: "bk-1".to_string(),
            success: true,
            summary: "Restored".to_string(),
        };
        let json = serde_json::to_string(&response).expect("serialize");
        let deserialized: RestoreResponse = serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.workspace, "ws");
        assert_eq!(deserialized.backup_id, "bk-1");
        assert!(deserialized.success);
        assert_eq!(deserialized.summary, "Restored");
    }
    // -- BackupListResponse --

    #[test]
    fn backup_list_response_empty() {
        let response = BackupListResponse {
            backups: vec![],
            count: 0,
        };
        assert_eq!(response.count, 0);
        assert!(response.backups.is_empty());
    }
    #[test]
    fn backup_list_response_serialization_roundtrip() {
        let response = BackupListResponse {
            backups: vec![],
            count: 0,
        };
        let json = serde_json::to_string(&response).expect("serialize");
        let deserialized: BackupListResponse = serde_json::from_str(&json).expect("deserialize roundtrip");
        assert_eq!(deserialized.count, 0);
    }
}
