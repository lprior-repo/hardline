//! Core domain types for SCP (Source Control Plane)
//!
//! Provides session, workspace, and change tracking types with zero-unwrap patterns.
//!
//! Split into smaller modules at crate root:
//! - type_session_name: Session name validation
//! - type_session_id: Session ID validation
//! - type_session_path: Absolute path validation
//! - type_branch_state: Branch state representation
//! - type_metadata: Validated metadata storage
//! - type_session_status: Session status state machine
//! - type_session: Session aggregate
//! - type_file_change: File change tracking
//! - type_beads_issue: Beads issue types

// Re-export all types for convenience
pub use crate::type_beads_issue::{BeadsIssue, BeadsSummary, IssueStatus};
pub use crate::type_branch_state::BranchState;
pub use crate::type_file_change::{ChangesSummary, DiffSummary, FileChange, FileDiffStat, FileStatus};
pub use crate::type_metadata::ValidatedMetadata;
pub use crate::type_session::Session;
pub use crate::type_session_id::SessionId;
pub use crate::type_session_name::SessionName;
pub use crate::type_session_path::AbsolutePath;
pub use crate::type_session_status::{Operation, SessionStatus};

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use proptest::prelude::*;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_session_status_transitions() {
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Creating.can_transition_to(SessionStatus::Failed));
        assert!(!SessionStatus::Creating.can_transition_to(SessionStatus::Paused));

        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Paused));
        assert!(SessionStatus::Active.can_transition_to(SessionStatus::Completed));
        assert!(!SessionStatus::Active.can_transition_to(SessionStatus::Creating));

        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Active));
        assert!(SessionStatus::Paused.can_transition_to(SessionStatus::Completed));
    }

    #[test]
    fn test_session_status_allowed_operations() {
        assert!(SessionStatus::Creating.allowed_operations().is_empty());
        assert!(SessionStatus::Active.allows_operation(Operation::Status));
        assert!(SessionStatus::Active.allows_operation(Operation::Focus));
        assert!(SessionStatus::Paused.allows_operation(Operation::Remove));
        assert!(!SessionStatus::Creating.allows_operation(Operation::Status));
    }

    #[test]
    fn test_session_name_rejects_invalid() {
        assert!(SessionName::parse("invalid name").is_err());
        assert!(SessionName::parse("123-start-with-number").is_err());
        assert!(SessionName::parse("").is_err());
        assert!(SessionName::parse(&"x".repeat(65)).is_err());
    }

    #[test]
    fn test_session_name_accepts_valid() {
        assert!(SessionName::parse("valid-name").is_ok());
        assert!(SessionName::parse("Feature_Auth").is_ok());
        assert!(SessionName::parse("a").is_ok());
    }

    #[test]
    fn test_absolute_path_rejects_relative() {
        assert!(AbsolutePath::parse("relative/path").is_err());
    }

    #[test]
    fn test_session_validate_timestamps() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(60);

        let session = Session {
            id: SessionId::parse("id123").expect("valid id"),
            name: SessionName::parse("valid-name").expect("valid name"),
            status: SessionStatus::Creating,
            state: WorkspaceState::Created,
            workspace_path: AbsolutePath::parse("/tmp/test").expect("valid path"),
            branch: BranchState::Detached,
            created_at: now,
            updated_at: earlier,
            last_synced: None,
            metadata: ValidatedMetadata::empty(),
        };

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_changes_summary_total() {
        let summary = ChangesSummary {
            modified: 5,
            added: 3,
            deleted: 2,
            renamed: 1,
            untracked: 4,
        };

        assert_eq!(summary.total(), 11);
        assert!(summary.has_changes());
        assert!(summary.has_tracked_changes());
    }

    #[test]
    fn test_changes_summary_no_changes() {
        let summary = ChangesSummary::default();
        assert_eq!(summary.total(), 0);
        assert!(!summary.has_changes());
    }

    #[test]
    fn test_beads_summary_active() {
        let summary = BeadsSummary {
            open: 3,
            in_progress: 2,
            blocked: 1,
            closed: 5,
        };

        assert_eq!(summary.total(), 11);
        assert_eq!(summary.active(), 5);
        assert!(summary.has_blockers());
    }

    #[test]
    fn test_file_change_renamed_validation() {
        let change = FileChange {
            path: PathBuf::from("new/path.txt"),
            status: FileStatus::Renamed,
            old_path: None,
        };

        assert!(change.validate().is_err());
    }

    #[test]
    fn test_file_change_renamed_valid() {
        let change = FileChange {
            path: PathBuf::from("new/path.txt"),
            status: FileStatus::Renamed,
            old_path: Some(PathBuf::from("old/path.txt")),
        };

        assert!(change.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_validation() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 5,
            files_changed: 2,
            files: vec![
                FileDiffStat {
                    path: PathBuf::from("file1.txt"),
                    insertions: 5,
                    deletions: 2,
                    status: FileStatus::Modified,
                },
                FileDiffStat {
                    path: PathBuf::from("file2.txt"),
                    insertions: 5,
                    deletions: 3,
                    status: FileStatus::Added,
                },
            ],
        };

        assert!(diff.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_mismatch() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 5,
            files_changed: 5,
            files: vec![FileDiffStat {
                path: PathBuf::from("file1.txt"),
                insertions: 5,
                deletions: 2,
                status: FileStatus::Modified,
            }],
        };

        assert!(diff.validate().is_err());
    }

    #[test]
    fn test_session_status_terminal_states() {
        assert!(SessionStatus::Completed.is_terminal());
        assert!(SessionStatus::Failed.is_terminal());
        assert!(!SessionStatus::Creating.is_terminal());
        assert!(!SessionStatus::Active.is_terminal());
        assert!(!SessionStatus::Paused.is_terminal());
    }

    #[test]
    fn test_session_name_max_length() {
        let exactly_63: String = "a".repeat(63);
        assert!(
            SessionName::parse(&exactly_63).is_ok(),
            "63 chars should be valid"
        );

        let too_long: String = "a".repeat(64);
        assert!(
            SessionName::parse(&too_long).is_err(),
            "64 chars should be invalid"
        );
    }

    #[test]
    fn test_session_name_special_chars() {
        assert!(SessionName::parse("name-with-dash").is_ok());
        assert!(SessionName::parse("name_with_underscore").is_ok());
        assert!(SessionName::parse("NameWithCaps123").is_ok());
        assert!(SessionName::parse("name with space").is_err());
        assert!(SessionName::parse("name@special").is_err());
        assert!(SessionName::parse("name.dots").is_err());
    }

    #[test]
    fn test_session_name_must_start_with_letter() {
        assert!(SessionName::parse("a").is_ok());
        assert!(SessionName::parse("A").is_ok());
        assert!(SessionName::parse("1start-with-number").is_err());
        assert!(SessionName::parse("_start-with-underscore").is_err());
        assert!(SessionName::parse("-start-with-dash").is_err());
    }

    #[test]
    fn test_branch_state_serialization() {
        let detached = BranchState::detached();
        let on_branch = BranchState::on_branch("feature/test");

        let detached_json = serde_json::to_string(&detached).expect("valid json");
        let branch_json = serde_json::to_string(&on_branch).expect("valid json");

        assert!(detached_json.contains("detached"));
        assert!(branch_json.contains("feature/test"));
    }

    proptest! {
        #[test]
        fn prop_session_name_adversarial(s in ".*") {
            let res = SessionName::parse(s.clone());

            // Check properties that the implementation *should* satisfy
            if let Ok(name) = res {
                let name_str = name.as_str();
                prop_assert!(!name_str.is_empty(), "Empty string allowed");
                prop_assert!(name_str.len() <= SessionName::MAX_LENGTH, "Max length exceeded: {} > {}", name_str.len(), SessionName::MAX_LENGTH);

                let first_char = name_str.chars().next().expect("has first char");
                prop_assert!(first_char.is_ascii_alphabetic(), "First char not alphabetic: {}", first_char);

                let valid_chars = name_str.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
                prop_assert!(valid_chars, "Contains invalid chars: {}", name_str);
            } else {
                let trimmed = s.trim();
                // If it failed, it must violate one of the rules.
                let violates_rules = trimmed.is_empty()
                    || trimmed.len() > SessionName::MAX_LENGTH
                    || !trimmed.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
                    || !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

                prop_assert!(violates_rules, "Valid string was rejected: {:?}", s);
            }
        }
    }
}
