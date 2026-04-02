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
pub use crate::type_file_change::{
    ChangesSummary, DiffSummary, FileChange, FileDiffStat, FileStatus,
};
pub use crate::type_metadata::ValidatedMetadata;
pub use crate::type_session::Session;
pub use crate::type_session_id::SessionId;
pub use crate::type_session_name::SessionName;
pub use crate::type_session_path::AbsolutePath;
pub use crate::type_session_status::{Operation, SessionStatus};
pub use crate::workspace_state::WorkspaceState;

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use proptest::prelude::*;
    use proptest::prop_assert;
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

    // ── Type alias and re-export smoke tests ─────────────────────────────────

    #[test]
    fn test_re_exports_are_accessible() {
        // Verify all re-exported types are usable via the types module
        let _session_id: Option<SessionId> = None;
        let _session_name: Option<SessionName> = None;
        let _absolute_path: Option<AbsolutePath> = None;
        let _session: Option<Session> = None;
        let _session_status: Option<SessionStatus> = None;
        let _branch_state: Option<BranchState> = None;
        let _metadata: Option<ValidatedMetadata> = None;
        let _beads_issue: Option<BeadsIssue> = None;
        let _beads_summary: Option<BeadsSummary> = None;
        let _changes_summary: Option<ChangesSummary> = None;
        let _diff_summary: Option<DiffSummary> = None;
        let _file_change: Option<FileChange> = None;
        let _file_diff_stat: Option<FileDiffStat> = None;
        let _file_status: Option<FileStatus> = None;
        let _issue_status: Option<IssueStatus> = None;
        let _operation: Option<Operation> = None;
        let _workspace_state: Option<WorkspaceState> = None;
    }

    // ── BeadsSummary construction and edge cases ────────────────────────────

    #[test]
    fn test_beads_summary_default_is_zero() {
        let summary = BeadsSummary::default();
        assert_eq!(summary.total(), 0);
        assert_eq!(summary.active(), 0);
        assert!(!summary.has_blockers());
    }

    #[test]
    fn test_beads_summary_all_fields_independent() {
        let summary = BeadsSummary {
            open: 1,
            in_progress: 0,
            blocked: 0,
            closed: 0,
        };
        assert_eq!(summary.total(), 1);
        assert_eq!(summary.active(), 1);

        let summary2 = BeadsSummary {
            open: 0,
            in_progress: 1,
            blocked: 0,
            closed: 0,
        };
        assert_eq!(summary2.total(), 1);
        assert_eq!(summary2.active(), 1);

        let summary3 = BeadsSummary {
            open: 0,
            in_progress: 0,
            blocked: 1,
            closed: 0,
        };
        assert_eq!(summary3.total(), 1);
        assert_eq!(summary3.active(), 0);
        assert!(summary3.has_blockers());
    }

    #[test]
    fn test_beads_summary_closed_not_in_active() {
        let summary = BeadsSummary {
            open: 0,
            in_progress: 0,
            blocked: 0,
            closed: 10,
        };
        assert_eq!(summary.total(), 10);
        assert_eq!(summary.active(), 0);
        assert!(!summary.has_blockers());
    }

    // ── IssueStatus enum variants ────────────────────────────────────────────

    #[test]
    fn test_issue_status_all_variants() {
        let statuses = [
            IssueStatus::Open,
            IssueStatus::InProgress,
            IssueStatus::Blocked,
            IssueStatus::Closed,
        ];
        assert_eq!(statuses.len(), 4);

        // Each variant should be distinct
        let mut set = std::collections::HashSet::new();
        for status in &statuses {
            assert!(set.insert(*status), "Duplicate variant: {status:?}");
        }
    }

    #[test]
    fn test_issue_status_serde_roundtrip() {
        for status in [
            IssueStatus::Open,
            IssueStatus::InProgress,
            IssueStatus::Blocked,
            IssueStatus::Closed,
        ] {
            let json = serde_json::to_string(&status).expect("serialize ok");
            let deserialized: IssueStatus = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_issue_status_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&IssueStatus::Open).expect("ok"),
            "\"open\""
        );
        assert_eq!(
            serde_json::to_string(&IssueStatus::InProgress).expect("ok"),
            "\"inprogress\""
        );
        assert_eq!(
            serde_json::to_string(&IssueStatus::Blocked).expect("ok"),
            "\"blocked\""
        );
        assert_eq!(
            serde_json::to_string(&IssueStatus::Closed).expect("ok"),
            "\"closed\""
        );
    }

    // ── Operation enum variants ──────────────────────────────────────────────

    #[test]
    fn test_operation_all_variants() {
        let ops = [
            Operation::Status,
            Operation::Diff,
            Operation::Focus,
            Operation::Remove,
        ];
        assert_eq!(ops.len(), 4);

        let mut set = std::collections::HashSet::new();
        for op in &ops {
            assert!(set.insert(*op), "Duplicate variant: {op:?}");
        }
    }

    #[test]
    fn test_operation_allowed_per_status_comprehensive() {
        // Creating: no operations allowed
        assert_eq!(SessionStatus::Creating.allowed_operations().len(), 0);
        assert!(!SessionStatus::Creating.allows_operation(Operation::Status));
        assert!(!SessionStatus::Creating.allows_operation(Operation::Diff));
        assert!(!SessionStatus::Creating.allows_operation(Operation::Focus));
        assert!(!SessionStatus::Creating.allows_operation(Operation::Remove));

        // Active: all operations
        let active_ops = SessionStatus::Active.allowed_operations();
        assert_eq!(active_ops.len(), 4);
        for op in [
            Operation::Status,
            Operation::Diff,
            Operation::Focus,
            Operation::Remove,
        ] {
            assert!(SessionStatus::Active.allows_operation(op));
        }

        // Paused: no Diff
        let paused_ops = SessionStatus::Paused.allowed_operations();
        assert_eq!(paused_ops.len(), 3);
        assert!(SessionStatus::Paused.allows_operation(Operation::Status));
        assert!(SessionStatus::Paused.allows_operation(Operation::Focus));
        assert!(SessionStatus::Paused.allows_operation(Operation::Remove));
        assert!(!SessionStatus::Paused.allows_operation(Operation::Diff));

        // Completed/Failed: only Remove
        for terminal in [SessionStatus::Completed, SessionStatus::Failed] {
            let ops = terminal.allowed_operations();
            assert_eq!(ops.len(), 1);
            assert!(terminal.allows_operation(Operation::Remove));
            assert!(!terminal.allows_operation(Operation::Status));
            assert!(!terminal.allows_operation(Operation::Diff));
            assert!(!terminal.allows_operation(Operation::Focus));
        }
    }

    // ── SessionStatus exhaustive transitions ────────────────────────────────

    #[test]
    fn test_session_status_valid_next_states() {
        let creating_next = SessionStatus::Creating.valid_next_states();
        assert_eq!(creating_next.len(), 2);
        assert!(creating_next.contains(&SessionStatus::Active));
        assert!(creating_next.contains(&SessionStatus::Failed));

        let active_next = SessionStatus::Active.valid_next_states();
        assert_eq!(active_next.len(), 2);
        assert!(active_next.contains(&SessionStatus::Paused));
        assert!(active_next.contains(&SessionStatus::Completed));

        let paused_next = SessionStatus::Paused.valid_next_states();
        assert_eq!(paused_next.len(), 2);
        assert!(paused_next.contains(&SessionStatus::Active));
        assert!(paused_next.contains(&SessionStatus::Completed));

        assert!(SessionStatus::Completed.valid_next_states().is_empty());
        assert!(SessionStatus::Failed.valid_next_states().is_empty());
    }

    #[test]
    fn test_session_status_no_self_transitions() {
        for status in SessionStatus::all_states() {
            assert!(
                !status.can_transition_to(*status),
                "Self-transition should not be allowed for {status:?}"
            );
        }
    }

    #[test]
    fn test_session_status_all_states_returns_all() {
        let all = SessionStatus::all_states();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&SessionStatus::Creating));
        assert!(all.contains(&SessionStatus::Active));
        assert!(all.contains(&SessionStatus::Paused));
        assert!(all.contains(&SessionStatus::Completed));
        assert!(all.contains(&SessionStatus::Failed));
    }

    #[test]
    fn test_session_status_serde_roundtrip() {
        for &status in SessionStatus::all_states() {
            let json = serde_json::to_string(&status).expect("serialize ok");
            let deserialized: SessionStatus = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(status, deserialized);
        }
    }

    // ── FileStatus enum variants ─────────────────────────────────────────────

    #[test]
    fn test_file_status_all_variants() {
        let statuses = [
            FileStatus::Modified,
            FileStatus::Added,
            FileStatus::Deleted,
            FileStatus::Renamed,
            FileStatus::Untracked,
        ];
        assert_eq!(statuses.len(), 5);

        let mut set = std::collections::HashSet::new();
        for s in &statuses {
            assert!(set.insert(*s), "Duplicate variant: {s:?}");
        }
    }

    #[test]
    fn test_file_status_serde_rename() {
        assert_eq!(
            serde_json::to_string(&FileStatus::Modified).expect("ok"),
            "\"M\""
        );
        assert_eq!(
            serde_json::to_string(&FileStatus::Added).expect("ok"),
            "\"A\""
        );
        assert_eq!(
            serde_json::to_string(&FileStatus::Deleted).expect("ok"),
            "\"D\""
        );
        assert_eq!(
            serde_json::to_string(&FileStatus::Renamed).expect("ok"),
            "\"R\""
        );
        assert_eq!(
            serde_json::to_string(&FileStatus::Untracked).expect("ok"),
            "\"?\""
        );
    }

    #[test]
    fn test_file_status_serde_roundtrip() {
        for status in [
            FileStatus::Modified,
            FileStatus::Added,
            FileStatus::Deleted,
            FileStatus::Renamed,
            FileStatus::Untracked,
        ] {
            let json = serde_json::to_string(&status).expect("serialize ok");
            let deserialized: FileStatus = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(status, deserialized);
        }
    }

    // ── FileChange construction ──────────────────────────────────────────────

    #[test]
    fn test_file_change_all_status_variants_valid() {
        for status in [
            FileStatus::Modified,
            FileStatus::Added,
            FileStatus::Deleted,
            FileStatus::Untracked,
        ] {
            let change = FileChange {
                path: PathBuf::from("some/file.txt"),
                status,
                old_path: None,
            };
            assert!(
                change.validate().is_ok(),
                "Status {status:?} should validate without old_path"
            );
        }
    }

    #[test]
    fn test_file_change_serde_roundtrip() {
        let change = FileChange {
            path: PathBuf::from("path/to/file.rs"),
            status: FileStatus::Modified,
            old_path: None,
        };
        let json = serde_json::to_string(&change).expect("serialize ok");
        let deserialized: FileChange = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(change.path, deserialized.path);
        assert_eq!(change.status, deserialized.status);
        assert_eq!(change.old_path, deserialized.old_path);
    }

    #[test]
    fn test_file_change_serde_with_old_path() {
        let change = FileChange {
            path: PathBuf::from("new/name.rs"),
            status: FileStatus::Renamed,
            old_path: Some(PathBuf::from("old/name.rs")),
        };
        let json = serde_json::to_string(&change).expect("serialize ok");
        assert!(json.contains("old/name.rs"));
        let deserialized: FileChange = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(
            deserialized.old_path.as_deref(),
            Some(std::path::Path::new("old/name.rs"))
        );
    }

    #[test]
    fn test_file_change_serde_skips_none_old_path() {
        let change = FileChange {
            path: PathBuf::from("file.rs"),
            status: FileStatus::Modified,
            old_path: None,
        };
        let json = serde_json::to_value(&change).expect("serialize ok");
        assert!(!json.get("old_path").is_some_and(|v| !v.is_null()));
    }

    // ── ChangesSummary construction ──────────────────────────────────────────

    #[test]
    fn test_changes_summary_default() {
        let summary = ChangesSummary::default();
        assert_eq!(summary.modified, 0);
        assert_eq!(summary.added, 0);
        assert_eq!(summary.deleted, 0);
        assert_eq!(summary.renamed, 0);
        assert_eq!(summary.untracked, 0);
        assert_eq!(summary.total(), 0);
        assert!(!summary.has_changes());
        assert!(!summary.has_tracked_changes());
    }

    #[test]
    fn test_changes_summary_has_tracked_vs_untracked() {
        // Only untracked, no tracked changes -- has_changes() is false because
        // total() excludes untracked, and has_changes() checks total() > 0.
        let summary = ChangesSummary {
            modified: 0,
            added: 0,
            deleted: 0,
            renamed: 0,
            untracked: 5,
        };
        assert!(!summary.has_changes());
        assert!(!summary.has_tracked_changes());

        // Only tracked changes
        let summary2 = ChangesSummary {
            modified: 1,
            added: 0,
            deleted: 0,
            renamed: 0,
            untracked: 0,
        };
        assert!(summary2.has_changes());
        assert!(summary2.has_tracked_changes());

        // Both
        let summary3 = ChangesSummary {
            modified: 1,
            added: 2,
            deleted: 3,
            renamed: 4,
            untracked: 5,
        };
        assert_eq!(summary3.total(), 10);
        assert!(summary3.has_changes());
        assert!(summary3.has_tracked_changes());
    }

    // ── DiffSummary construction ─────────────────────────────────────────────

    #[test]
    fn test_diff_summary_empty() {
        let diff = DiffSummary {
            insertions: 0,
            deletions: 0,
            files_changed: 0,
            files: vec![],
        };
        assert!(diff.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_file_stats_correctness() {
        let diff = DiffSummary {
            insertions: 100,
            deletions: 50,
            files_changed: 3,
            files: vec![
                FileDiffStat {
                    path: PathBuf::from("a.rs"),
                    insertions: 50,
                    deletions: 0,
                    status: FileStatus::Added,
                },
                FileDiffStat {
                    path: PathBuf::from("b.rs"),
                    insertions: 30,
                    deletions: 20,
                    status: FileStatus::Modified,
                },
                FileDiffStat {
                    path: PathBuf::from("c.rs"),
                    insertions: 20,
                    deletions: 30,
                    status: FileStatus::Deleted,
                },
            ],
        };
        assert!(diff.validate().is_ok());
        assert_eq!(diff.insertions, 100);
        assert_eq!(diff.deletions, 50);
        assert_eq!(diff.files_changed, 3);
    }

    #[test]
    fn test_diff_summary_serde_roundtrip() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 5,
            files_changed: 1,
            files: vec![FileDiffStat {
                path: PathBuf::from("test.rs"),
                insertions: 10,
                deletions: 5,
                status: FileStatus::Modified,
            }],
        };
        let json = serde_json::to_string(&diff).expect("serialize ok");
        let deserialized: DiffSummary = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(diff.insertions, deserialized.insertions);
        assert_eq!(diff.deletions, deserialized.deletions);
        assert_eq!(diff.files_changed, deserialized.files_changed);
        assert_eq!(diff.files.len(), deserialized.files.len());
    }

    // ── BeadsIssue construction ──────────────────────────────────────────────

    #[test]
    fn test_beads_issue_construction() {
        let issue = BeadsIssue {
            id: "bd-abc123".to_string(),
            title: "Fix bug".to_string(),
            status: IssueStatus::Open,
            priority: Some("high".to_string()),
            issue_type: Some("bug".to_string()),
        };
        assert_eq!(issue.id, "bd-abc123");
        assert_eq!(issue.title, "Fix bug");
        assert_eq!(issue.status, IssueStatus::Open);
        assert_eq!(issue.priority.as_deref(), Some("high"));
        assert_eq!(issue.issue_type.as_deref(), Some("bug"));
    }

    #[test]
    fn test_beads_issue_with_none_optional_fields() {
        let issue = BeadsIssue {
            id: "bd-xyz".to_string(),
            title: "Task".to_string(),
            status: IssueStatus::Closed,
            priority: None,
            issue_type: None,
        };
        assert!(issue.priority.is_none());
        assert!(issue.issue_type.is_none());
    }

    #[test]
    fn test_beads_issue_serde_skips_none_fields() {
        let issue = BeadsIssue {
            id: "bd-123".to_string(),
            title: "Test".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
        };
        let json = serde_json::to_value(&issue).expect("serialize ok");
        assert!(json.get("priority").is_none());
        assert!(json.get("issue_type").is_none());
    }

    #[test]
    fn test_beads_issue_serde_with_optional_fields() {
        let issue = BeadsIssue {
            id: "bd-123".to_string(),
            title: "Test".to_string(),
            status: IssueStatus::InProgress,
            priority: Some("low".to_string()),
            issue_type: Some("feature".to_string()),
        };
        let json = serde_json::to_value(&issue).expect("serialize ok");
        assert_eq!(json["priority"], "low");
        assert_eq!(json["type"], "feature");
    }

    // ── BranchState enum variants ────────────────────────────────────────────

    #[test]
    fn test_branch_state_detached() {
        let state = BranchState::Detached;
        assert!(state.is_detached());
        assert!(state.branch_name().is_none());
    }

    #[test]
    fn test_branch_state_on_branch() {
        let state = BranchState::on_branch("feature/test");
        assert!(!state.is_detached());
        assert_eq!(state.branch_name(), Some("feature/test"));
    }

    #[test]
    fn test_branch_state_constructors() {
        assert_eq!(BranchState::detached(), BranchState::Detached);
        assert_eq!(
            BranchState::on_branch("main"),
            BranchState::OnBranch("main".to_string())
        );
    }

    #[test]
    fn test_branch_state_equality_and_hash() {
        let a = BranchState::on_branch("main");
        let b = BranchState::on_branch("main");
        let c = BranchState::detached();
        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = std::collections::HashSet::new();
        assert!(set.insert(a.clone()));
        assert!(!set.insert(b)); // duplicate
        assert!(set.insert(c.clone()));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_branch_state_serde_roundtrip() {
        // Detached
        let detached = BranchState::Detached;
        let json = serde_json::to_string(&detached).expect("serialize ok");
        let deserialized: BranchState = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(detached, deserialized);

        // OnBranch
        let on_branch = BranchState::on_branch("develop");
        let json = serde_json::to_string(&on_branch).expect("serialize ok");
        let deserialized: BranchState = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(on_branch, deserialized);
    }

    #[test]
    fn test_branch_state_deserialize_detached_keyword() {
        let json = "\"detached\"";
        let deserialized: BranchState = serde_json::from_str(json).expect("deserialize ok");
        assert_eq!(deserialized, BranchState::Detached);
    }

    #[test]
    fn test_branch_state_deserialize_branch_name() {
        let json = "\"feature/something\"";
        let deserialized: BranchState = serde_json::from_str(json).expect("deserialize ok");
        assert_eq!(
            deserialized,
            BranchState::OnBranch("feature/something".to_string())
        );
    }
}
