//! Data types for the prune command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the prune command,
//! which removes invalid session records whose workspace directories
//! no longer exist on disk.

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the prune command (parsed from CLI).
#[derive(Debug, Clone, Default)]
pub struct PruneOptions {
    /// Skip confirmation prompt (for scripting/CI use).
    pub yes: bool,
    /// Show what would be removed without actually removing.
    pub dry_run: bool,
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of a prune operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PruneOutput {
    /// Total number of invalid sessions discovered.
    pub invalid_count: usize,
    /// Number of sessions actually removed.
    pub removed_count: usize,
    /// Names of invalid sessions found.
    pub invalid_sessions: Vec<String>,
}

impl PruneOutput {
    /// Create an empty prune output (no invalid sessions found).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            invalid_count: 0,
            removed_count: 0,
            invalid_sessions: Vec::new(),
        }
    }

    /// Create a dry-run output showing what would be pruned.
    #[must_use]
    pub fn dry_run(invalid_sessions: Vec<String>) -> Self {
        let invalid_count = invalid_sessions.len();
        Self {
            invalid_count,
            removed_count: 0,
            invalid_sessions,
        }
    }

    /// Create a completed prune output.
    #[must_use]
    pub fn completed(invalid_sessions: Vec<String>, removed_count: usize) -> Self {
        let invalid_count = invalid_sessions.len();
        Self {
            invalid_count,
            removed_count,
            invalid_sessions,
        }
    }
}

/// Represents a single prunable session record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrunableItem {
    /// Session name.
    pub name: String,
    /// Path that no longer exists.
    pub workspace_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- PruneOptions defaults --

    #[test]
    fn prune_options_default_values() {
        let opts = PruneOptions::default();
        assert!(!opts.yes);
        assert!(!opts.dry_run);
    }

    #[test]
    fn prune_options_with_yes_flag() {
        let opts = PruneOptions {
            yes: true,
            ..PruneOptions::default()
        };
        assert!(opts.yes);
        assert!(!opts.dry_run);
    }

    #[test]
    fn prune_options_with_dry_run() {
        let opts = PruneOptions {
            dry_run: true,
            ..PruneOptions::default()
        };
        assert!(opts.dry_run);
        assert!(!opts.yes);
    }

    #[test]
    fn prune_options_yes_and_dry_run() {
        let opts = PruneOptions {
            yes: true,
            dry_run: true,
        };
        assert!(opts.yes);
        assert!(opts.dry_run);
    }

    // -- PruneOutput construction --

    #[test]
    fn prune_output_empty() {
        let output = PruneOutput::empty();
        assert_eq!(output.invalid_count, 0);
        assert_eq!(output.removed_count, 0);
        assert!(output.invalid_sessions.is_empty());
    }

    #[test]
    fn prune_output_dry_run() {
        let sessions = vec!["session-a".to_string(), "session-b".to_string()];
        let output = PruneOutput::dry_run(sessions.clone());
        assert_eq!(output.invalid_count, 2);
        assert_eq!(output.removed_count, 0);
        assert_eq!(output.invalid_sessions, sessions);
    }

    #[test]
    fn prune_output_completed() {
        let sessions = vec![
            "session-a".to_string(),
            "session-b".to_string(),
            "session-c".to_string(),
        ];
        let output = PruneOutput::completed(sessions.clone(), 3);
        assert_eq!(output.invalid_count, 3);
        assert_eq!(output.removed_count, 3);
        assert_eq!(output.invalid_sessions, sessions);
    }

    #[test]
    fn prune_output_completed_partial_removal() {
        let sessions = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let output = PruneOutput::completed(sessions.clone(), 2);
        assert_eq!(output.invalid_count, 3);
        assert_eq!(output.removed_count, 2);
        assert_eq!(output.invalid_sessions, sessions);
    }

    // -- PruneOutput serialization --

    #[test]
    fn prune_output_serialization_roundtrip() {
        let output = PruneOutput::completed(vec!["s1".to_string(), "s2".to_string()], 2);
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: PruneOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, output);
    }

    #[test]
    fn prune_output_serialization_fields() {
        let output = PruneOutput {
            invalid_count: 5,
            removed_count: 4,
            invalid_sessions: vec!["a".to_string()],
        };
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"invalid_count\":5"));
        assert!(json.contains("\"removed_count\":4"));
        assert!(json.contains("\"invalid_sessions\""));
    }

    #[test]
    fn prune_output_empty_serialization() {
        let output = PruneOutput::empty();
        let json = serde_json::to_string(&output).expect("serialize");
        assert!(json.contains("\"invalid_count\":0"));
        assert!(json.contains("\"removed_count\":0"));
    }

    // -- PrunableItem --

    #[test]
    fn prunable_item_construction() {
        let item = PrunableItem {
            name: "my-session".to_string(),
            workspace_path: "/tmp/missing-workspace".to_string(),
        };
        assert_eq!(item.name, "my-session");
        assert_eq!(item.workspace_path, "/tmp/missing-workspace");
    }

    #[test]
    fn prunable_item_serialization_roundtrip() {
        let item = PrunableItem {
            name: "test-session".to_string(),
            workspace_path: "/path/to/ws".to_string(),
        };
        let json = serde_json::to_string(&item).expect("serialize");
        let deserialized: PrunableItem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, item);
    }

    #[test]
    fn prunable_item_equality() {
        let a = PrunableItem {
            name: "s".to_string(),
            workspace_path: "/p".to_string(),
        };
        let b = PrunableItem {
            name: "s".to_string(),
            workspace_path: "/p".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn prunable_item_inequality() {
        let a = PrunableItem {
            name: "s1".to_string(),
            workspace_path: "/p".to_string(),
        };
        let b = PrunableItem {
            name: "s2".to_string(),
            workspace_path: "/p".to_string(),
        };
        assert_ne!(a, b);
    }
}
