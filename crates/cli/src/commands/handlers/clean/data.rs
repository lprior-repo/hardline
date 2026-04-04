//! Data types for the clean command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the clean command.

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the clean command (parsed from CLI).
#[derive(Debug, Clone, Default)]
pub struct CleanOptions {
    /// List stale sessions without removing them.
    pub dry_run: bool,

    /// Skip confirmation prompt (non-interactive removal).
    pub force: bool,

    /// Enable verbose output showing session details.
    pub verbose: bool,

    /// Age threshold in seconds for considering a session stale (default: 7200 = 2hr).
    pub age_threshold: Option<u64>,
}

// ============================================================================
// Output Types
// ============================================================================

/// Output from the clean command.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanOutput {
    /// Total number of stale sessions detected.
    pub stale_count: usize,

    /// Number of sessions actually removed.
    pub removed_count: usize,

    /// Names of the stale sessions found.
    pub stale_sessions: Vec<String>,
}

impl CleanOutput {
    /// Create an empty output indicating no stale sessions.
    #[must_use]
    pub fn no_stale() -> Self {
        Self {
            stale_count: 0,
            removed_count: 0,
            stale_sessions: Vec::new(),
        }
    }

    /// Create a dry-run output listing stale sessions without removing them.
    #[must_use]
    pub fn dry_run(stale_sessions: Vec<String>) -> Self {
        let stale_count = stale_sessions.len();
        Self {
            stale_count,
            removed_count: 0,
            stale_sessions,
        }
    }

    /// Create an output for a successful cleanup run.
    #[must_use]
    pub fn cleaned(removed_count: usize, stale_sessions: Vec<String>) -> Self {
        let stale_count = stale_sessions.len();
        Self {
            stale_count,
            removed_count,
            stale_sessions,
        }
    }
}

// ============================================================================
// Pure Helper Functions
// ============================================================================

/// Determine the effective age threshold in seconds.
///
/// Falls back to the default of 7200 seconds (2 hours) if not specified.
#[must_use]
pub fn effective_age_threshold(options: &CleanOptions) -> u64 {
    options.age_threshold.unwrap_or(7200)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- CleanOptions ----

    #[test]
    fn clean_options_default_all_fields_are_default() {
        let opts = CleanOptions::default();
        assert!(!opts.dry_run);
        assert!(!opts.force);
        assert!(!opts.verbose);
        assert!(opts.age_threshold.is_none());
    }

    #[test]
    fn clean_options_with_explicit_fields() {
        let opts = CleanOptions {
            dry_run: true,
            force: true,
            verbose: true,
            age_threshold: Some(3600),
        };
        assert!(opts.dry_run);
        assert!(opts.force);
        assert!(opts.verbose);
        assert_eq!(opts.age_threshold, Some(3600));
    }

    // ---- CleanOutput ----

    #[test]
    fn clean_output_default_is_empty() {
        let output = CleanOutput::default();
        assert_eq!(output.stale_count, 0);
        assert_eq!(output.removed_count, 0);
        assert!(output.stale_sessions.is_empty());
    }

    #[test]
    fn clean_output_no_stale() {
        let output = CleanOutput::no_stale();
        assert_eq!(output.stale_count, 0);
        assert_eq!(output.removed_count, 0);
        assert!(output.stale_sessions.is_empty());
    }

    #[test]
    fn clean_output_dry_run() {
        let sessions = vec!["session-1".to_string(), "session-2".to_string()];
        let output = CleanOutput::dry_run(sessions.clone());
        assert_eq!(output.stale_count, 2);
        assert_eq!(output.removed_count, 0);
        assert_eq!(output.stale_sessions, sessions);
    }

    #[test]
    fn clean_output_cleaned() {
        let sessions = vec![
            "session-1".to_string(),
            "session-2".to_string(),
            "session-3".to_string(),
        ];
        let output = CleanOutput::cleaned(3, sessions.clone());
        assert_eq!(output.stale_count, 3);
        assert_eq!(output.removed_count, 3);
        assert_eq!(output.stale_sessions, sessions);
    }

    #[test]
    fn clean_output_serialization_roundtrip_json() {
        let output = CleanOutput::cleaned(
            2,
            vec!["feature-auth".to_string(), "feature-login".to_string()],
        );
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: CleanOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.stale_count, 2);
        assert_eq!(deserialized.removed_count, 2);
        assert_eq!(deserialized.stale_sessions.len(), 2);
    }

    #[test]
    fn clean_output_serialization_empty_roundtrip() {
        let output = CleanOutput::no_stale();
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: CleanOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.stale_count, 0);
        assert!(deserialized.stale_sessions.is_empty());
    }

    // ---- effective_age_threshold ----

    #[test]
    fn effective_age_threshold_defaults_to_7200() {
        let opts = CleanOptions::default();
        assert_eq!(effective_age_threshold(&opts), 7200);
    }

    #[test]
    fn effective_age_threshold_uses_custom_value() {
        let opts = CleanOptions {
            age_threshold: Some(3600),
            ..Default::default()
        };
        assert_eq!(effective_age_threshold(&opts), 3600);
    }

    #[test]
    fn effective_age_threshold_uses_zero_if_specified() {
        let opts = CleanOptions {
            age_threshold: Some(0),
            ..Default::default()
        };
        assert_eq!(effective_age_threshold(&opts), 0);
    }
}
