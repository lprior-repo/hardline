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

/// Execution mode for the prune command.
/// Replaces two booleans (`yes`, `dry_run`) with a single enum,
/// making the three mutually-exclusive modes explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PruneMode {
    /// Interactive: prompt user before removing each item.
    Interactive,
    /// Confirm: skip confirmation prompt (scripting/CI use).
    Confirm,
    /// DryRun: show what would be removed without removing.
    DryRun,
}

/// Options for the prune command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct PruneOptions {
    /// Execution mode (interactive, confirm, or dry-run).
    pub mode: PruneMode,
}

impl Default for PruneOptions {
    fn default() -> Self {
        Self {
            mode: PruneMode::Interactive,
        }
    }
}

impl PruneOptions {
    /// Construct options from CLI boolean flags.
    ///
    /// Maps `--yes` and `--dry-run` flags to the appropriate `PruneMode`.
    /// Dry-run takes precedence over yes when both are set.
    pub const fn from_cli(yes: bool, dry_run: bool) -> Self {
        let mode = if dry_run {
            PruneMode::DryRun
        } else if yes {
            PruneMode::Confirm
        } else {
            PruneMode::Interactive
        };
        Self { mode }
    }
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
    pub const fn empty() -> Self {
        Self {
            invalid_count: 0,
            removed_count: 0,
            invalid_sessions: Vec::new(),
        }
    }

    /// Create a dry-run output showing what would be pruned.
    #[must_use]
    pub const fn dry_run(invalid_sessions: Vec<String>) -> Self {
        let invalid_count = invalid_sessions.len();
        Self {
            invalid_count,
            removed_count: 0,
            invalid_sessions,
        }
    }

    /// Create a completed prune output.
    #[must_use]
    pub const fn completed(invalid_sessions: Vec<String>, removed_count: usize) -> Self {
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

    // -- PruneMode --

    #[test]
    fn prune_mode_interactive_is_default() {
        let opts = PruneOptions::default();
        assert_eq!(opts.mode, PruneMode::Interactive);
    }

    #[test]
    fn prune_mode_confirm() {
        let opts = PruneOptions {
            mode: PruneMode::Confirm,
        };
        assert_eq!(opts.mode, PruneMode::Confirm);
    }

    #[test]
    fn prune_mode_dry_run() {
        let opts = PruneOptions {
            mode: PruneMode::DryRun,
        };
        assert_eq!(opts.mode, PruneMode::DryRun);
    }

    #[test]
    fn prune_mode_equality() {
        assert_eq!(PruneMode::Interactive, PruneMode::Interactive);
        assert_eq!(PruneMode::Confirm, PruneMode::Confirm);
        assert_eq!(PruneMode::DryRun, PruneMode::DryRun);
        assert_ne!(PruneMode::Interactive, PruneMode::Confirm);
        assert_ne!(PruneMode::Confirm, PruneMode::DryRun);
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
