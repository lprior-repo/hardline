//! Beads issue tracking types

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    Open,
    InProgress,
    Blocked,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsIssue {
    pub id: String,
    pub title: String,
    pub status: IssueStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeadsSummary {
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub closed: usize,
}

impl BeadsSummary {
    #[must_use]
    pub const fn total(&self) -> usize {
        self.open + self.in_progress + self.blocked + self.closed
    }

    #[must_use]
    pub const fn active(&self) -> usize {
        self.open + self.in_progress
    }

    #[must_use]
    pub const fn has_blockers(&self) -> bool {
        self.blocked > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::proptest;

    // ── IssueStatus variants ─────────────────────────────────────────────────

    #[test]
    fn test_issue_status_all_variants() {
        let variants = [
            IssueStatus::Open,
            IssueStatus::InProgress,
            IssueStatus::Blocked,
            IssueStatus::Closed,
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_issue_status_copy() {
        let s = IssueStatus::Open;
        let copied = s;
        assert_eq!(s, copied);
    }

    #[test]
    fn test_issue_status_debug() {
        let debug = format!("{:?}", IssueStatus::InProgress);
        assert!(debug.contains("InProgress"));
    }

    // ── IssueStatus serde roundtrip ──────────────────────────────────────────

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
            assert_eq!(status, deserialized, "Roundtrip failed for {status:?}");
        }
    }

    #[test]
    fn test_issue_status_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&IssueStatus::Open).expect("ok"),
            "\"open\""
        );
        // rename_all = "lowercase" removes underscores: InProgress -> "inprogress"
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

    #[test]
    fn test_issue_status_serde_unknown_string_fails() {
        let result: std::result::Result<IssueStatus, _> = serde_json::from_str("\"unknown\"");
        assert!(result.is_err());
    }

    // ── BeadsIssue construction ──────────────────────────────────────────────

    #[test]
    fn test_beads_issue_minimal() {
        let issue = BeadsIssue {
            id: "hl-001".to_string(),
            title: "Fix the thing".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
        };
        assert_eq!(issue.id, "hl-001");
        assert_eq!(issue.title, "Fix the thing");
        assert!(issue.priority.is_none());
        assert!(issue.issue_type.is_none());
    }

    #[test]
    fn test_beads_issue_with_optionals() {
        let issue = BeadsIssue {
            id: "hl-002".to_string(),
            title: "Add feature".to_string(),
            status: IssueStatus::InProgress,
            priority: Some("high".to_string()),
            issue_type: Some("feature".to_string()),
        };
        assert_eq!(issue.priority.as_deref(), Some("high"));
        assert_eq!(issue.issue_type.as_deref(), Some("feature"));
    }

    #[test]
    fn test_beads_issue_clone() {
        let issue = BeadsIssue {
            id: "hl-003".to_string(),
            title: "Bug".to_string(),
            status: IssueStatus::Blocked,
            priority: None,
            issue_type: None,
        };
        let cloned = issue.clone();
        assert_eq!(issue.id, cloned.id);
        assert_eq!(issue.title, cloned.title);
    }

    #[test]
    fn test_beads_issue_debug() {
        let issue = BeadsIssue {
            id: "hl-004".to_string(),
            title: "Debug test".to_string(),
            status: IssueStatus::Closed,
            priority: None,
            issue_type: None,
        };
        let debug = format!("{issue:?}");
        assert!(debug.contains("hl-004"));
        assert!(debug.contains("Debug test"));
    }

    // ── BeadsIssue serde roundtrip ───────────────────────────────────────────

    #[test]
    fn test_beads_issue_serde_roundtrip_minimal() {
        let issue = BeadsIssue {
            id: "hl-005".to_string(),
            title: "Serde test".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
        };
        let json = serde_json::to_string(&issue).expect("serialize ok");
        let deserialized: BeadsIssue = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(issue.id, deserialized.id);
        assert_eq!(issue.title, deserialized.title);
        assert_eq!(issue.status, deserialized.status);
        assert!(deserialized.priority.is_none());
        assert!(deserialized.issue_type.is_none());
    }

    #[test]
    fn test_beads_issue_serde_roundtrip_full() {
        let issue = BeadsIssue {
            id: "hl-006".to_string(),
            title: "Full serde test".to_string(),
            status: IssueStatus::Blocked,
            priority: Some("critical".to_string()),
            issue_type: Some("bugfix".to_string()),
        };
        let json = serde_json::to_string(&issue).expect("serialize ok");
        let deserialized: BeadsIssue = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.priority.as_deref(), Some("critical"));
        assert_eq!(deserialized.issue_type.as_deref(), Some("bugfix"));
    }

    #[test]
    fn test_beads_issue_serde_type_renamed() {
        let issue = BeadsIssue {
            id: "hl-007".to_string(),
            title: "Rename test".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: Some("enhancement".to_string()),
        };
        let json_val = serde_json::to_value(&issue).expect("serialize ok");
        let obj = json_val.as_object().expect("should be object");
        assert!(
            obj.contains_key("type"),
            "issue_type should be renamed to 'type'"
        );
        assert!(!obj.contains_key("issue_type"));
    }

    #[test]
    fn test_beads_issue_serde_skips_none_fields() {
        let issue = BeadsIssue {
            id: "hl-008".to_string(),
            title: "Skip test".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
        };
        let json_val = serde_json::to_value(&issue).expect("serialize ok");
        let obj = json_val.as_object().expect("should be object");
        assert!(!obj.contains_key("priority"));
        assert!(!obj.contains_key("issue_type"));
    }

    // ── BeadsSummary ─────────────────────────────────────────────────────────

    #[test]
    fn test_beads_summary_default() {
        let s = BeadsSummary::default();
        assert_eq!(s.open, 0);
        assert_eq!(s.in_progress, 0);
        assert_eq!(s.blocked, 0);
        assert_eq!(s.closed, 0);
        assert_eq!(s.total(), 0);
        assert_eq!(s.active(), 0);
        assert!(!s.has_blockers());
    }

    #[test]
    fn test_beads_summary_total() {
        let s = BeadsSummary {
            open: 3,
            in_progress: 2,
            blocked: 1,
            closed: 5,
        };
        assert_eq!(s.total(), 11);
    }

    #[test]
    fn test_beads_summary_active() {
        let s = BeadsSummary {
            open: 3,
            in_progress: 2,
            blocked: 1,
            closed: 5,
        };
        assert_eq!(s.active(), 5); // open + in_progress
    }

    #[test]
    fn test_beads_summary_has_blockers_true() {
        let s = BeadsSummary {
            open: 0,
            in_progress: 0,
            blocked: 1,
            closed: 0,
        };
        assert!(s.has_blockers());
    }

    #[test]
    fn test_beads_summary_has_blockers_false() {
        let s = BeadsSummary {
            open: 10,
            in_progress: 5,
            blocked: 0,
            closed: 3,
        };
        assert!(!s.has_blockers());
    }

    #[test]
    fn test_beads_summary_serde_roundtrip() {
        let s = BeadsSummary {
            open: 2,
            in_progress: 1,
            blocked: 3,
            closed: 4,
        };
        let json = serde_json::to_string(&s).expect("serialize ok");
        let deserialized: BeadsSummary = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(s.open, deserialized.open);
        assert_eq!(s.in_progress, deserialized.in_progress);
        assert_eq!(s.blocked, deserialized.blocked);
        assert_eq!(s.closed, deserialized.closed);
    }

    #[test]
    fn test_beads_summary_debug() {
        let s = BeadsSummary {
            open: 1,
            in_progress: 0,
            blocked: 0,
            closed: 0,
        };
        let debug = format!("{s:?}");
        assert!(debug.contains("open"));
        assert!(debug.contains("1"));
    }

    // ── BeadsSummary proptests ───────────────────────────────────────────────

    proptest! {
        #[test]
        fn prop_summary_total_matches_sum(
            open in 0u32..100u32,
            in_progress in 0u32..100u32,
            blocked in 0u32..100u32,
            closed in 0u32..100u32
        ) {
            let s = BeadsSummary {
                open: open as usize,
                in_progress: in_progress as usize,
                blocked: blocked as usize,
                closed: closed as usize,
            };
            assert_eq!(s.total(), (open + in_progress + blocked + closed) as usize);
        }

        #[test]
        fn prop_summary_active_equals_open_plus_in_progress(
            open in 0u32..100u32,
            in_progress in 0u32..100u32
        ) {
            let s = BeadsSummary {
                open: open as usize,
                in_progress: in_progress as usize,
                blocked: 0,
                closed: 0,
            };
            assert_eq!(s.active(), (open + in_progress) as usize);
        }
    }
}
